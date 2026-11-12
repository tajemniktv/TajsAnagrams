use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

#[test]
fn real_child_failures_timeout_sampling_and_output_bounds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = root.join(".codex/temp/xtask/process-tests");
    fs::create_dir_all(&scratch).unwrap();
    let temp = tempfile::tempdir_in(scratch).unwrap();
    let cases = temp.path().join("cases.json");
    fs::write(&cases,json!({"schema_version":1,"cases":[{"id":"ab","source":"ab","split":"train","relevance":{"ab":3},"provenance":"Synthetic process-capture fixture"}]}).to_string()).unwrap();
    let dictionary = temp.path().join("dictionary");
    let unigrams = temp.path().join("unigrams");
    fs::write(&unigrams, "").unwrap();
    for (mode, expected) in [
        ("invalid", Some("invalid_response")),
        ("array", Some("invalid_response")),
        ("exit", Some("process_failed")),
        ("error", Some("timed_out")),
        ("sleep", Some("process_timeout")),
        ("large", Some("output_too_large")),
        ("success", None),
    ] {
        fs::write(&dictionary, mode).unwrap();
        let output = temp.path().join(format!("{mode}.jsonl"));
        let status = Command::new(env!("CARGO_BIN_EXE_xtask"))
            .current_dir(root)
            .args([
                "benchmark",
                "--cli",
                env!("CARGO_BIN_EXE_capture-fixture"),
                "--mode",
                "generate",
                "--cases",
            ])
            .arg(&cases)
            .arg("--dictionary")
            .arg(&dictionary)
            .arg("--unigrams")
            .arg(&unigrams)
            .args([
                "--guard-ms",
                if mode == "sleep" { "100" } else { "10000" },
                "--output",
            ])
            .arg(&output)
            .status()
            .unwrap();
        assert!(status.success(), "{mode}");
        let records = fs::read_to_string(output).unwrap();
        let v: Value = serde_json::from_str(records.trim()).unwrap();
        assert_eq!(v["error"]["code"].as_str(), expected, "{mode}: {v}");
        assert!(v["wall_ms"].as_f64().unwrap() >= 0.);
        if mode == "success" {
            assert_eq!(v["result"]["bags"], json!([["ab"]]));
            assert!(v["peak_rss_bytes"].as_u64().unwrap() > 0);
            assert!(v["rss_samples"].as_u64().unwrap() > 0);
        }
        if mode == "sleep" {
            assert!(v["wall_ms"].as_f64().unwrap() < 3000.);
        }
    }
}
#[test]
fn strategy_treatments_retain_exact_requests_and_manifest_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = root.join(".codex/temp/xtask/process-tests");
    fs::create_dir_all(&scratch).unwrap();
    let temp = tempfile::tempdir_in(scratch).unwrap();
    let cases = temp.path().join("cases.json");
    fs::write(&cases,json!({"schema_version":1,"cases":[{"id":"ab","source":"ab","split":"train","relevance":{"ab":3},"provenance":"Synthetic strategy-schedule fixture"}]}).to_string()).unwrap();
    let dictionary = temp.path().join("dictionary");
    fs::write(&dictionary, "success").unwrap();
    let unigrams = temp.path().join("unigrams");
    fs::write(&unigrams, "").unwrap();
    let output = temp.path().join("strategies.jsonl");
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(root)
        .args([
            "benchmark",
            "--cli",
            env!("CARGO_BIN_EXE_capture-fixture"),
            "--mode",
            "generate",
            "--strategies",
            "prefix,diverse",
            "--repeats",
            "2",
            "--cases",
        ])
        .arg(&cases)
        .arg("--dictionary")
        .arg(dictionary)
        .arg("--unigrams")
        .arg(unigrams)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let records: Vec<Value> = fs::read_to_string(&output)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(records.len(), 4);
    for strategy in ["prefix", "diverse"] {
        for trial in 0..2 {
            let matching: Vec<_> = records
                .iter()
                .filter(|r| r["request"]["strategy"] == strategy && r["trial"] == trial)
                .collect();
            assert_eq!(matching.len(), 1);
            assert!(matching[0]["error"].is_null());
        }
    }
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(output.with_extension("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["evaluation_profile"], "custom");
    assert_eq!(manifest["strategies"], json!(["prefix", "diverse"]));
    assert_eq!(
        manifest["inputs"]["cases"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
}
