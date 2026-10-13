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
