use anagram_core::{control::Control, policy::DeploymentLimits};
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn scratch() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.codex/temp");
    fs::create_dir_all(&root).unwrap();
    tempfile::Builder::new()
        .prefix("budget-")
        .tempdir_in(root)
        .unwrap()
}

fn generate(dictionary: &Path, unigrams: Option<&Path>, policy: &Path) -> (bool, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_anagram-cli"));
    command.arg("generate").arg(dictionary);
    if let Some(path) = unigrams {
        command.arg(path);
    }
    let mut child = command
        .arg("--limits")
        .arg(policy)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request = json!({"schema_version":1,"text":"ate","min_words":1,"max_words":1,"min_word_length":1,"max_word_length":20,"min_zipf":0,"candidate_budget":1,"allow_repeat":true,"strategy":"prefix","hint_mode":"any"});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let result = child.wait_with_output().unwrap();
    (
        result.status.success(),
        serde_json::from_slice(&result.stdout).unwrap(),
    )
}

#[test]
fn generation_enforces_exact_per_role_and_aggregate_intake_limits() {
    let temp = scratch();
    let dictionary = temp.path().join("dictionary");
    let unigrams = temp.path().join("unigrams");
    let policy = temp.path().join("limits.json");
    fs::write(&dictionary, b"ate\n").unwrap();
    fs::write(&unigrams, b"ate\t10\n").unwrap();
    for (role, total, success) in [(7, 11, true), (3, 11, false), (7, 10, false)] {
        fs::write(
            &policy,
            json!({"max_corpus_role_bytes":role,"max_total_corpus_bytes":total}).to_string(),
        )
        .unwrap();
        let (ok, result) = generate(&dictionary, Some(&unigrams), &policy);
        assert_eq!(ok, success, "{result}");
        if !ok {
            assert_eq!(result["error"]["code"], "corpus_too_large");
        }
    }
    fs::write(
        &policy,
        r#"{"max_corpus_role_bytes":null,"max_total_corpus_bytes":null}"#,
    )
    .unwrap();
    assert!(generate(&dictionary, Some(&unigrams), &policy).0);
}

#[test]
fn generation_rejects_long_line_before_unbounded_line_allocation() {
    let temp = scratch();
    let dictionary = temp.path().join("dictionary");
    fs::write(&dictionary, vec![b'a'; 128 * 1024]).unwrap();
    let policy = temp.path().join("limits.json");
    fs::write(
        &policy,
        r#"{"max_corpus_role_bytes":32,"max_total_corpus_bytes":32}"#,
    )
    .unwrap();
    let (ok, result) = generate(&dictionary, None, &policy);
    assert!(!ok);
    assert_eq!(result["error"]["code"], "corpus_too_large");
}

#[test]
fn dictionary_preparation_honors_one_shared_budget_and_never_publishes_failure() {
    let temp = scratch();
    let dictionary = temp.path().join("dictionary");
    let unigrams = temp.path().join("unigrams");
    let output = temp.path().join("output");
    fs::write(&dictionary, b"ate\n").unwrap();
    fs::write(&unigrams, b"ate\t10\n").unwrap();
    let args: Vec<OsString> = vec![
        "prepare-dictionary".into(),
        dictionary.into_os_string(),
        unigrams.into_os_string(),
        output.clone().into_os_string(),
    ];
    for (role, total) in [(3, 11), (7, 10)] {
        let limits = DeploymentLimits {
            max_corpus_role_bytes: Some(role),
            max_total_corpus_bytes: Some(total),
            ..Default::default()
        };
        let error = anagram_cli::utilities::run_with_input_and_limits(
            &args,
            &Control::default(),
            &mut std::io::empty(),
            &limits,
        )
        .unwrap_err();
        assert_eq!(error.code, "corpus_too_large");
        assert!(!output.exists());
    }
    let limits = DeploymentLimits {
        max_corpus_role_bytes: Some(7),
        max_total_corpus_bytes: Some(11),
        ..Default::default()
    };
    anagram_cli::utilities::run_with_input_and_limits(
        &args,
        &Control::default(),
        &mut std::io::empty(),
        &limits,
    )
    .unwrap();
    assert!(output.exists());
}

#[test]
fn phrase_builder_limits_decompressed_bytes_and_aggregate_sources() {
    use anagram_core::provenance::CorpusReadBudget;
    let temp = scratch();
    let compressed = temp.path().join("titles.gz");
    let mut gzip = flate2::write::GzEncoder::new(
        fs::File::create(&compressed).unwrap(),
        flate2::Compression::default(),
    );
    gzip.write_all("a_b\n".repeat(1024).as_bytes()).unwrap();
    gzip.finish().unwrap();
    assert!(fs::metadata(&compressed).unwrap().len() < 128);
    let output = temp.path().join("failed.db");
    let mut budget = CorpusReadBudget::new(Some(128), Some(256));
    let error = anagram_cli::phrase_builder::build_with_budget(
        &output,
        &[compressed],
        2,
        &Control::default(),
        &mut budget,
    )
    .unwrap_err();
    assert_eq!(error.code, "corpus_too_large");
    assert!(!output.exists());
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    fs::write(&first, b"a_b\n").unwrap();
    fs::write(&second, b"a_b\n").unwrap();
    let mut budget = CorpusReadBudget::new(Some(4), Some(7));
    let error = anagram_cli::phrase_builder::build_with_budget(
        &output,
        &[first.clone(), second.clone()],
        2,
        &Control::default(),
        &mut budget,
    )
    .unwrap_err();
    assert_eq!(error.code, "corpus_too_large");
    assert!(!output.exists());
    let mut exact = CorpusReadBudget::new(Some(4), Some(8));
    anagram_cli::phrase_builder::build_with_budget(
        &output,
        &[first, second],
        2,
        &Control::default(),
        &mut exact,
    )
    .unwrap();
    assert!(output.exists());
}

#[test]
fn ranker_build_budgets_wordnet_and_phrase_identity_together() {
    let temp = scratch();
    for name in ["index.noun", "index.verb", "index.adj", "index.adv"] {
        fs::write(temp.path().join(name), b" ").unwrap();
    }
    let phrase = temp.path().join("phrases.db");
    rusqlite::Connection::open(&phrase).unwrap().execute_batch("CREATE TABLE ngrams(text TEXT PRIMARY KEY,n INTEGER,count INTEGER); INSERT INTO ngrams VALUES('we are home',3,1)").unwrap();
    let args: Vec<OsString> = vec![
        "ranker-build".into(),
        temp.path().into(),
        phrase.into_os_string(),
    ];
    let limits = DeploymentLimits {
        max_corpus_role_bytes: None,
        max_total_corpus_bytes: Some(4),
        ..Default::default()
    };
    let error = anagram_cli::utilities::run_with_input_and_limits(
        &args,
        &Control::default(),
        &mut b"[{\"answer\":\"we are home\"}]".as_slice(),
        &limits,
    )
    .unwrap_err();
    assert_eq!(error.code, "corpus_too_large");
}
