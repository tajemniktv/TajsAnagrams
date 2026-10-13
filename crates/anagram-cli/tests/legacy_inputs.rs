use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.codex/temp")
            .join(format!("native-compat-{}-{name}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn run(path: &Path, args: &[&str], request: &Value) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_anagram-cli"))
        .arg("generate")
        .arg(path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
fn request() -> Value {
    json!({"schema_version":1,"text":"atexyz","required":[],"hints":["eat"],"excluded":[],
        "min_words":1,"max_words":1,"min_word_length":1,"max_word_length":8,"min_zipf":0.0,
        "candidate_budget":1,"allow_repeat":false,"strategy":"prefix","hint_mode":"any"})
}

#[test]
fn native_subtract_exclusion_import_and_new_exports_work_through_the_cli() {
    let temp = Scratch::new("export");
    let dictionary = temp.0.join("dictionary");
    fs::write(&dictionary, "ate\neat\ntea\n").unwrap();
    let exclusions = temp.0.join("exclude");
    fs::write(&exclusions, "ate\ntea\n").unwrap();
    let stream = temp.0.join("stream.jsonl");
    let export = temp.0.join("result.json");
    let args = [
        "--hint-stats",
        "--benchmark-answer",
        "eat",
        "--subtract",
        "x y z",
        "--exclude-file",
        exclusions.to_str().unwrap(),
        "--stream-export",
        stream.to_str().unwrap(),
        "--export",
        export.to_str().unwrap(),
    ];
    let (success, result) = run(&dictionary, &args, &request());
    assert!(success, "{result}");
    assert_eq!(result["normalized_input"], "ate");
    assert_eq!(result["bags"], json!([["eat"]]));
    assert_eq!(result["hint_stats"]["counts"]["eat"], 1);
    assert_eq!(result["benchmark"]["matches"], json!([1]));
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&export).unwrap()).unwrap(),
        result
    );
    let records: Vec<Value> = fs::read_to_string(&stream)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records[0]["complete"], false);
    assert_eq!(records[1]["bag"], json!(["eat"]));
    assert_eq!(records[2]["complete"], true);
    assert_eq!(records.len(), 3);
    let original = fs::read(&stream).unwrap();
    let (success, _) = run(&dictionary, &args, &request());
    assert!(!success);
    assert_eq!(fs::read(&stream).unwrap(), original);
}

#[test]
fn streaming_does_not_export_the_extra_cap_probe() {
    let temp = Scratch::new("cap");
    let dictionary = temp.0.join("dictionary");
    fs::write(&dictionary, "ate\neat\ntea\n").unwrap();
    let stream = temp.0.join("stream");
    let mut req = request();
    req["text"] = "ate".into();
    req["hints"] = json!([]);
    let (success, result) = run(
        &dictionary,
        &["--stream-export", stream.to_str().unwrap()],
        &req,
    );
    assert!(success);
    assert_eq!(result["generated"], 1);
    assert_eq!(result["stop"], "candidate_cap");
    let lines: Vec<Value> = fs::read_to_string(stream)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1]["bag"], result["bags"][0]);
}

#[test]
fn ranked_reports_exports_and_subtraction_cache_hits_preserve_solver_results() {
    let temp = Scratch::new("ranked");
    for name in [
        "dictionary",
        "one",
        "two",
        "index.noun",
        "index.verb",
        "index.adj",
        "index.adv",
        "data.verb",
        "noun.exc",
        "verb.exc",
    ] {
        fs::write(
            temp.0.join(name),
            match name {
                "dictionary" => "ate\n",
                "one" => "ate\t10\n",
                _ => "",
            },
        )
        .unwrap();
    }
    let export = temp.0.join("export.json");
    let deep = temp.0.join("deep.json");
    let cache = temp.0.join("cache.sqlite");
    let req = json!({"generation":request(),"deep_per_group":10,"deep_all":false,
        "order_mode":"auto","beam_width":8,"exact_max_words":5,"retained_orders":8,
        "phrase_rescore_top":10,"phrase_bonus_max":5.0,"positive_bigrams":false,"result_limit_per_group":10});
    // Force a single bag whose unknown WordNet entry still ranks normally.
    let mut req = req;
    req["generation"]["hints"] = json!(["ate"]);
    let invoke = |reports: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_anagram-cli"));
        command
            .arg("solve")
            .arg(temp.0.join("dictionary"))
            .arg(temp.0.join("one"))
            .arg(temp.0.join("two"))
            .arg(&temp.0)
            .args(["--subtract", "xyz", "--cache"])
            .arg(&cache);
        if reports {
            command
                .args(["--hint-stats", "--benchmark-answer", "ate", "--export"])
                .arg(&export)
                .arg("--deep-export")
                .arg(&deep);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(req.to_string().as_bytes())
            .unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        serde_json::from_slice::<Value>(&result.stdout).unwrap()
    };
    let cold = invoke(true);
    let warm = invoke(false);
    assert_eq!(cold["normalized_input"], "ate");
    assert_eq!(warm["normalized_input"], "ate");
    assert_eq!(cold["buckets"], warm["buckets"]);
    assert_eq!(warm["status"]["cache"]["hit"], true);
    assert_eq!(cold["hint_stats"]["counts"]["ate"], 1);
    assert_eq!(
        cold["benchmark"]["matches"],
        json!([{"word_count":"1","rank":1}])
    );
    let filtered: Value = serde_json::from_slice(&fs::read(deep).unwrap()).unwrap();
    assert_eq!(filtered["buckets"], cold["buckets"]);
    assert_eq!(filtered["export_scope"], "deep_returned_rows");
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(export).unwrap()).unwrap(),
        cold
    );
}
