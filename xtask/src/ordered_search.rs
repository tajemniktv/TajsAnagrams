//! Bounded development experiment, not an installed solver strategy.
use crate::{Args, Result, benchmark, digest, quality, research, write_json};
use anagram_core::{
    Inventory,
    control::Control,
    generation::HintMode,
    lexicon::Unigrams,
    request::{GenerateRequest, SearchSpace},
};
use serde_json::{Value, json};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    time::{Duration, Instant},
};

#[derive(Clone)]
struct State {
    remaining: Inventory,
    required: BTreeMap<String, usize>,
    used: BTreeSet<String>,
    order: Vec<String>,
    logp: f64,
    lexical: f64,
    short: usize,
    priority: f64,
}
// Heap head is the worst retained prefix, so pruning is bounded in memory.
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .total_cmp(&self.priority)
            .then_with(|| self.order.cmp(&other.order))
    }
}
impl PartialOrd for State {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl PartialEq for State {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for State {}
fn mean(s: &State) -> f64 {
    (s.logp + 0.05 * s.lexical - 0.15 * s.short as f64) / s.order.len().max(1) as f64
}
fn search(
    request: &GenerateRequest,
    space: SearchSpace,
    bounds: (usize, usize),
    control: &Control,
    zipf: impl Fn(&str) -> f64,
    mut probability: impl FnMut(&[String], &str) -> Result<f64>,
    observer: &mut dyn FnMut(&[String], f64),
) -> Result<Value> {
    let (beam, work_cap) = bounds;
    if beam == 0 || beam > 1024 || work_cap == 0 || work_cap > 1_000_000 || request.max_words > 10 {
        return Err(
            "Invalid research search limits (beam <=1024, work <=1000000, words <=10)".into(),
        );
    }
    let mut required = BTreeMap::new();
    for w in &space.required {
        *required.entry(w.clone()).or_default() += 1;
    }
    let mut optional = space
        .candidates
        .iter()
        .map(|w| (w.word.clone(), Inventory::from_text(&w.word)))
        .collect::<BTreeMap<_, _>>();
    let optional_allowed = optional.keys().cloned().collect::<BTreeSet<_>>();
    for w in &space.required {
        optional
            .entry(w.clone())
            .or_insert_with(|| Inventory::from_text(w));
    }
    let mut frontier = vec![State {
        remaining: space.remaining,
        required,
        used: BTreeSet::new(),
        order: vec![],
        logp: 0.,
        lexical: 0.,
        short: 0,
        priority: 0.,
    }];
    let mut buckets = BTreeMap::<usize, Vec<Value>>::new();
    let (mut work, mut evaluated, mut complete, mut pruned) = (0, 0, 0, false);
    let mut stop = "exhausted";
    let mut first_result_ms = None;
    let started = Instant::now();
    'depth: for _ in 0..request.max_words {
        let mut next = BinaryHeap::new();
        for state in &frontier {
            for (word, inventory) in &optional {
                if let Err(reason) = control.check() {
                    stop = reason;
                    break 'depth;
                }
                if work >= work_cap {
                    stop = "work_cap";
                    break 'depth;
                }
                work += 1;
                let pending = state.required.get(word).copied().unwrap_or(0);
                if pending == 0 && !optional_allowed.contains(word) {
                    continue;
                }
                if pending == 0 && !request.allow_repeat && state.used.contains(word) {
                    continue;
                }
                let remaining = if pending > 0 {
                    state.remaining
                } else {
                    let Some(r) = state.remaining.subtract(*inventory) else {
                        continue;
                    };
                    r
                };
                let mut child = state.clone();
                child.remaining = remaining;
                if pending > 0 {
                    if pending == 1 {
                        child.required.remove(word);
                    } else {
                        child.required.insert(word.clone(), pending - 1);
                    }
                } else {
                    child.used.insert(word.clone());
                }
                child.order.push(word.clone());
                let hints = space
                    .hints
                    .iter()
                    .filter(|h| child.order.contains(h))
                    .count();
                if matches!(request.hint_mode, HintMode::ExactlyOne) && hints > 1 {
                    continue;
                }
                let missing_required: usize = child.required.values().sum();
                let slots = request.max_words - child.order.len();
                if missing_required > slots
                    || (!child.remaining.is_empty() && slots <= missing_required)
                {
                    continue;
                }
                let p = probability(&state.order, word)?;
                if !p.is_finite() {
                    return Err("Nonfinite conditional score".into());
                }
                evaluated += 1;
                child.logp += p;
                child.lexical += zipf(word);
                child.short += usize::from(word.len() <= 2);
                child.priority = mean(&child);
                if child.remaining.is_empty() && child.required.is_empty() {
                    if child.order.len() < request.min_words
                        || (!space.hints.is_empty() && hints == 0)
                    {
                        continue;
                    }
                    child.logp += probability(&child.order, "</s>")?;
                    child.priority = mean(&child);
                    if !child.priority.is_finite() {
                        return Err("Nonfinite final score".into());
                    }
                    complete += 1;
                    first_result_ms.get_or_insert(started.elapsed().as_millis());
                    observer(&child.order, child.priority);
                    let mut bag = child.order.clone();
                    bag.sort();
                    let row = json!({"words":bag,"best_order":child.order,"display_phrase":anagram_core::format_phrase(&child.order),"final":child.priority});
                    let bucket = buckets.entry(child.order.len()).or_default();
                    if let Some(i) = bucket.iter().position(|r| r["words"] == row["words"]) {
                        if bucket[i]["final"].as_f64().unwrap() >= child.priority {
                            continue;
                        }
                        bucket.remove(i);
                    }
                    bucket.push(row);
                    bucket.sort_by(|a, b| {
                        b["final"]
                            .as_f64()
                            .unwrap()
                            .total_cmp(&a["final"].as_f64().unwrap())
                            .then_with(|| {
                                a["best_order"]
                                    .to_string()
                                    .cmp(&b["best_order"].to_string())
                            })
                    });
                    bucket.truncate(10);
                } else {
                    next.push(child);
                    if next.len() > beam {
                        next.pop();
                        pruned = true;
                    }
                }
            }
        }
        frontier = next.into_vec();
        frontier.sort_by(|a, b| {
            b.priority
                .total_cmp(&a.priority)
                .then_with(|| a.order.cmp(&b.order))
        });
        if frontier.is_empty() {
            break;
        }
    }
    if stop == "exhausted" && pruned {
        stop = "beam_pruned";
    }
    Ok(
        json!({"schema_version":1,"kind":"ordered_search_research","buckets":buckets,"generation_stop":stop,"search_exhausted":stop=="exhausted","final_score_optimality_claimed":false,"word_checks":work,"conditional_evaluations":evaluated,"completed_orders":complete,"beam_pruned":pruned,"first_result_ms":first_result_ms,"search_ms":started.elapsed().as_millis(),"score":"mean log10 LM including EOS + 0.05 mean Zipf - 0.15 short-word fraction; one best order per bag, ten bags per word count; not native final score"}),
    )
}

pub fn worker(a: &Args) -> Result<()> {
    let request: anagram_core::solve::Request =
        serde_json::from_reader(std::io::stdin().lock().take(65537))?;
    let timeout = a.number("timeout-ms", 10000)?;
    let control = Control::with_deadline(Instant::now() + Duration::from_millis(timeout));
    let outcome = (|| -> Result<Value> {
        let uni = Unigrams::load(BufReader::new(control.reader(File::open(
            a.value("unigrams", ".anagram_data/ngrams/count_1w.txt"),
        )?)))?;
        let space = anagram_core::request::prepare_search_space(
            &request.generation,
            BufReader::new(File::open(a.required("dictionary")?)?),
            Some(&uni),
            &control,
        )
        .map_err(|e| format!("{}: {}", e.code, e.message))?;
        let vocabulary = space
            .vocabulary
            .iter()
            .map(|w| anagram_core::format_phrase(std::slice::from_ref(w)))
            .chain(["<s>".into(), "</s>".into(), "<unk>".into()])
            .collect::<BTreeSet<_>>();
        let started = Instant::now();
        let model = research::load_arpa_matching(
            BufReader::new(control.reader(File::open(a.required("arpa")?)?)),
            |g| g.iter().all(|w| vocabulary.contains(w)),
        )?;
        let intake_ms = started.elapsed().as_millis();
        let prob = |prefix: &[String], word: &str| {
            let mut tokens = vec!["<s>".into()];
            for w in prefix
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(word))
            {
                let display = if w == "</s>" {
                    w.to_owned()
                } else {
                    anagram_core::format_phrase(&[w.into()])
                };
                tokens.push(if model.contains_key(&vec![display.clone()]) {
                    display
                } else {
                    "<unk>".into()
                });
            }
            Ok(research::conditional(&model, &tokens)?.0)
        };
        let mut result = search(
            &request.generation,
            space,
            (
                a.number("beam-width", 64)? as usize,
                a.number("work-cap", 200000)? as usize,
            ),
            &control,
            |w| uni.zipf(w),
            prob,
            &mut |_, _| {},
        )?;
        result["model_intake_ms"] = json!(intake_ms);
        Ok(result)
    })();
    let out = match outcome {
        Ok(v) => v,
        Err(e) => {
            json!({"error":{"code":control.check().err().unwrap_or("research_error"),"message":e.to_string()}})
        }
    };
    write_json(None, &out)
}

