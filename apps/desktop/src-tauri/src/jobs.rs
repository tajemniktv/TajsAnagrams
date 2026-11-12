use crate::settings::{self, Settings};
use anagram_core::{cache, contracts::JobStatus, control::Control, solve};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize, Default)]
pub struct Snapshot {
    pub id: u64,
    pub active: bool,
    pub status: Option<JobStatus>,
    pub result: Option<Value>,
    pub error: Option<String>,
}
#[derive(Clone, Serialize)]
pub struct ActiveOperation {
    pub id: u64,
    pub kind: &'static str,
    pub cancelling: bool,
}
pub struct Inner {
    pub operation: Option<ActiveOperation>,
    operation_sequence: u64,
    pub wordle: crate::wordle::HostState,
    pub experiment: crate::experimental::Snapshot,
    pub settings: Settings,
    pub notice: Option<String>,
    pub job: Snapshot,
    pub review_context: Value,
    pub control: Control,
    pub worker: Option<JoinHandle<()>>,
    pub closing: bool,
    #[cfg(test)]
    pub worker_gate: Option<Arc<std::sync::Barrier>>,
    #[cfg(test)]
    pub panic_after_progress: bool,
}
impl Inner {
    pub fn ensure_idle(&self) -> Result<(), String> {
        if self.closing || self.operation.is_some() {
            return Err("An operation is already running or the app is closing. Wait for it, or cancel it first".into());
        }
        Ok(())
    }
    pub fn admit(&mut self, kind: &'static str) -> Result<u64, String> {
        self.ensure_idle()?;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.operation_sequence += 1;
        let id = self.operation_sequence;
        self.operation = Some(ActiveOperation {
            id,
            kind,
            cancelling: false,
        });
        Ok(id)
    }
    pub fn cancel_operation(&mut self, id: u64) {
        if let Some(operation) = &mut self.operation
            && operation.id == id
        {
            operation.cancelling = true;
            self.control.cancel();
        }
    }
}
pub struct Desktop {
    pub inner: Arc<Mutex<Inner>>,
    pub data: PathBuf,
}

