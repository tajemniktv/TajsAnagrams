use anagram_core::{
    contracts::DataRepresentation,
    control::Control,
    corpus_ranking::PhraseCorpus,
    phrase_index::PhraseIndex,
    provenance::{CorpusReadBudget, Snapshot},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
};

#[test]
fn learned_model_identity_tracks_loaded_bytes_without_changing_model_format() {
    use anagram_core::learned::Model;
    let model = Model::new([0.25; 18]).unwrap();
    assert!(model.identity().is_none());
    let bytes = serde_json::to_vec(&model).unwrap();
    let scratch = Scratch::new("model");
    let path = scratch.0.join("model.json");
    fs::write(&path, &bytes).unwrap();
    let loaded = Model::load(&path).unwrap();
    fs::write(&path, b"invalid replacement").unwrap();
    let identity = loaded.identity().unwrap();
    assert_eq!(identity.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(identity.bytes, bytes.len() as u64);
    assert_eq!(identity.role, "learned_model");
    assert_eq!(serde_json::to_vec(&loaded).unwrap(), bytes);
    assert_eq!(loaded.score(&[1.0; 18]).unwrap(), 4.5);
    let items = vec![anagram_core::learned::Item {
        key: "first".into(),
        features: [1.0; 18],
        baseline_score: 2.0,
    }];
    let report =
        anagram_core::learned::rank_result(&items, Some(&loaded), &Control::default()).unwrap();
    assert_eq!(report.model_identity.as_ref(), Some(identity));
    assert_eq!(report.indices, vec![0]);
    assert_eq!(report.model_schema, Some(anagram_core::learned::SCHEMA));
    let baseline = anagram_core::learned::rank_result(&items, None, &Control::default()).unwrap();
    assert!(baseline.model_identity.is_none() && baseline.model_schema.is_none());
    let cancelled = Control::default();
    cancelled.cancel();
    let error = Model::load_controlled(&scratch.0.join("missing"), &cancelled).unwrap_err();
    assert_eq!(error.to_string(), "cancelled");
    fs::write(&path, vec![b' '; 65537]).unwrap();
    let error = Model::load_controlled(&path, &Control::default()).unwrap_err();
    assert_eq!(error.to_string(), "ranker model exceeds 64 KiB");
    assert!(anagram_core::learned::rank_result(&items, Some(&loaded), &cancelled).is_err());
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.codex/temp");
        fs::create_dir_all(&root).unwrap();
        let path = root
            .canonicalize()
            .unwrap()
            .join(format!("provenance-{}-{name}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn file_identity_hashes_the_bytes_retained_for_parsing() {
    let scratch = Scratch::new("file");
    let path = scratch.0.join("data");
    fs::write(&path, b"abc").unwrap();
    let snapshot = Snapshot::load(&path, "dictionary", &Control::default()).unwrap();
    fs::write(&path, b"changed").unwrap();
    assert_eq!(snapshot.bytes(), b"abc");
    assert_eq!(
        snapshot.identity().sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(snapshot.identity().bytes, 3);
    assert!(snapshot.identity().present);
}

#[test]
fn snapshot_read_budget_accepts_exact_limit_and_rejects_limit_plus_one() {
    let at_limit = Scratch::new("budget-at-limit");
    let path = at_limit.0.join("data");
    fs::write(&path, b"abc").unwrap();
    let mut budget = CorpusReadBudget::new(Some(3), Some(3));
    let snapshot =
        Snapshot::load_with_budget(&path, "dictionary", &Control::default(), &mut budget).unwrap();
    assert_eq!(snapshot.bytes(), b"abc");
    assert_eq!(snapshot.identity().bytes, 3);

    let over_limit = Scratch::new("budget-over-limit");
    let path = over_limit.0.join("data");
    fs::write(&path, b"abcd").unwrap();
    let mut budget = CorpusReadBudget::new(Some(3), Some(10));
    let error = Snapshot::load_with_budget(&path, "dictionary", &Control::default(), &mut budget)
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::FileTooLarge);
    assert!(error.to_string().contains("per-role byte limit of 3"));
}

#[test]
fn snapshot_read_budget_enforces_cumulative_role_and_aggregate_bytes() {
    let scratch = Scratch::new("budget-cumulative");
    let first = scratch.0.join("first");
    let second = scratch.0.join("second");
    let third = scratch.0.join("third");
    fs::write(&first, b"abc").unwrap();
    fs::write(&second, b"de").unwrap();
    fs::write(&third, b"f").unwrap();

    let mut role_budget = CorpusReadBudget::new(Some(4), Some(20));
    Snapshot::load_with_budget(&first, "dictionary", &Control::default(), &mut role_budget)
        .unwrap();
    let error =
        Snapshot::load_with_budget(&second, "dictionary", &Control::default(), &mut role_budget)
            .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::FileTooLarge);
    assert!(error.to_string().contains("per-role byte limit of 4"));

    let mut aggregate_budget = CorpusReadBudget::new(Some(4), Some(5));
    Snapshot::load_with_budget(
        &first,
        "dictionary",
        &Control::default(),
        &mut aggregate_budget,
    )
    .unwrap();
    Snapshot::load_with_budget(
        &second,
        "bigrams",
        &Control::default(),
        &mut aggregate_budget,
    )
    .unwrap();
    let error = Snapshot::load_with_budget(
        &third,
        "wordnet/index.noun",
        &Control::default(),
        &mut aggregate_budget,
    )
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::FileTooLarge);
    assert!(error.to_string().contains("aggregate byte limit of 5"));
}

#[test]
#[cfg(unix)] // Windows set_len does not guarantee sparse allocation.
fn sparse_large_file_is_rejected_after_only_a_small_read() {
    let scratch = Scratch::new("sparse");
    let path = scratch.0.join("sparse");
    fs::File::create(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024 * 1024)
        .unwrap();
    let mut budget = CorpusReadBudget::new(Some(32), Some(32));
    let error = Snapshot::load_with_budget(&path, "dictionary", &Control::default(), &mut budget)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
}

#[test]
fn growing_file_is_limited_by_actual_reads_without_metadata_checks() {
    struct GrowingFile {
        file: fs::File,
        path: PathBuf,
        grew: bool,
    }
    impl Read for GrowingFile {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let n = self.file.read(buffer)?;
            if !self.grew {
                fs::OpenOptions::new()
                    .append(true)
                    .open(&self.path)?
                    .write_all(b"def")?;
                self.grew = true;
            }
            Ok(n)
        }
    }
    let scratch = Scratch::new("growing");
    let path = scratch.0.join("data");
    fs::write(&path, b"abc").unwrap();
    let file = GrowingFile {
        file: fs::File::open(&path).unwrap(),
        path,
        grew: false,
    };
    let control = Control::default();
    let mut budget = CorpusReadBudget::new(Some(4), Some(10));
    let mut reader = budget.reader(file, "dictionary", &control);
    let mut buffer = [0; 16];
    assert_eq!(reader.read(&mut buffer).unwrap(), 3);
    assert_eq!(
        reader.read(&mut buffer).unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
    assert_eq!(
        reader.read(&mut buffer).unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
}

#[test]
fn cancellation_during_eof_or_overflow_wins_over_success_or_limit_errors() {
    struct CancelRead {
        control: Control,
        bytes: usize,
    }
    impl Read for CancelRead {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.control.cancel();
            let n = self.bytes.min(buffer.len());
            buffer[..n].fill(b'x');
            Ok(n)
        }
    }
    for bytes in [0, 4] {
        let control = Control::default();
        let mut budget = CorpusReadBudget::new(Some(3), Some(3));
        let mut reader = budget.reader(
            CancelRead {
                control: control.clone(),
                bytes,
            },
            "dictionary",
            &control,
        );
        let error = reader.read(&mut [0; 16]).unwrap_err();
        assert_eq!(error.to_string(), "cancelled");
    }
}

#[test]
fn failed_reader_preserves_consumption_when_budget_is_reused() {
    let control = Control::default();
    let mut budget = CorpusReadBudget::new(Some(3), Some(3));
    let error = budget
        .reader(b"abcd".as_slice(), "dictionary", &control)
        .read(&mut [0; 16])
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
    let error = budget
        .reader(b"x".as_slice(), "unigrams", &control)
        .read(&mut [0; 16])
        .unwrap_err();
    assert!(error.to_string().contains("aggregate byte limit of 3"));
}

#[test]
fn format_cap_is_per_file_while_role_budget_is_cumulative() {
    let scratch = Scratch::new("file-cap");
    let path = scratch.0.join("data");
    fs::write(&path, b"abc").unwrap();
    let control = Control::default();
    let mut budget = CorpusReadBudget::new(Some(6), Some(6));
    for _ in 0..2 {
        Snapshot::load_with_budget_and_file_limit(
            &path,
            "learned_model",
            &control,
            &mut budget,
            Some(3),
        )
        .unwrap();
    }
    assert_eq!(
        Snapshot::load_with_budget_and_file_limit(
            &path,
            "learned_model",
            &control,
            &mut budget,
            Some(3)
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::FileTooLarge
    );
}

#[test]
fn independent_concurrent_intakes_do_not_share_global_counters() {
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let control = Control::default();
                    let mut budget = CorpusReadBudget::new(Some(3), Some(3));
                    let mut data = Vec::new();
                    budget
                        .reader(b"abc".as_slice(), "dictionary", &control)
                        .read_to_end(&mut data)
                        .unwrap();
                    assert_eq!(data, b"abc");
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

#[test]
fn wordnet_and_models_consume_the_same_aggregate_budget() {
    use anagram_core::{learned::Model, wordnet::WordNet};
    let scratch = Scratch::new("all-roles");
    for name in [
        "index.noun",
        "index.verb",
        "index.adj",
        "index.adv",
        "noun.exc",
        "verb.exc",
        "data.verb",
    ] {
        fs::write(scratch.0.join(name), b" ").unwrap();
    }
    let control = Control::default();
    let mut budget = CorpusReadBudget::new(Some(1), Some(6));
    let error = WordNet::load_identified_with_budget(&scratch.0, &control, &mut budget)
        .err()
        .unwrap();
    assert_eq!(error.kind(), io::ErrorKind::FileTooLarge);
    assert!(error.to_string().contains("aggregate"));
    let model = scratch.0.join("model.json");
    let data = serde_json::to_vec(&Model::new([0.0; 18]).unwrap()).unwrap();
    fs::write(&model, &data).unwrap();
    let mut budget = CorpusReadBudget::new(None, Some(data.len() + 1));
    Snapshot::load_with_budget(
        &scratch.0.join("index.noun"),
        "dictionary",
        &control,
        &mut budget,
    )
    .unwrap();
    Model::load_controlled_with_budget(&model, &control, &mut budget).unwrap();
    assert_eq!(
        Snapshot::load_with_budget(
            &scratch.0.join("index.verb"),
            "bigrams",
            &control,
            &mut budget
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::FileTooLarge
    );
}

#[test]
fn phrase_budget_counts_escaped_rows_and_accepts_exact_canonical_size() {
    let scratch = Scratch::new("phrase-escaped");
    let path = scratch.0.join("phrases.sqlite");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer
        .execute_batch("CREATE TABLE ngrams(text TEXT PRIMARY KEY,n INTEGER,count INTEGER)")
        .unwrap();
    writer
        .execute("INSERT INTO ngrams VALUES(?1,2,1)", ["\"\\\n".repeat(100)])
        .unwrap();
    let index = PhraseIndex::open(&path).unwrap();
    let identity = index.identity().unwrap();
    let n = identity.bytes as usize;
    let mut exact = CorpusReadBudget::new(Some(n), Some(n));
    assert_eq!(index.identity_with_budget(&mut exact).unwrap(), identity);
    let mut small = CorpusReadBudget::new(Some(n - 1), Some(n));
    assert_eq!(
        index.identity_with_budget(&mut small).unwrap_err().kind(),
        io::ErrorKind::FileTooLarge
    );
    let mut schema_small = CorpusReadBudget::new(Some(1), None);
    assert_eq!(
        index
            .identity_with_budget(&mut schema_small)
            .unwrap_err()
            .kind(),
        io::ErrorKind::FileTooLarge
    );
}

#[test]
fn phrase_lookup_rejects_duplicates_and_accepts_identifier_case() {
    let scratch = Scratch::new("case-duplicates");
    let path = scratch.0.join("phrases.sqlite");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("CREATE TABLE NGrams(text TEXT,n INTEGER,count INTEGER); INSERT INTO NGrams VALUES('a b',2,1);").unwrap();
    {
        let index = PhraseIndex::open(&path).unwrap();
        assert!(index.identity().is_ok());
    }
    writer
        .execute_batch("INSERT INTO NGrams VALUES('a b',2,99)")
        .unwrap();
    let index = PhraseIndex::open(&path).unwrap();
    assert!(index.counts(&["a b".into()]).is_err());
}

#[test]
fn phrase_identity_uses_the_shared_read_budget() {
    let scratch = Scratch::new("phrase-budget");
    let path = scratch.0.join("phrases.sqlite");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("CREATE TABLE ngrams(text TEXT PRIMARY KEY,n INTEGER,count INTEGER); INSERT INTO ngrams VALUES('a b',2,1);").unwrap();
    let index = PhraseIndex::open(&path).unwrap();
    let schema: String = writer
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE name='ngrams'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let header_bytes = serde_json::to_vec(&("schema", schema)).unwrap().len() + 1;
    let mut budget = CorpusReadBudget::new(None, Some(header_bytes + 1));
    let error = index.identity_with_budget(&mut budget).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::FileTooLarge);
    assert!(error.to_string().contains("aggregate byte limit"));
}

#[test]
fn sqlite_identity_and_queries_share_a_snapshot_during_wal_writes() {
    let scratch = Scratch::new("sqlite");
    let path = scratch.0.join("phrases.sqlite");
    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE ngrams(text TEXT PRIMARY KEY,n INTEGER,count INTEGER); INSERT INTO ngrams VALUES ('a b',2,10);").unwrap();
    let index = PhraseIndex::open(&path).unwrap();
    let identity = index.identity().unwrap();
    assert_eq!(identity.representation, DataRepresentation::PhraseRowsV1);
    let mut canonical = serde_json::to_vec(&(
        "schema",
        "CREATE TABLE ngrams(text TEXT PRIMARY KEY,n INTEGER,count INTEGER)",
    ))
    .unwrap();
    canonical.extend_from_slice(b"\n[\"a b\",2,10]\n");
    assert_eq!(identity.sha256, format!("{:x}", Sha256::digest(&canonical)));
    assert_eq!(identity.bytes, canonical.len() as u64);
    writer.execute("UPDATE ngrams SET count=99", []).unwrap();
    assert_eq!(index.counts(&["a b".to_owned()]).unwrap()["a b"], 10);
    assert_eq!(index.identity().unwrap(), identity);
    let fresh = PhraseIndex::open(&path).unwrap();
    assert_eq!(fresh.counts(&["a b".to_owned()]).unwrap()["a b"], 99);
    assert_ne!(fresh.identity().unwrap(), identity);
}
