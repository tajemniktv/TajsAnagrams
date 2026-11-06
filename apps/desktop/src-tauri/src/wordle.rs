//! Wordle sessions share the desktop worker, but never the anagram result slot.
use crate::{jobs::Desktop, settings};
use anagram_core::{
    control::Control,
    lexicon::Unigrams,
    provenance::Snapshot,
    wordle::{
        self, Analysis, Mode, ProfileKind, SessionAction, SessionCommand, SessionSnapshot,
        Vocabulary,
    },
};
use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone)]
struct Session {
    vocabulary: Arc<Vocabulary>,
    secret: Option<String>,
    history: Vec<wordle::HistoryRow>,
    attempts: usize,
    length: usize,
    finished: bool,
    candidates: Vec<String>,
    snapshot: SessionSnapshot,
}
#[derive(Default)]
pub struct HostState {
    pub snapshot: SessionSnapshot,
    sessions: HashMap<Mode, Session>,
    cache: Option<(String, Arc<Vocabulary>)>,
}
fn fail(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn vocabulary(
    app: &Desktop,
    c: &SessionCommand,
    settings: &settings::Settings,
    control: &Control,
) -> Result<Arc<Vocabulary>, String> {
    let mut budget = settings.runtime.maintenance_limits().corpus_read_budget();
    let guess_path = if c.profile == ProfileKind::Custom {
        if c.guesses_path.is_empty() {
            &c.answers_path
        } else {
            &c.guesses_path
        }
    } else {
        &settings.corpora.dictionary
    };
    let guesses = Snapshot::load_with_budget(
        Path::new(guess_path),
        "wordle_guesses",
        control,
        &mut budget,
    )
    .map_err(fail)?;
    let answers = if c.profile == ProfileKind::Custom {
        Some(
            Snapshot::load_with_budget(
                Path::new(&c.answers_path),
                "wordle_answers",
                control,
                &mut budget,
            )
            .map_err(fail)?,
        )
    } else {
        None
    };
    let frequencies = if !settings.corpora.unigrams.is_empty()
        && Path::new(&settings.corpora.unigrams).is_file()
    {
        Some(
            Snapshot::load_with_budget(
                Path::new(&settings.corpora.unigrams),
                "wordle_frequencies",
                control,
                &mut budget,
            )
            .map_err(fail)?,
        )
    } else {
        None
    };
    let mut sources = vec![guesses.identity().clone()];
    if let Some(a) = &answers {
        sources.push(a.identity().clone());
    }
    if let Some(f) = &frequencies {
        sources.push(f.identity().clone());
    }
    let key = format!(
        "{:?}:{}",
        c.profile,
        serde_json::to_string(&sources).map_err(fail)?
    );
    if let Some((old, v)) = &app.inner.lock().map_err(fail)?.wordle.cache
        && old == &key
    {
        return Ok(v.clone());
    }
    let frequencies = frequencies
        .as_ref()
        .map(|s| Unigrams::load(control.reader(std::io::Cursor::new(s.bytes()))).map_err(fail))
        .transpose()?;
    let v = Arc::new(Vocabulary::load(
        c.profile,
        guesses.bytes(),
        answers.as_ref().map(Snapshot::bytes),
        frequencies.as_ref(),
        sources,
        control,
    )?);
    app.inner.lock().map_err(fail)?.wordle.cache = Some((key, v.clone()));
    Ok(v)
}
fn random_index(len: usize) -> Result<usize, String> {
    let ceiling = u64::MAX - u64::MAX % (len as u64);
    loop {
        let mut bytes = [0; 8];
        getrandom::fill(&mut bytes).map_err(fail)?;
        let n = u64::from_le_bytes(bytes);
        if n < ceiling {
            return Ok((n % len as u64) as usize);
        }
    }
}
fn execute(
    app: &Desktop,
    command: &mut SessionCommand,
    settings: &settings::Settings,
    previous: Option<Session>,
    id: u64,
    control: &Control,
) -> Result<(Session, SessionSnapshot), String> {
    let practice = command.mode == Mode::Practice;
    let fresh = matches!(command.action, SessionAction::NewPractice);
    if !practice
        && matches!(
            command.action,
            SessionAction::NewPractice | SessionAction::Guess | SessionAction::Reveal
        )
    {
        return Err("This action requires Practice mode".into());
    }
    let v = if practice && !fresh {
        previous
            .as_ref()
            .ok_or("Start a new practice game first")?
            .vocabulary
            .clone()
    } else {
        vocabulary(app, command, settings, control)?
    };
    if !practice
        && (!command.request.history.is_empty() || !command.request.partial_history.is_empty())
        && previous
            .as_ref()
            .is_some_and(|s| s.vocabulary.profile.fingerprint != v.profile.fingerprint)
    {
        return Err("Vocabulary changed. Start a new puzzle to use the updated profile.".into());
    }
    let mut session = if practice && !fresh {
        previous.ok_or("Start a new practice game first")?
    } else {
        Session {
            vocabulary: v.clone(),
            secret: None,
            history: vec![],
            attempts: command.attempts,
            length: command.request.length,
            finished: false,
            candidates: vec![],
            snapshot: Default::default(),
        }
    };
    if practice {
        if session.length != command.request.length
            || session.attempts != command.attempts
            || session.vocabulary.profile.kind != command.profile
        {
            return Err("Start a new practice game after changing its configuration".into());
        }
        if fresh {
            let answers: Vec<_> = v
                .answers
                .iter()
                .filter(|&&i| v.words[i].word.len() == session.length)
                .collect();
            if answers.is_empty() {
                return Err(
                    "No practice answers at this length. Select another profile or length.".into(),
                );
            }
            session.secret = Some(v.words[*answers[random_index(answers.len())?]].word.clone());
        }
        match command.action {
            SessionAction::Guess => {
                if session.finished {
                    return Err("This game is finished. Start a new game.".into());
                }
                let word = wordle::validate_word(&command.word)?;
                if word.len() != session.length || !v.contains(&word) {
                    return Err("Guess is not in this profile’s allowed vocabulary".into());
                }
                let secret = session.secret.as_ref().ok_or("Start a new practice game")?;
                session.history.push(wordle::HistoryRow {
                    feedback: wordle::feedback(secret, &word)?,
                    word: word.clone(),
                });
                session.finished = word == *secret || session.history.len() >= session.attempts;
            }
            SessionAction::Reveal => session.finished = true,
            _ => {}
        }
        command.request.history = session.history.clone();
        command.request.partial_history.clear();
        command.request.constraints = Default::default();
        command.request.attempts_remaining = if session.finished {
            0
        } else {
            session.attempts - session.history.len()
        };
    } else {
        session.history = command.request.history.clone();
    }
    let (mut analysis, ids) = wordle::filter(&v, &command.request, control)?;
    session.candidates = ids.iter().map(|&i| v.words[i].word.clone()).collect();
    // Desktop lists scroll and search the full filtered pool. The core's paged
    // contract remains useful to other consumers, but must not truncate this UI.
    analysis.candidates = session.candidates.clone();
    analysis.offset = 0;
    let make = |analysis: Analysis, active| SessionSnapshot {
        id,
        revision: command.revision,
        mode: command.mode,
        active,
        analysis: Some(analysis),
        history: session.history.clone(),
        finished: session.finished,
        revealed_answer: if session.finished {
            session.secret.clone()
        } else {
            None
        },
        error: None,
    };
    app.inner.lock().map_err(fail)?.wordle.snapshot = make(analysis.clone(), true);
    if matches!(command.action, SessionAction::Analyze) {
        wordle::recommend(&v, &command.request, &ids, control, &mut analysis)?;
    }
    control.check().map_err(str::to_owned)?;
    let snapshot = make(analysis, false);
    session.snapshot = snapshot.clone();
    Ok((session, snapshot))
}

impl Desktop {
    pub fn wordle_start(&self, mut command: SessionCommand) -> Result<u64, String> {
        if !(1..=12).contains(&command.attempts) || !(4..=8).contains(&command.request.length) {
            return Err("Choose 4–8 letters and 1–12 attempts".into());
        }
        // Desktop callers cannot turn an interactive job into an unlimited offline search.
        command.request.budget = wordle::SearchBudget::default();
        command.request.strategy = wordle::Strategy::Adaptive;
        let mut inner = self.inner.lock().map_err(fail)?;
        if inner.closing
            || inner.job.active
            || inner.experiment.active
            || inner.wordle.snapshot.active
        {
            return Err("Wait for the current operation, or cancel it first".into());
        }
        if let Some(worker) = inner.worker.take() {
            let _ = worker.join();
        }
        let id = inner.wordle.snapshot.id + 1;
        inner.control = Control::with_deadline(Instant::now() + Duration::from_secs(30));
        let control = inner.control.clone();
        let previous = inner.wordle.sessions.get(&command.mode).cloned();
        let settings = inner.settings.clone();
        inner.wordle.snapshot = SessionSnapshot {
            id,
            revision: command.revision,
            mode: command.mode,
            active: true,
            ..Default::default()
        };
        let app = Desktop {
            inner: self.inner.clone(),
            data: self.data.clone(),
        };
        #[cfg(test)]
        let gate = inner.worker_gate.take();
        let spawned = std::thread::Builder::new()
            .name("Wordle-assistant".into())
            .spawn(move || {
                #[cfg(test)]
                if let Some(gate) = gate {
                    gate.wait();
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    execute(&app, &mut command, &settings, previous, id, &control)
                }))
                .unwrap_or_else(|_| Err("Wordle operation stopped unexpectedly".into()));

                if let Ok(mut inner) = app.inner.lock() {
                    match result {
                        Ok((session, snapshot)) => {
                            inner.wordle.sessions.insert(command.mode, session);
                            inner.wordle.snapshot = snapshot;
                        }
                        Err(error) => {
                            let mut snapshot = inner
                                .wordle
                                .sessions
                                .get(&command.mode)
                                .map(|session| session.snapshot.clone())
                                .unwrap_or_default();
                            snapshot.id = id;
                            snapshot.revision = command.revision;
                            snapshot.mode = command.mode;
                            snapshot.active = false;
                            snapshot.error = Some(error);
                            inner.wordle.snapshot = snapshot;
                        }
                    }
                }
            });
        match spawned {
            Ok(worker) => inner.worker = Some(worker),
            Err(e) => {
                inner.wordle.snapshot.active = false;
                inner.wordle.snapshot.error = Some(e.to_string());
                return Err(e.to_string());
            }
        }
        Ok(id)
    }
    pub fn wordle_page(&self, mode: Mode, offset: usize) -> Result<SessionSnapshot, String> {
        let inner = self.inner.lock().map_err(fail)?;
        let s = inner
            .wordle
            .sessions
            .get(&mode)
            .ok_or("No Wordle analysis yet")?;
        let mut snapshot = s.snapshot.clone();
        if let Some(a) = &mut snapshot.analysis {
            a.offset = offset;
            a.candidates = s
                .candidates
                .iter()
                .skip(offset)
                .take(100)
                .cloned()
                .collect();
        }
        Ok(snapshot)
    }
}