fn generate_only(
    request: &solve::Request,
    corpora: &settings::Corpora,
    control: &Control,
    limits: &anagram_core::policy::DeploymentLimits,
    id: &str,
    observer: &mut dyn FnMut(&JobStatus),
) -> Result<Value, anagram_core::request::Error> {
    use anagram_core::{
        contracts::{Exhaustion, Stage},
        lexicon::Unigrams,
        progress::Progress,
        provenance::Snapshot,
        request::Error,
    };
    let control = limits.control(control)?;
    let mut progress = Progress::new(request, limits, &control, id, observer);
    progress.start();
    let result = (|| {
        let fail = Error::corpus;
        let mut corpus_budget = limits.corpus_read_budget();
        let dictionary = Snapshot::load_with_budget(
            Path::new(&corpora.dictionary),
            "dictionary",
            &control,
            &mut corpus_budget,
        )
        .map_err(fail)?;
        progress
            .status
            .versions
            .data
            .push(dictionary.identity().clone());
        let unigrams = if request.generation.min_zipf <= 0.0 || corpora.unigrams.is_empty() {
            None
        } else {
            let source = Snapshot::load_with_budget(
                Path::new(&corpora.unigrams),
                "unigrams",
                &control,
                &mut corpus_budget,
            )
            .map_err(fail)?;
            progress
                .status
                .versions
                .data
                .push(source.identity().clone());
            Some(
                Unigrams::load(control.reader(std::io::Cursor::new(source.bytes())))
                    .map_err(fail)?,
            )
        };
        progress.status.versions.data_complete = true;
        progress.stage(Stage::Generating);
        let generated = anagram_core::request::generate_with_limits(
            &request.generation,
            control.reader(std::io::Cursor::new(dictionary.bytes())),
            unigrams.as_ref(),
            &control,
            limits,
        )?;
        progress.status.counts.generated = generated.generated;
        progress.status.exhaustion = match generated.stop {
            anagram_core::generation::Stop::Exhausted => Exhaustion::Exhausted,
            anagram_core::generation::Stop::CandidateCap => Exhaustion::Truncated,
            _ => Exhaustion::Unknown,
        };
        control.check().map_err(|e| Error::new(e, e))?;
        progress.status.counts.shown = generated.bags.len();
        serde_json::to_value(generated)
            .map_err(|e| Error::new("serialization_error", e.to_string()))
    })();
    let result = match control.check() {
        Ok(()) => result,
        Err(reason) => Err(Error::new(reason, reason)),
    };
    if result.is_err() {
        progress.status.counts.shown = 0;
    }
    progress.finish(result.as_ref().err());
    result.and_then(|mut value| {
        value["status"] = serde_json::to_value(progress.status)
            .map_err(|e| Error::new("serialization_error", e.to_string()))?;
        Ok(value)
    })
}
impl Desktop {
    pub fn new(data: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
        let (settings, notice) = settings::load(&data);
        Ok(Self {
            data,
            inner: Arc::new(Mutex::new(Inner {
                operation: None,
                operation_sequence: 0,
                wordle: Default::default(),
                settings,
                notice,
                job: Snapshot::default(),
                review_context: Value::Null,
                experiment: Default::default(),
                control: Control::default(),
                worker: None,
                closing: false,
                #[cfg(test)]
                worker_gate: None,
                #[cfg(test)]
                panic_after_progress: false,
            })),
        })
    }
    #[cfg(test)]
    pub fn start(&self, request: Value) -> Result<u64, String> {
        self.start_with_options(request, false, false)
    }
    #[cfg(test)]
    pub fn start_with_options(
        &self,
        request: Value,
        rebuild: bool,
        extended: bool,
    ) -> Result<u64, String> {
        self.start_with_mode(request, rebuild, extended, false)
    }
    pub fn start_with_mode(
        &self,
        request: Value,
        rebuild: bool,
        extended: bool,
        generation_only: bool,
    ) -> Result<u64, String> {
        let request: solve::Request = serde_json::from_value(request).map_err(|e| e.to_string())?;
        if generation_only {
            anagram_core::request::validate(&request.generation).map_err(|e| e.message)?;
            solve::validate_budgets(&request).map_err(|e| e.message)?;
        } else {
            solve::validate(&request).map_err(|e| e.message)?;
        }
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        if !generation_only
            && !extended
            && !inner.settings.runtime.custom_limits
            && request.result_limit_per_group > 100
        {
            return Err("Desktop results are limited to 100 rows per word-count group.".into());
        }
        let limits = if extended {
            anagram_core::policy::DeploymentLimits::default()
        } else if inner.settings.runtime.custom_limits {
            inner.settings.runtime.limits.clone()
        } else {
            settings::limits()
        };
        if generation_only {
            limits.admit_generation(&request.generation)
        } else {
            limits.admit(&request)
        }
        .map_err(|e| e.message)?;
        inner.ensure_idle()?;
        if !generation_only {
            inner.settings.corpora.validate()?;
        }
        let mut settings = inner.settings.clone();
        settings.request = serde_json::to_value(&request).map_err(|e| e.to_string())?;
        settings::save(&self.data, &settings)?;
        inner.settings = settings.clone();
        let id = inner.admit("anagrams")?;
        inner.review_context = json!({"origin":"installed desktop capture; no source family supplied", "request":settings.request,"extended":extended,"generation_only":generation_only,"limits":limits,"configured_corpora":settings.corpora,"configured_model":settings.model,"started_at_unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_millis()});
        inner.job = Snapshot {
            id,
            active: true,
            ..Default::default()
        };
        inner.control = Control::default();
        let control = inner.control.clone();
        let shared = self.inner.clone();
        let cache_path = settings.runtime.cache_path(&self.data);
        #[cfg(test)]
        let worker_gate = inner.worker_gate.take();
        #[cfg(test)]
        let panic_after_progress = std::mem::take(&mut inner.panic_after_progress);
        let spawn = std::thread::Builder::new()
            .name("TajsAnagrams-solver".into())
            .spawn(move || {
                #[cfg(test)]
                if let Some(gate) = worker_gate {
                    gate.wait();
                }
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let c = settings.corpora;
                    if generation_only {
                        return generate_only(
                            &request,
                            &c,
                            &control,
                            &limits,
                            &format!("desktop-{id}"),
                            &mut |status| {
                                if let Ok(mut inner) = shared.lock() {
                                    inner.job.status = Some(status.clone());
                                }
                                #[cfg(test)]
                                if panic_after_progress {
                                    panic!("injected worker failure");
                                }
                            },
                        );
                    }
                    let config = cache::Config {
                        path: &cache_path,
                        limits: settings.runtime.cache_limits(),
                        rebuild,
                    };
                    solve::solve_cached_observed(
                        &request,
                        solve::Paths {
                            dictionary: Path::new(&c.dictionary),
                            unigrams: Path::new(&c.unigrams),
                            bigrams: Path::new(&c.bigrams),
                            wordnet: Path::new(&c.wordnet),
                            phrase: (!c.phrase.is_empty()).then(|| Path::new(&c.phrase)),
                            model: (!settings.model.is_empty()).then(|| Path::new(&settings.model)),
                        },
                        &control,
                        &limits,
                        (&format!("desktop-{id}"), &mut |status| {
                            if let Ok(mut inner) = shared.lock() {
                                inner.job.status = Some(status.clone());
                            }
                        }),
                        Some(&config),
                    )
                    .and_then(|r| {
                        serde_json::to_value(r).map_err(|e| {
                            anagram_core::request::Error::new("serialization_error", e.to_string())
                        })
                    })
                }))
                .unwrap_or_else(|_| {
                    Err(anagram_core::request::Error::new(
                        "worker_panic",
                        "Solver stopped unexpectedly. You can start a new solve.",
                    ))
                });
                if let Ok(mut inner) = shared.lock() {
                    match outcome {
                        Ok(result) => inner.job.result = Some(result),
                        Err(error) => {
                            if inner.job.status.as_ref().is_none_or(|s| {
                                matches!(
                                    s.state,
                                    anagram_core::contracts::JobState::Queued
                                        | anagram_core::contracts::JobState::Running
                                )
                            }) {
                                let previous = inner.job.status.clone();
                                let mut publish =
                                    |status: &JobStatus| inner.job.status = Some(status.clone());
                                let mut progress = anagram_core::progress::Progress::new(
                                    &request,
                                    &limits,
                                    &control,
                                    &format!("desktop-{id}"),
                                    &mut publish,
                                );
                                if let Some(previous) = previous {
                                    progress.status = previous;
                                }
                                progress.status.counts.shown = 0;
                                progress.finish(Some(&error));
                            }
                            inner.job.error = Some(format!("{}: {}", error.code, error.message))
                        }
                    }
                    inner.job.active = false;
                    inner.operation = None;
                }
            });
        match spawn {
            Ok(worker) => inner.worker = Some(worker),
            Err(error) => {
                inner.job.active = false;
                inner.operation = None;
                inner.job.error = Some(error.to_string());
                return Err(error.to_string());
            }
        }
        Ok(id)
    }
    pub fn shutdown(&self) {
        let worker = if let Ok(mut inner) = self.inner.lock() {
            inner.closing = true;
            inner.control.cancel();
            inner.worker.take()
        } else {
            None
        };
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
    pub fn clear_cache(&self) -> Result<(), String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        inner.ensure_idle()?;
        let path = inner.settings.runtime.cache_path(&self.data);
        if !path.exists() {
            return Ok(());
        }
        let mut store = cache::Store::open(
            &path,
            inner.settings.runtime.cache_limits(),
            &Default::default(),
        )
        .map_err(|e| e.message)?;
        store.clear().map_err(|e| e.message)
    }
    pub fn bootstrap(&self) -> Result<Value, String> {
        let inner = self.inner.lock().map_err(|e| e.to_string())?;
        Ok(
            json!({"settings":inner.settings,"notice":inner.notice,"limits":settings::limits(),
            "wordle_frequencies_available":Path::new(&inner.settings.corpora.unigrams).is_file(),"corpus_error":inner.settings.corpora.validate().err(),"data_directory":self.data,
            "engine_version":env!("CARGO_PKG_VERSION"),"active_jobs":1,"queue_capacity":0}),
        )
    }
}

