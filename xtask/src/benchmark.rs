//! Isolated native executables, bounded process capture, sampled process-tree RSS.
use crate::{
    Args, Result, digest,
    quality::{self, Case, Cases},
    root, run, stages, write_json,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub fn desktop_request(source: &str, budget: u64) -> Value {
    json!({"generation":{"schema_version":1,"text":source,"required":[],"hints":[],"excluded":[],"exclude_regex":[],"forbid_chars":"","short_policy":"common","extra_short_words":["hi"],"min_words":1,"max_words":6,"min_word_length":1,"max_word_length":24,"min_zipf":1.8,"candidate_budget":budget,"allow_repeat":false,"strategy":"prefix","hint_mode":"any"},"deep_per_group":100,"deep_all":false,"order_mode":"auto","beam_width":128,"exact_max_words":5,"retained_orders":56,"phrase_rescore_top":300,"phrase_bonus_max":6.0,"positive_bigrams":true,"result_limit_per_group":10})
}
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}
fn bounded_read(mut reader: impl Read, limit: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut bytes = vec![];
    let mut buffer = [0; 16384];
    let mut exceeded = false;
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        let keep = n.min(limit.saturating_sub(bytes.len()));
        bytes.extend_from_slice(&buffer[..keep]);
        exceeded |= keep < n;
    }
    Ok((bytes, exceeded))
}
pub fn capture(command: &[String], request: &Value, guard: Duration) -> Value {
    let started = Instant::now();
    let mut record = json!({});
    let result = (|| -> Result<()> {
        let mut child = Command::new(command.first().ok_or("Missing executable")?)
            .args(&command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let input = serde_json::to_vec(request)?;
        let output = thread::spawn(move || bounded_read(stdout, 64 * 1024 * 1024));
        let errors = thread::spawn(move || bounded_read(stderr, 2 * 1024 * 1024));
        let write = thread::spawn(move || stdin.write_all(&input));
        let pid = Pid::from_u32(child.id());
        let mut sys = System::new();
        let mut descendants = BTreeMap::new();
        let mut peak = None;
        let mut samples = 0;
        let mut timeout = false;
        let status = loop {
            sys.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
            let mut tree = BTreeSet::from([pid]);
            loop {
                let before = tree.len();
                for (id, p) in sys.processes() {
                    if p.parent().is_some_and(|parent| tree.contains(&parent)) {
                        tree.insert(*id);
                    }
                }
                if tree.len() == before {
                    break;
                }
            }
            for id in tree.iter().filter(|p| **p != pid) {
                if let Some(p) = sys.process(*id) {
                    descendants.insert(*id, p.start_time());
                }
            }
            let measured = tree
                .iter()
                .filter_map(|p| sys.process(*p))
                .collect::<Vec<_>>();
            if !measured.is_empty() {
                let rss = measured.iter().map(|p| p.memory()).sum::<u64>();
                peak = Some(peak.unwrap_or(0).max(rss));
                samples += 1;
            }
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() >= guard {
                timeout = true;
                for (id, start) in &descendants {
                    if let Some(p) = sys.process(*id).filter(|p| p.start_time() == *start) {
                        p.kill();
                    }
                }
                let _ = child.kill();
                break child.wait()?;
            }
            thread::sleep(Duration::from_millis(20));
        };
        // Native CLI/probe children are not daemons. Do not leave observed owned descendants holding pipe handles.
        for (id, start) in &descendants {
            if let Some(p) = sys.process(*id).filter(|p| p.start_time() == *start) {
                p.kill();
            }
        }
        let (stdout, overflow) = output.join().map_err(|_| "stdout reader panicked")??;
        let (stderr, stderr_overflow) = errors.join().map_err(|_| "stderr reader panicked")??;
        let write = write.join().map_err(|_| "stdin writer panicked")?;
        record["returncode"] = json!(status.code());
        record["peak_rss_bytes"] = json!(peak);
        record["rss_samples"] = json!(samples);
        record["rss_sample_sleep_ms"] = json!(20);
        if !stderr.is_empty() {
            record["stderr"] = json!(
                String::from_utf8_lossy(&stderr)
                    .chars()
                    .take(2048)
                    .collect::<String>()
            );
            record["stderr_truncated"] = json!(stderr_overflow || stderr.len() > 2048);
        }
        if timeout {
            record["error"] = json!({"code":"process_timeout","message":"Native child exceeded subprocess guard"});
        } else if overflow {
            record["error"] = json!({"code":"output_too_large","message":"Native stdout exceeded 64 MiB capture budget"});
        } else {
            let response = serde_json::from_slice::<Value>(&stdout)
                .ok()
                .filter(Value::is_object);
            match response {
                None => {
                    record["error"] = json!({"code":"invalid_response","message":"CLI did not return a JSON object"});
                    record["stdout"] = json!(
                        String::from_utf8_lossy(&stdout)
                            .chars()
                            .take(2048)
                            .collect::<String>()
                    );
                }
                Some(r) if r.get("error").is_some() => record["error"] = r["error"].clone(),
                Some(_) if !status.success() => {
                    record["error"] = json!({"code":"process_failed","message":format!("CLI exited with {status}")})
                }
                Some(_) if write.is_err() => {
                    record["error"] = json!({"code":"process_input_error","message":write.unwrap_err().to_string()})
                }
                Some(r) => record["result"] = r,
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        record["error"] = json!({"code":"process_error","message":e.to_string()});
    }
    record["wall_ms"] = json!(started.elapsed().as_secs_f64() * 1000.);
    record
}
fn path(a: &Args, key: &str, default: &str) -> Result<String> {
    Ok(fs::canonicalize(a.value(key, default))?
        .to_string_lossy()
        .into_owned())
}
pub fn cli_command(a: &Args, mode: &str) -> Result<Vec<String>> {
    let cli_default = if cfg!(windows) {
        "build/release/anagram-cli.exe"
    } else {
        "build/release/anagram-cli"
    };
    let mut cmd = vec![
        path(a, "cli", cli_default)?,
        mode.into(),
        path(
            a,
            "dictionary",
            ".anagram_data/dictionary/normal_user_v2.txt",
        )?,
        path(a, "unigrams", ".anagram_data/ngrams/count_1w.txt")?,
    ];
    if mode == "solve" {
        cmd.push(path(a, "bigrams", ".anagram_data/ngrams/count_2w.txt")?);
        cmd.push(path(a, "wordnet", ".anagram_data/wordnet31/dict")?);
        cmd.push(
            a.get("phrase-db")
                .map(|_| path(a, "phrase-db", ""))
                .transpose()?
                .unwrap_or("-".into()),
        );
        if a.has("model") {
            cmd.push(path(a, "model", "")?);
        }
    }
    cmd.extend([
        "--timeout-ms".into(),
        a.number("timeout-ms", 120000)?.to_string(),
    ]);
    Ok(cmd)
}
fn metadata(a: &Args, cases: &Cases, command: &[String]) -> Result<Value> {
    let mut inputs = BTreeMap::new();
    let mut add = |role: &str, path: &Path| -> Result<()> {
        inputs.insert(
            role.to_owned(),
            json!({"sha256":digest(path)?,"bytes":path.metadata()?.len()}),
        );
        Ok(())
    };
    add("cli", Path::new(&command[0]))?;
    add("dictionary", Path::new(&command[2]))?;
    add("unigrams", Path::new(&command[3]))?;
    add(
        "cases",
        Path::new(
            a.get("workloads")
                .unwrap_or(a.value("cases", "tests/quality/cases.v1.json")),
        ),
    )?;
    add(
        "desktop_defaults_source",
        &root().join("apps/desktop/src-tauri/src/settings.rs"),
    )?;
    if command[1] == "solve" || a.has("stages") {
        add(
            "bigrams",
            Path::new(&path(a, "bigrams", ".anagram_data/ngrams/count_2w.txt")?),
        )?;
        let wordnet = path(a, "wordnet", ".anagram_data/wordnet31/dict")?;
        for entry in fs::read_dir(wordnet)? {
            let e = entry?;
            if e.file_type()?.is_file() {
                add(
                    &format!("wordnet/{}", e.file_name().to_string_lossy()),
                    &e.path(),
                )?;
            }
        }
    }
    for role in ["phrase-db", "model", "before", "ordering-probe"] {
        if let Some(p) = a.get(role) {
            add(role, Path::new(p))?;
        }
    }
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    Ok(
        json!({"schema_version":2,"inputs":inputs,"case_ids":cases.keys().collect::<Vec<_>>(),"source_head":git(&["rev-parse","HEAD"]),"source_dirty":git(&["status","--porcelain"]).map(|s|!s.is_empty()),"os_filesystem_cache":"uncontrolled","runtime":"isolated native CLI/probe, not installed desktop","binary_source_correspondence":"Hashes recorded; source/build correspondence requires separate build evidence","rng":"splitmix64 Fisher-Yates v1","rss":"20ms sampled process-tree resident memory, not guaranteed peak; null if unobserved"}),
    )
}
pub fn semantic_identity(result: &Value) -> Value {
    let mut v = result.clone();
    if let Some(m) = v.as_object_mut() {
        m.remove("engine_version");
    }
    v
}
pub fn paired_summary(records: &[Value]) -> Result<Value> {
    let mut groups: BTreeMap<_, Vec<&Value>> = BTreeMap::new();
    for r in records {
        groups
            .entry((
                r["case_id"].as_str().ok_or("Missing case")?,
                r["strategy"].as_str().ok_or("Missing strategy")?,
            ))
            .or_default()
            .push(r);
    }
    let mut out = vec![];
    for ((id, strategy), group) in groups {
        let mut row = json!({"case_id":id,"strategy":strategy});
        for v in ["before", "after"] {
            let runs = group
                .iter()
                .filter(|r| r["variant"] == v)
                .collect::<Vec<_>>();
            let times = runs
                .iter()
                .map(|r| quality::wall(r))
                .collect::<Result<Vec<_>>>()?;
            row[v] = json!({"median_attempt_ms":quality::percentile(&times,0.5),"attempted":runs.len(),"completed":runs.iter().filter(|r|r.get("error").is_none()).count()});
        }
        let mut trials: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        for r in &group {
            let entry = trials
                .entry(r["trial"].as_u64().ok_or("Invalid trial")?)
                .or_default();
            if entry
                .insert(r["variant"].as_str().ok_or("Missing variant")?, *r)
                .is_some()
            {
                return Err("Duplicate paired record".into());
            }
        }
        let pairs = trials
            .values()
            .filter(|t| {
                t.len() == 2
                    && t.contains_key("before")
                    && t.contains_key("after")
                    && t.values().all(|r| r.get("error").is_none())
            })
            .collect::<Vec<_>>();
        row["compared_completed_pairs"] = json!(pairs.len());
        row["completed_output_equal"] = if pairs.is_empty() {
            Value::Null
        } else {
            json!(
                pairs
                    .iter()
                    .all(|p| semantic_identity(&p["before"]["result"])
                        == semantic_identity(&p["after"]["result"]))
            )
        };
        let speedups = pairs
            .iter()
            .map(|p| {
                let after = quality::wall(p["after"])?;
                if after <= 0. {
                    return Err("Nonpositive paired duration".into());
                }
                Ok(quality::wall(p["before"])? / after)
            })
            .collect::<Result<Vec<_>>>();
        row["median_paired_speedup"] =
            speedups.map_or(Value::Null, |v| quality::percentile(&v, 0.5));
        out.push(row);
    }
    Ok(json!(out))
}
fn list(a: &Args, key: &str, default: &str) -> Vec<String> {
    a.value(key, default).split(',').map(String::from).collect()
}
fn append(out: &mut File, records: &mut Vec<Value>, record: Value) -> Result<()> {
    writeln!(out, "{}", serde_json::to_string(&record)?)?;
    out.flush()?;
    eprintln!(
        "{} / {} / {}: {:.0}ms {}",
        record["case_id"],
        record["variant"],
        record["phase"].as_str().unwrap_or("solve"),
        record["wall_ms"].as_f64().unwrap_or(0.),
        record["error"]["code"].as_str().unwrap_or("completed")
    );
    records.push(record);
    Ok(())
}
fn stage_capture(
    a: &Args,
    cases: &Cases,
    output: &mut File,
    records: &mut Vec<Value>,
    budget: u64,
    guard: Duration,
) -> Result<()> {
    let solve = cli_command(a, "solve")?;
    let generate = cli_command(a, "generate")?;
    let mut record = |case: &Case,
                      variant: &str,
                      phase: &str,
                      request: Value,
                      command: &[String],
                      trial: u64,
                      limit: usize|
     -> Result<()> {
        let mut r = capture(command, &request, guard);
        if r.get("error").is_none() {
            let validation = if phase == "ordering" {
                Ok(())
            } else {
                crate::verify::output(&request, &r["result"])
            };
            if let Err(e) = validation {
                r["error"] = json!({"code":"invalid_result","message":e.to_string()});
            }
        }
        r.as_object_mut().unwrap().extend(json!({"case_id":case.id,"variant":variant,"phase":phase,"trial":trial,"cache_state":"uncached","request":request}).as_object().unwrap().clone());
        stages::annotate(case, phase, &mut r, limit);
        append(output, records, r)
    };
    let words = cases
        .values()
        .flat_map(|c| c.relevance.keys())
        .flat_map(|p| {
            quality::phrase_key(p)
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>();
    for word in words {
        let c = Case {
            id: format!("word:{word}"),
            source: word.clone(),
            split: "train".into(),
            provenance: "Admission intervention".into(),
            relevance: BTreeMap::from([(word.clone(), 3)]),
            negatives: vec![],
        };
        let mut req = desktop_request(&word, 500)["generation"].clone();
        req["min_words"] = json!(1);
        req["max_words"] = json!(1);
        record(
            &c,
            "native_admission",
            "admission",
            req.clone(),
            &generate,
            0,
            10,
        )?; // Admission retries below are explicit zipf interventions.
        req["min_zipf"] = json!(0.);
        record(
            &c,
            "native_admission_zipf_zero",
            "admission",
            req,
            &generate,
            0,
            10,
        )?;
    }
    let mut schedule = vec![];
    let strategies = list(a, "strategies", "prefix,diverse");
    let budgets = list(a, "candidate-budgets", &budget.to_string())
        .iter()
        .map(|s| s.parse::<u64>())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if budgets.contains(&0)
        || strategies
            .iter()
            .any(|s| !["prefix", "diverse"].contains(&s.as_str()))
    {
        return Err("Invalid stage budget/strategy".into());
    }
    for trial in 0..a.number("repeats", 1)? {
        for c in cases.values() {
            for s in &strategies {
                for b in &budgets {
                    schedule.push((c, s, *b, trial));
                }
            }
        }
    }
    Rng::new(a.number("seed", 13)?).shuffle(&mut schedule);
    for (c, strategy, b, trial) in schedule {
        let mut req = desktop_request(&c.source, b);
        req["generation"]["strategy"] = json!(strategy);
        let variant = format!("native_{strategy}_{b}");
        record(
            c,
            &variant,
            "generation",
            req["generation"].clone(),
            &generate,
            trial,
            10,
        )?;
        req["result_limit_per_group"] = json!(b.max(10));
        record(c, &variant, "shortlist", req.clone(), &solve, trial, 10)?;
        if a.has("deep-all") {
            req["deep_all"] = json!(true);
            record(c, &variant, "deep_all", req, &solve, trial, 10)?;
        }
    }
    for c in cases.values() {
        for gold in c.relevance.keys() {
            let mut req = desktop_request(&c.source, 500);
            let words = quality::phrase_key(gold)
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<_>>();
            req["generation"]["required"] = json!(words);
            req["order_mode"] = json!("exact");
            record(
                c,
                "native_required_bag",
                "required_gold_bag",
                req,
                &solve,
                0,
                10,
            )?;
            if let Some(probe) = a.get("ordering-probe") {
                let n = words.len();
                let factorial = (1..=n).product::<usize>().min(512);
                let cmd = vec![
                    fs::canonicalize(probe)?.to_string_lossy().into_owned(),
                    path(a, "wordnet", ".anagram_data/wordnet31/dict")?,
                ];
                record(
                    c,
                    "native_order_pool",
                    "ordering",
                    json!({"words":words,"exact":true,"beam_width":128,"top_k":factorial}),
                    &cmd,
                    0,
                    10,
                )?;
            }
        }
    }
    Ok(())
}
pub fn command(args: &[String]) -> Result<()> {
    if args == ["--help"] {
        println!(
            "benchmark [--cli PATH --before PATH --mode solve|generate --cases FILE --case-ids comma,list --dictionary FILE --unigrams FILE --bigrams FILE --wordnet DIR --phrase-db FILE --model FILE --candidate-budget N --strategy prefix|diverse --timeout-ms N --guard-ms N --repeats N --seed N --shuffle-input --output FILE]\n--before makes randomized paired native revision measurements, no parity gate.\n--stages [--strategies prefix,diverse --candidate-budgets 500,5000 --deep-all --ordering-probe PATH] isolates native admission/generation/shortlist/order/display. Required-bag interventions are not end-to-end quality successes.\nDefault cases are seven familiar fixtures, not a comprehensive gate."
        );
        println!(
            "--workloads FILE uses explicit native stress requests and verifies invariants/cache roundtrips without quality labels. Use --case-ids to select workloads. Active corpus preparation remains tools/prepare-native-data.ps1."
        );
        return Ok(());
    }
    let a = Args::parse(
        args,
        &[
            "cli",
            "before",
            "mode",
            "cases",
            "case-ids",
            "dictionary",
            "unigrams",
            "bigrams",
            "wordnet",
            "phrase-db",
            "model",
            "candidate-budget",
            "strategy",
            "timeout-ms",
            "guard-ms",
            "repeats",
            "seed",
            "output",
            "strategies",
            "candidate-budgets",
            "ordering-probe",
            "workloads",
        ],
        &["shuffle-input", "stages", "deep-all"],
    )?;
    let mode = a.value("mode", "solve");
    if !["solve", "generate"].contains(&mode) {
        return Err("Invalid mode".into());
    }
    if a.has("stages") && a.has("before") {
        return Err("Stage interventions cannot be paired revision runs".into());
    }
    if a.has("workloads")
        && [
            "cases",
            "candidate-budget",
            "strategy",
            "shuffle-input",
            "stages",
            "mode",
        ]
        .iter()
        .any(|k| a.has(k))
    {
        return Err("Workloads carry explicit solve requests; do not combine with case/profile/stage overrides".into());
    }
    let budget = a.number("candidate-budget", 10000)?;
    let timeout = a.number("timeout-ms", 120000)?;
    let repeats = a.number("repeats", 1)?;
    if budget == 0 || timeout == 0 || repeats == 0 {
        return Err("Positive budgets/repeats required".into());
    }
    let guard = Duration::from_millis(a.number(
        "guard-ms",
        timeout.checked_add(30000).ok_or("Timeout overflow")?,
    )?);
    if guard.is_zero() {
        return Err("Positive subprocess guard required".into());
    }
    if !a.has("cli") {
        run(Command::new("cargo").args(["build", "--release", "--locked", "-p", "anagram-cli"]))?;
    }
    let mut overrides = BTreeMap::new();
    let mut cases = if let Some(path) = a.get("workloads") {
        let v = crate::read_json(path)?;
        if v["schema_version"] != 1 {
            return Err("Unsupported workload schema".into());
        }
        let mut cases = Cases::new();
        for workload in v["workloads"].as_array().ok_or("Missing workloads")? {
            let id = workload["id"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("Missing workload ID")?;
            let provenance = workload["provenance"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("Missing workload provenance")?;
            let request = workload["request"].clone();
            let _: anagram_core::solve::Request = serde_json::from_value(request.clone())?;
            let source = request["generation"]["text"]
                .as_str()
                .ok_or("Missing workload text")?;
            if cases
                .insert(
                    id.into(),
                    Case {
                        id: id.into(),
                        source: source.into(),
                        split: "train".into(),
                        relevance: BTreeMap::new(),
                        negatives: vec![],
                        provenance: provenance.into(),
                    },
                )
                .is_some()
            {
                return Err("Duplicate workload ID".into());
            }
            overrides.insert(id.to_owned(), request);
        }
        if cases.is_empty() {
            return Err("Empty workloads".into());
        }
        cases
    } else {
        quality::load_cases(a.value("cases", "tests/quality/cases.v1.json"))?
    };
    if a.has("case-ids") {
        let ids = list(&a, "case-ids", "");
        if ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
            || ids.iter().any(|id| !cases.contains_key(id))
        {
            return Err("Case IDs must be known and unique".into());
        }
        cases.retain(|id, _| ids.contains(id));
    }
    let command = cli_command(&a, mode)?;
    let mut manifest = metadata(&a, &cases, &command)?;
    let output = PathBuf::from(a.value("output", ".codex/temp/native-benchmark/runs.jsonl"));
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let scratch = root().join(".codex/temp/xtask/benchmark");
    fs::create_dir_all(&scratch)?;
    let cache_root = tempfile::tempdir_in(scratch)?;
    let mut file = File::create(&output)?;
    let mut records = vec![];
    let mut rng = Rng::new(a.number("seed", 13)?);
    let mut requests = BTreeMap::new();
    for c in cases.values() {
        if let Some(request) = overrides.get(&c.id) {
            requests.insert(c.id.clone(), request.clone());
            continue;
        }
        let mut letters = quality::key(&c.source).into_bytes();
        if a.has("shuffle-input") {
            let mut r = Rng::new(a.number("seed", 13)? ^ u64::from_le_bytes(sha_prefix(&c.id)));
            r.shuffle(&mut letters);
        }
        let mut req = desktop_request(&String::from_utf8(letters)?, budget);
        let strategy = a.value("strategy", "prefix");
        if !["prefix", "diverse"].contains(&strategy) {
            return Err("Invalid strategy".into());
        }
        req["generation"]["strategy"] = json!(strategy);
        requests.insert(
            c.id.clone(),
            if mode == "generate" {
                req["generation"].clone()
            } else {
                req
            },
        );
    }
    manifest["requests"] = json!(requests);
    manifest["timeout_ms"] = json!(timeout);
    manifest["guard_ms"] = json!(guard.as_millis());
    manifest["seed"] = json!(a.number("seed", 13)?);
    manifest["repeats"] = json!(repeats);
    manifest["shuffle_input"] = json!(a.has("shuffle-input"));
    manifest["stages"] = json!(a.has("stages"));
    manifest["quality_labels"] = json!(!a.has("workloads"));
    write_json(output.with_extension("manifest.json").to_str(), &manifest)?;
    if a.has("stages") {
        stage_capture(&a, &cases, &mut file, &mut records, budget, guard)?;
        write_json(
            output.with_extension("summary.json").to_str(),
            &json!(
                records
                    .iter()
                    .map(|r| r["summary"].clone())
                    .collect::<Vec<_>>()
            ),
        )?;
        return Ok(());
    }
    let mut schedule = vec![];
    for trial in 0..repeats {
        for c in cases.values() {
            schedule.push((c, trial));
        }
    }
    rng.shuffle(&mut schedule);
    for (c, trial) in schedule {
        let req = &requests[&c.id];
        let cache = cache_root
            .path()
            .join(format!("{}-{trial}.sqlite", hex::encode(sha_prefix(&c.id))));
        let mut variants = if a.has("before") {
            vec!["before", "after"]
        } else {
            vec!["native"]
        };
        rng.shuffle(&mut variants);
        for variant in variants {
            let mut cmd = command.clone();
            if variant == "before" {
                cmd[0] = fs::canonicalize(a.required("before")?)?
                    .to_string_lossy()
                    .into_owned();
            }
            let variant_id = if a.has("before") {
                variant.into()
            } else {
                format!(
                    "native_{mode}_{}_{}_{}ms_binary-{}{}{}{}",
                    req["generation"]["strategy"]
                        .as_str()
                        .or_else(|| req["strategy"].as_str())
                        .unwrap(),
                    req["generation"]["candidate_budget"]
                        .as_u64()
                        .or_else(|| req["candidate_budget"].as_u64())
                        .unwrap(),
                    timeout,
                    &digest(&cmd[0])?[..12],
                    a.get("phrase-db")
                        .map(|p| digest(p).map(|h| format!("_phrase-{}", &h[..12])))
                        .transpose()?
                        .unwrap_or_default(),
                    a.get("model")
                        .map(|p| digest(p).map(|h| format!("_model-{}", &h[..12])))
                        .transpose()?
                        .unwrap_or_default(),
                    if a.has("shuffle-input") {
                        format!("_shuffled_seed{}", a.number("seed", 13)?)
                    } else {
                        String::new()
                    }
                )
            };
            let states = if mode == "solve" && !a.has("before") {
                vec!["disk_cache_empty", "disk_cache_reuse"]
            } else {
                vec!["uncached"]
            };
            let mut cold_result: Option<Value> = None;
            for state in states {
                let mut cmd = cmd.clone();
                if state != "uncached" {
                    cmd.extend(["--cache".into(), cache.to_string_lossy().into_owned()]);
                }
                let mut r = capture(&cmd, req, guard);
                r.as_object_mut().unwrap().extend(json!({"case_id":c.id,"variant":variant_id,"trial":trial,"cache_state":state,"request":req,"strategy":req["generation"]["strategy"].as_str().or_else(||req["strategy"].as_str()).unwrap()}).as_object().unwrap().clone());
                if r.get("error").is_none() {
                    let validation = (|| -> Result<()> {
                        crate::verify::output(req, &r["result"])?;
                        if !a.has("workloads") && mode == "solve" {
                            quality::metrics(c, &quality::rows(&r, "desktop")?)?;
                        }
                        if state == "disk_cache_reuse"
                            && let Some(cold) = &cold_result
                            && (r["result"]["status"]["cache"]["hit"] != true
                                || r["result"]["buckets"] != cold["buckets"])
                        {
                            return Err("Successful cold/warm cache roundtrip changed result or missed cache".into());
                        }
                        Ok(())
                    })();
                    if let Err(e) = validation {
                        r["error"] = json!({"code":"invalid_result","message":e.to_string()});
                    }
                    if state == "disk_cache_empty" && r.get("error").is_none() {
                        cold_result = Some(r["result"].clone());
                    }
                }
                r["persistent_bytes"] = json!(cache.metadata().ok().map(|m| m.len()));
                append(&mut file, &mut records, r)?;
            }
        }
    }
    if a.has("before") {
        write_json(
            output.with_extension("summary.json").to_str(),
            &paired_summary(&records)?,
        )?;
    } else if a.has("workloads") {
        let summary=records.iter().map(|r|json!({"case_id":r["case_id"],"cache_state":r["cache_state"],"wall_ms":r["wall_ms"],"peak_rss_bytes":r["peak_rss_bytes"],"error":r["error"],"cache_hit":r["result"]["status"]["cache"]["hit"],"generated":r["result"]["generated"],"generation_stop":r["result"]["generation_stop"]})).collect::<Vec<_>>();
        write_json(
            output.with_extension("summary.json").to_str(),
            &json!({"quality_labels":false,"attempts":summary.len(),"records":summary}),
        )?;
    }
    Ok(())
}
fn sha_prefix(s: &str) -> [u8; 8] {
    use sha2::{Digest, Sha256};
    Sha256::digest(s.as_bytes())[..8].try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_defaults_do_not_drift() {
        let source = include_str!("../../apps/desktop/src-tauri/src/settings.rs");
        let body = source
            .split("request: json!(")
            .nth(1)
            .unwrap()
            .split("),")
            .next()
            .unwrap();
        let expected: Value = serde_json::from_str(body).unwrap();
        assert_eq!(expected, desktop_request("", 10000));
    }
    #[test]
    fn shuffle_repeatability_and_inventory() {
        let mut a = b"testinganagramsolver".to_vec();
        let mut b = a.clone();
        Rng::new(13).shuffle(&mut a);
        Rng::new(13).shuffle(&mut b);
        assert_eq!(a, b);
        a.sort();
        b = b"testinganagramsolver".to_vec();
        b.sort();
        assert_eq!(a, b);
    }
    #[test]
    fn failed_or_missing_pairs_are_unknown() {
        let r = json!({"case_id":"a","strategy":"prefix","trial":0,"variant":"before","wall_ms":1,"error":{"code":"timed_out"}});
        let v = paired_summary(&[r]).unwrap();
        assert!(v[0]["completed_output_equal"].is_null());
        assert!(v[0]["median_paired_speedup"].is_null());
        assert_eq!(v[0]["after"]["attempted"], 0);
    }
    #[test]
    fn pair_identity_includes_stop_and_order() {
        let a = json!({"engine_version":"v1","bags":[["a"],["b"]],"stop":"exhausted"});
        let mut b = a.clone();
        b["engine_version"] = json!("v2");
        assert_eq!(semantic_identity(&a), semantic_identity(&b));
        b["stop"] = json!("candidate_cap");
        assert_ne!(semantic_identity(&a), semantic_identity(&b));
        b["stop"] = json!("exhausted");
        b["bags"] = json!([["b"], ["a"]]);
        assert_ne!(semantic_identity(&a), semantic_identity(&b));
    }
    #[test]
    fn process_launch_error_is_recorded() {
        let r = capture(
            &["no-such-tajsanagrams-executable".into()],
            &json!({}),
            Duration::from_millis(1),
        );
        assert_eq!(r["error"]["code"], "process_error");
        assert!(r["wall_ms"].as_f64().unwrap() >= 0.);
    }
}
