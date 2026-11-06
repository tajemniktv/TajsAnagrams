use crate::jobs::Desktop;
use anagram_core::wordle::*;
use std::{
    fs, thread,
    time::{Duration, Instant},
};
fn command(mode: Mode, action: SessionAction) -> SessionCommand {
    SessionCommand {
        mode,
        action,
        revision: 1,
        profile: ProfileKind::Broad,
        attempts: 6,
        answers_path: String::new(),
        guesses_path: String::new(),
        word: String::new(),
        request: Default::default(),
    }
}
fn wait(app: &Desktop) -> SessionSnapshot {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        let s = app.inner.lock().unwrap().wordle.snapshot.clone();
        if !s.active {
            return s;
        }
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(2));
    }
}
fn setup() -> (tempfile::TempDir, Desktop) {
    let temp = tempfile::tempdir().unwrap();
    let app = Desktop::new(temp.path().join("user-data")).unwrap();
    let dictionary = temp.path().join("dictionary.txt");
    fs::write(&dictionary, "apple\nalley\nother\ncrane\nstone\n").unwrap();
    {
        let mut inner = app.inner.lock().unwrap();
        inner.settings.corpora.dictionary = dictionary.to_string_lossy().into_owned();
        inner.settings.corpora.unigrams = String::new();
    }
    (temp, app)
}
#[test]
fn wordle_needs_no_anagram_corpora_and_keeps_its_results_separate() {
    let (_temp, app) = setup();
    {
        let mut inner = app.inner.lock().unwrap();
        inner.job.id = 42;
        inner.job.result = Some(serde_json::json!({"previous":"anagram"}));
    }
    app.wordle_start(command(Mode::Play, SessionAction::Analyze))
        .unwrap();
    let s = wait(&app);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.analysis.unwrap().candidate_count, 5);
    let page = app.wordle_page(Mode::Play, 2).unwrap();
    assert_eq!(page.analysis.unwrap().candidates.len(), 3);
    let inner = app.inner.lock().unwrap();
    assert_eq!(inner.job.id, 42);
    assert_eq!(inner.job.result.as_ref().unwrap()["previous"], "anagram");
    drop(inner);
    app.shutdown();
}
#[test]
fn desktop_publishes_the_full_candidate_pool_for_scroll_and_search() {
    let (temp, app) = setup();
    let words: Vec<String> = (0..350)
        .map(|mut n| {
            (0..5)
                .map(|_| {
                    let ch = char::from(b'a' + (n % 26) as u8);
                    n /= 26;
                    ch
                })
                .collect()
        })
        .collect();
    fs::write(temp.path().join("dictionary.txt"), words.join("\n")).unwrap();
    let mut c = command(Mode::Instant, SessionAction::Filter);
    c.request.limit = 1;
    c.request.offset = 100;
    app.wordle_start(c).unwrap();
    let s = wait(&app);
    assert!(s.error.is_none(), "{:?}", s.error);
    let a = s.analysis.unwrap();
    assert_eq!(a.candidate_count, 350);
    assert_eq!(a.candidates.len(), 350);
    assert_eq!(a.offset, 0);
    assert!(words.iter().all(|word| a.candidates.contains(word)));
    app.shutdown();
}
#[test]
fn play_accepts_partial_feedback_and_practice_ignores_external_partial_clues() {
    let (_temp, app) = setup();
    let mut c = command(Mode::Play, SessionAction::Filter);
    c.request.partial_history = vec![PartialHistoryRow {
        word: "alley".into(),
        feedback: vec![Some(Mark::Green), None, Some(Mark::Gray), None, None],
    }];
    app.wordle_start(c.clone()).unwrap();
    let s = wait(&app);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.analysis.unwrap().candidates, vec!["apple"]);
    c.mode = Mode::Practice;
    c.action = SessionAction::NewPractice;
    app.wordle_start(c).unwrap();
    let s = wait(&app);
    assert!(s.error.is_none(), "{:?}", s.error);
    assert_eq!(s.analysis.unwrap().candidate_count, 5);
    assert!(s.history.is_empty());
    assert!(s.revealed_answer.is_none());
    app.shutdown();
}
#[test]
fn practice_secret_stays_native_and_invalid_guesses_do_not_consume_attempts() {
    let (_temp, app) = setup();
    app.wordle_start(command(Mode::Practice, SessionAction::NewPractice))
        .unwrap();
    let initial = wait(&app);
    assert!(initial.error.is_none());
    assert!(initial.revealed_answer.is_none());
    assert!(initial.history.is_empty());
    assert!(initial.analysis.unwrap().strategic.is_none());
    let mut c = command(Mode::Practice, SessionAction::Guess);
    c.word = "zzzzz".into();
    app.wordle_start(c).unwrap();
    assert!(wait(&app).error.is_some());
    let mut c = command(Mode::Practice, SessionAction::Guess);
    c.word = "apple".into();
    c.revision = 3;
    app.wordle_start(c).unwrap();
    let s = wait(&app);
    assert!(s.error.is_none());
    assert_eq!(s.history.len(), 1);
    app.wordle_start(command(Mode::Practice, SessionAction::Reveal))
        .unwrap();
    let s = wait(&app);
    assert!(s.finished);
    let answer = s.revealed_answer.unwrap();
    assert_eq!(s.history[0].feedback, feedback(&answer, "apple").unwrap());
    app.shutdown();
}
#[test]
fn wordle_uses_shared_worker_and_cancels_before_publishing() {
    let (_temp, app) = setup();
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    app.inner.lock().unwrap().worker_gate = Some(gate.clone());
    app.wordle_start(command(Mode::Play, SessionAction::Analyze))
        .unwrap();
    assert!(app.operation(1, |_| Ok(serde_json::Value::Null)).is_err());
    assert!(
        app.wordle_start(command(Mode::Instant, SessionAction::Analyze))
            .is_err()
    );
    app.inner.lock().unwrap().control.cancel();
    gate.wait();
    let s = wait(&app);
    assert!(s.error.unwrap().contains("cancelled"));
    assert!(s.analysis.is_none());
    app.shutdown();
}
#[test]
fn old_settings_get_default_preferences() {
    let s = crate::settings::Settings::default();
    let mut json = serde_json::to_value(s).unwrap();
    json.as_object_mut().unwrap().remove("wordle");
    let s: crate::settings::Settings = serde_json::from_value(json).unwrap();
    assert_eq!(s.wordle.length, 5);
    assert_eq!(s.wordle.attempts, 6);
}
#[test]
fn changed_source_content_invalidates_vocabulary_and_requires_history_restart() {
    let (temp, app) = setup();
    app.wordle_start(command(Mode::Play, SessionAction::Analyze))
        .unwrap();
    let first = wait(&app).analysis.unwrap().profile.fingerprint;
    fs::write(
        temp.path().join("dictionary.txt"),
        "apple\nalley\nother\ncrane\nstone\nsound\n",
    )
    .unwrap();
    let mut c = command(Mode::Play, SessionAction::Analyze);
    c.request.history.push(HistoryRow {
        word: "alley".into(),
        feedback: feedback("apple", "alley").unwrap(),
    });
    app.wordle_start(c).unwrap();
    assert!(wait(&app).error.unwrap().contains("Vocabulary changed"));
    app.wordle_start(command(Mode::Play, SessionAction::Analyze))
        .unwrap();
    let next = wait(&app).analysis.unwrap();
    assert_ne!(first, next.profile.fingerprint);
    assert_eq!(next.candidate_count, 6);
    app.shutdown();
}