fn publish_manifest(path: &std::path::Path, value: &Value, replace: bool) -> Result<()> {
    if !replace {
        let mut file = File::create_new(path)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        file.flush()?;
        file.sync_all()?;
    } else {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer_pretty(&mut temp, value)?;
        temp.flush()?;
        temp.as_file().sync_all()?;
        temp.persist(path)?;
    }
    Ok(())
}
pub fn batch(a: &Args) -> Result<()> {
    let output = a.required("output")?;
    let manifest_path = std::path::PathBuf::from(format!("{output}.manifest.json"));
    if std::path::Path::new(output).exists() || manifest_path.exists() {
        return Err("Refusing to overwrite experiment".into());
    }
    let cases = quality::load_cases(a.value("cases", "tests/quality/cases.v4.json"))?;
    let mut jobs = cases
        .values()
        .flat_map(|c| [(&c.id, &c.source, "native"), (&c.id, &c.source, "ordered")])
        .collect::<Vec<_>>();
    benchmark::Rng::new(a.number("seed", 1608)?).shuffle(&mut jobs);
    let mut baseline = Args(a.0.clone());
    baseline
        .0
        .insert("cli".into(), a.required("before")?.into());
    let baseline = benchmark::cli_command(&baseline, "solve")?;
    let mut worker = vec![
        std::env::current_exe()?.to_string_lossy().into_owned(),
        "research".into(),
        "ordered-worker".into(),
    ];
    for k in [
        "dictionary",
        "unigrams",
        "arpa",
        "timeout-ms",
        "beam-width",
        "work-cap",
    ] {
        if let Some(v) = a.get(k) {
            worker.extend([format!("--{k}"), v.into()]);
        }
    }
    let timeout = a.number("timeout-ms", 10000)?;
    let mut f = BufWriter::new(File::create_new(output)?);
    let mut manifest = json!({"schema_version":1,"research_only":true,"source_head":String::from_utf8(std::process::Command::new("git").args(["rev-parse","HEAD"]).output()?.stdout)?.trim(),"source_dirty":true,"seed":a.number("seed",1608)?,"timeout_ms":timeout,"scope":"Randomized cold-process native/ordered pairs, same dictionary and generation request. Ordered model load is included in the deadline and wall time. Both are bounded; objectives differ. Not a production strategy or an optimality claim.","cases_sha256":digest(a.value("cases","tests/quality/cases.v4.json"))?,"dictionary_sha256":digest(a.required("dictionary")?)?,"model_sha256":digest(a.required("arpa")?)?,"native_binary_sha256":digest(a.required("before")?)?,"tool_binary_sha256":digest(std::env::current_exe()?)?,"requests":[]});
    manifest["beam_width"] = json!(a.number("beam-width", 64)?);
    manifest["work_cap"] = json!(a.number("work-cap", 200000)?);
    manifest["worker_command"] = json!(worker);
    manifest["native_command"] = json!(baseline);
    manifest["status"] = json!("running");
    manifest["scheduled_jobs"] = json!(
        jobs.iter()
            .map(|(id, _, variant)| json!({"case_id":id,"variant":variant}))
            .collect::<Vec<_>>()
    );
    // A process interruption must not leave valid raw records without the
    // experiment's options and input identities. Publish provenance up front.
    publish_manifest(&manifest_path, &manifest, false)?;
    for (id, source, variant) in jobs {
        let request = benchmark::desktop_request(source, 10000);
        let mut record = benchmark::capture(
            if variant == "native" {
                &baseline
            } else {
                &worker
            },
            &request,
            Duration::from_millis(timeout + 5000),
        );
        record["request"] = request.clone();
        record["case_id"] = json!(id);
        record["variant"] = json!(variant);
        record["cache_state"] = json!("uncached");
        record["trial"] = json!(0);
        manifest["requests"]
            .as_array_mut()
            .unwrap()
            .push(json!({"case_id":id,"variant":variant,"request":request}));
        writeln!(f, "{}", serde_json::to_string(&record)?)?;
        f.flush()?;
        eprintln!(
            "{id} / {variant}: {}ms {}",
            record["wall_ms"],
            record.get("error").unwrap_or(&json!("delivered"))
        );
    }
    manifest["capture_sha256"] = json!(digest(output)?);
    manifest["status"] = json!("complete");
    publish_manifest(&manifest_path, &manifest, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn experiment_manifest_is_owned_and_replaced_atomically() {
        let root = crate::root().join(".codex/temp/xtask/ordered-tests");
        std::fs::create_dir_all(&root).unwrap();
        let t = tempfile::tempdir_in(root).unwrap();
        let out = t.path().join("runs.jsonl");
        let manifest = t.path().join("runs.jsonl.manifest.json");
        publish_manifest(&manifest, &json!({"status":"running"}), false).unwrap();
        let args = Args::parse(
            &["--output".into(), out.to_string_lossy().into_owned()],
            &["output"],
            &[],
        )
        .unwrap();
        assert!(batch(&args).is_err());
        assert!(!out.exists());
        assert!(publish_manifest(&manifest, &json!({"status":"bad"}), false).is_err());
        assert_eq!(crate::read_json(&manifest).unwrap()["status"], "running");
        publish_manifest(&manifest, &json!({"status":"complete"}), true).unwrap();
        assert_eq!(crate::read_json(&manifest).unwrap()["status"], "complete");
        assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 1);
    }
    fn req(text: &str) -> GenerateRequest {
        let mut v = benchmark::desktop_request(text, 10000);
        v["generation"]["min_zipf"] = json!(0.);
        v["generation"]["short_policy"] = json!("all");
        serde_json::from_value(v["generation"].clone()).unwrap()
    }
    fn run(r: &GenerateRequest, control: &Control, beam: usize, cap: usize) -> Value {
        let dictionary = "a\nb\nab\nba\n";
        let space =
            anagram_core::request::prepare_search_space(r, dictionary.as_bytes(), None, control)
                .unwrap();
        search(
            r,
            space,
            (beam, cap),
            control,
            |_| 0.,
            |_, _| Ok(-1.),
            &mut |_, _| {},
        )
        .unwrap()
    }
    #[test]
    fn exhaustive_tiny_bags_match_native_and_independent_letters() {
        for repeat in [false, true] {
            for required in [vec![], vec!["a".to_owned()], vec!["a".into(), "a".into()]] {
                for hints in [
                    vec![],
                    vec!["a".to_owned()],
                    vec!["a".into(), "b".into()],
                    vec!["ab".into()],
                ] {
                    for hint_mode in [HintMode::Any, HintMode::ExactlyOne] {
                        let mut r = req("aabb");
                        r.max_words = 4;
                        r.allow_repeat = repeat;
                        r.required = required.clone();
                        r.hints = hints.clone();
                        r.hint_mode = hint_mode;
                        if let Err(expected) = anagram_core::request::validate(&r) {
                            assert_eq!(expected.code, "impossible_hints");
                            match anagram_core::request::prepare_search_space(
                                &r,
                                "a\nb\nab\nba\n".as_bytes(),
                                None,
                                &Control::default(),
                            ) {
                                Err(actual) => assert_eq!(actual.code, expected.code),
                                Ok(_) => panic!("invalid hint request was admitted"),
                            }
                            continue;
                        }
                        let v = run(&r, &Control::default(), 1024, 100000);
                        assert_eq!(v["generation_stop"], "exhausted");
                        let raw = anagram_core::request::generate_controlled(
                            &r,
                            "a\nb\nab\nba\n".as_bytes(),
                            None,
                            &Control::default(),
                        )
                        .unwrap();
                        let expected = raw
                            .bags
                            .into_iter()
                            .map(|mut w| {
                                w.sort();
                                w
                            })
                            .collect::<BTreeSet<_>>();
                        let rows = v["buckets"]
                            .as_object()
                            .unwrap()
                            .values()
                            .flat_map(|b| b.as_array().unwrap())
                            .collect::<Vec<_>>();
                        let actual = rows
                            .iter()
                            .map(|r| {
                                serde_json::from_value::<Vec<String>>(r["words"].clone()).unwrap()
                            })
                            .collect::<BTreeSet<_>>();
                        assert_eq!(actual, expected);
                        for r in rows {
                            assert_eq!(
                                quality::signature(r["display_phrase"].as_str().unwrap()),
                                "aabb"
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn limits_hints_exclusions_and_anytime_observer_are_explicit() {
        let mut r = req("aabb");
        r.max_words = 4;
        r.allow_repeat = true;
        r.hints = vec!["a".into(), "b".into()];
        r.hint_mode = HintMode::ExactlyOne;
        r.excluded = vec!["ba".into()];
        r.exclude_regex = vec!["^ab$".into()];
        let v = run(&r, &Control::default(), 64, 10000);
        assert!(v["buckets"].as_object().unwrap().is_empty());
        let mut r = req("aabb");
        r.max_words = 4;
        r.allow_repeat = true;
        assert_eq!(
            run(&r, &Control::default(), 1, 1)["generation_stop"],
            "work_cap"
        );
        assert_eq!(
            run(&r, &Control::default(), 1, 10000)["generation_stop"],
            "beam_pruned"
        );
        let control = Control::default();
        let space = anagram_core::request::prepare_search_space(
            &r,
            "a\nb\nab\nba\n".as_bytes(),
            None,
            &control,
        )
        .unwrap();
        let mut seen = 0;
        let out = search(
            &r,
            space,
            (64, 100000),
            &control,
            |_| 0.,
            |_, _| Ok(-1.),
            &mut |_, _| {
                seen += 1;
                control.cancel();
            },
        )
        .unwrap();
        assert!(seen > 0);
        assert_eq!(out["generation_stop"], "cancelled");
        assert!(!out["buckets"].as_object().unwrap().is_empty());
    }
    #[test]
    fn required_words_do_not_admit_extra_optional_copies_and_subtraction_is_shared() {
        let mut r = req("abcabc");
        r.subtract = vec!["cc".into()];
        r.required = vec!["ab".into()];
        r.max_words = 4;
        let control = Control::default();
        let space =
            anagram_core::request::prepare_search_space(&r, "a\nb\n".as_bytes(), None, &control)
                .unwrap();
        let out = search(
            &r,
            space,
            (1024, 100000),
            &control,
            |_| 0.,
            |_, _| Ok(-1.),
            &mut |_, _| {},
        )
        .unwrap();
        assert!(out["buckets"].get("2").is_none());
        assert!(
            out["buckets"]["3"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| quality::signature(row["display_phrase"].as_str().unwrap()) == "aabb")
        );
        let space =
            anagram_core::request::prepare_search_space(&r, "a\nb\n".as_bytes(), None, &control)
                .unwrap();
        let expired = Control::with_deadline(Instant::now() - Duration::from_secs(1));
        let out = search(
            &r,
            space,
            (64, 10000),
            &expired,
            |_| 0.,
            |_, _| Ok(-1.),
            &mut |_, _| {},
        )
        .unwrap();
        assert_eq!(out["generation_stop"], "timed_out");
        assert_eq!(out["search_exhausted"], false);
    }
}