impl Desktop {
    pub fn operation(
        &self,
        seconds: u64,
        work: impl FnOnce(&Control) -> Result<Value, String> + Send + 'static,
    ) -> Result<(), String> {
        let mut inner = self.inner.lock().map_err(|e| e.to_string())?;
        inner.admit("maintenance")?;
        inner.control = Control::with_deadline(Instant::now() + Duration::from_secs(seconds));
        inner.experiment = crate::experimental::Snapshot {
            active: true,
            ..Default::default()
        };
        let control = inner.control.clone();
        let shared = self.inner.clone();
        #[cfg(test)]
        let worker_gate = inner.worker_gate.take();
        match std::thread::Builder::new()
            .name("TajsAnagrams-maintenance".into())
            .spawn(move || {
                #[cfg(test)]
                if let Some(gate) = worker_gate {
                    gate.wait();
                }
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&control)))
                        .unwrap_or_else(|_| Err("Operation stopped unexpectedly".into()));
                if let Ok(mut inner) = shared.lock() {
                    inner.experiment.active = false;
                    inner.operation = None;
                    match result {
                        Ok(report) => inner.experiment.report = Some(report),
                        Err(error) => inner.experiment.error = Some(error),
                    }
                }
            }) {
            Ok(worker) => inner.worker = Some(worker),
            Err(error) => {
                inner.experiment.active = false;
                inner.operation = None;
                inner.experiment.error = Some(error.to_string());
                return Err(error.to_string());
            }
        }
        Ok(())
    }
}
