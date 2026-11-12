//! Opt-in, local review evidence. No solver scores or judgments are training input.
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::Path,
};
use unicode_normalization::UnicodeNormalization;

type Result<T> = std::result::Result<T, String>;
const MAX_BYTES: usize = 32 * 1024 * 1024;
const REASONS: &[&str] = &[
    "unspecified",
    "grammar",
    "meaning",
    "incomplete_meaning",
    "semantic_anomaly",
    "unfamiliar",
    "unverified_name",
    "spelling_rendering",
    "contrived_context",
    "task_mismatch",
];
fn hash(value: &Value) -> String {
    hex::encode(Sha256::digest(value.to_string().as_bytes()))
}
fn now() -> Result<u128> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .map_err(|e| e.to_string())
}
fn db(data: &Path) -> Result<Connection> {
    fs::create_dir_all(data).map_err(|e| e.to_string())?;
    let mut c = Connection::open(data.join("reviews.sqlite")).map_err(|e| e.to_string())?;
    c.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS captures(id TEXT PRIMARY KEY, payload TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS judgments(id TEXT PRIMARY KEY, payload TEXT NOT NULL);
      CREATE TABLE IF NOT EXISTS preferences(id TEXT PRIMARY KEY, payload TEXT NOT NULL);",
    )
    .map_err(|e| e.to_string())?;
    if c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
        .map_err(|e| e.to_string())?
        < 1
    {
        let tx = c.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS review_rows(row_id TEXT NOT NULL,capture_id TEXT NOT NULL,context_id TEXT NOT NULL,judgeable INTEGER NOT NULL,PRIMARY KEY(row_id,capture_id));
          CREATE INDEX IF NOT EXISTS review_rows_context ON review_rows(context_id,row_id,judgeable);
          INSERT OR IGNORE INTO review_rows SELECT json_extract(r.value,'$.id'),c.id,json_extract(c.payload,'$.context_id'),json_extract(r.value,'$.judgeable') FROM captures c,json_each(c.payload,'$.rows') r;
          PRAGMA user_version=1;").map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(c)
}
fn bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("Review input exceeds 32 MiB".into());
    }
    Ok(bytes)
}
fn letters(s: &str) -> Vec<u8> {
    let mut b = s
        .nfkd()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase() as u8)
        .collect::<Vec<_>>();
    b.sort_unstable();
    b
}
pub fn capture(mut run: Value) -> Result<Value> {
    let request = run["request"].clone();
    let text = request["generation"]["text"]
        .as_str()
        .ok_or("Review requires a ranked solve request")?;
    let parsed: anagram_core::solve::Request =
        serde_json::from_value(request.clone()).map_err(|e| e.to_string())?;
    anagram_core::solve::validate(&parsed).map_err(|e| e.message)?;
    let case = run["case_id"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| hash(&json!(request["generation"])));
    let family = run["source_family"].as_str().map(str::to_owned);
    let context = json!({"rubric":"standalone-usefulness-v1", "case_id":case, "generation":request["generation"], "source_context":run["source_context"]});
    let context_id = hash(&context);
    let mut rows = Vec::new();
    if let Some(buckets) = run["result"]["buckets"].as_object() {
        let mut groups = buckets
            .iter()
            .map(|(k, v)| {
                Ok((
                    k.parse::<usize>().map_err(|e| e.to_string())?,
                    v.as_array().ok_or("Invalid bucket")?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        groups.sort_by_key(|(n, _)| *n);
        for (count, bucket) in groups {
            for (position, row) in bucket.iter().enumerate() {
                let words = row["words"]
                    .as_array()
                    .ok_or("Review row lacks words")?
                    .iter()
                    .map(|w| w.as_str().ok_or("Invalid word"))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                if words.len() != count || letters(&words.join("")) != letters(text) {
                    return Err("Review row inventory/count mismatch".into());
                }
                let fallback = row["best_order"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let phrase = row["display_phrase"]
                    .as_str()
                    .filter(|p| !p.is_empty())
                    .unwrap_or(&fallback);
                if !phrase.is_empty() && letters(phrase) != letters(text) {
                    return Err("Rendered phrase inventory mismatch".into());
                }
                let key = phrase
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                rows.push(json!({"id":hash(&json!([context_id,key])),"phrase":phrase,"word_count":count,"rank":position+1,"score":row["final"],"judgeable":!phrase.is_empty()}));
                if rows.len() > 10000 {
                    return Err("Too many review rows".into());
                }
            }
        }
    } else if run.get("error").is_none_or(Value::is_null) {
        return Err(
            "Ranked result or explicit failure required; unranked word bags are not phrases".into(),
        );
    }
    // Keep the original request/status/identities/timings, never current settings.
    run["rows"] = json!(rows);
    run["case_id"] = json!(case);
    run["source_family"] = json!(family);
    run["context_id"] = json!(context_id);
    run["input"] = json!(text);
    run["rubric"] = json!("standalone-usefulness-v1");
    run["id"] = json!(hash(&run));
    if run.to_string().len() > MAX_BYTES {
        return Err("Capture exceeds 32 MiB".into());
    }
    Ok(run)
}
pub fn store(data: &Path, runs: &[Value]) -> Result<usize> {
    let captures = runs
        .iter()
        .cloned()
        .map(capture)
        .collect::<Result<Vec<_>>>()?;
    let mut c = db(data)?;
    let tx = c.transaction().map_err(|e| e.to_string())?;
    let mut added = 0;
    for r in captures {
        added += tx
            .execute(
                "INSERT OR IGNORE INTO captures VALUES(?1,?2)",
                params![r["id"].as_str(), r.to_string()],
            )
            .map_err(|e| e.to_string())?;
        for row in r["rows"].as_array().ok_or("Missing capture rows")? {
            tx.execute(
                "INSERT OR IGNORE INTO review_rows VALUES(?1,?2,?3,?4)",
                params![
                    row["id"].as_str(),
                    r["id"].as_str(),
                    r["context_id"].as_str(),
                    row["judgeable"].as_bool()
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(added)
}
pub fn import(data: &Path, path: &Path, workloads: Option<&Path>) -> Result<usize> {
    let bytes = bounded(path)?;
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let mut runs = Vec::new();
    let mut seen_lines = 0;
    fn source_identity(run: &Value) -> String {
        let mut original = run.clone();
        if let Some(fields) = original.as_object_mut() {
            for name in [
                "id",
                "rows",
                "context_id",
                "input",
                "rubric",
                "source_context",
                "source_family",
                "task_mapping",
            ] {
                fields.remove(name);
            }
        }
        hash(&original)
    }
    // Automatic metadata discovery must not fork already-reviewed captures into
    // fresh row IDs. Preserve existing source/context and its human judgments.
    let existing = if workloads.is_none() {
        read(data)?["captures"]
            .as_array()
            .unwrap()
            .iter()
            .map(source_identity)
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let metadata: Option<Value> = workloads
        .map(|p| bounded(p).and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string())))
        .transpose()?;
    // Quick import needs only the capture chooser. A sibling workload file may
    // supply context, but only per-case exact request matches may be attached.
    // Explicit metadata stays fail-closed; absent/malformed automatic metadata
    // never invents a source family or prevents importing the capture itself.
    let automatic = if workloads.is_none() {
        path.parent()
            .map(|p| p.join("workloads.json"))
            .and_then(|p| bounded(&p).ok())
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .filter(|v| v["schema_version"] == 1 && v["workloads"].is_array())
    } else {
        None
    };
    if metadata
        .as_ref()
        .is_some_and(|v| v["schema_version"] != 1 || !v["workloads"].is_array())
    {
        return Err("Invalid workload metadata".into());
    }
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        seen_lines += 1;
        if seen_lines > 1000 {
            return Err("Import limited to 1000 attempts".into());
        }
        let mut r: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
        if r["mode"] == "generate" || r["request"]["generation"].is_null() {
            return Err("Import ranked solve JSONL, not generation or stage captures".into());
        }
        r["origin"] =
            json!("imported native capture; source-family/context metadata may be absent");
        let explicit_context = !r["source_context"].is_null()
            || !r["source_family"].is_null()
            || !r["task_mapping"].is_null();
        if !explicit_context && existing.contains(&source_identity(&r)) {
            continue;
        }
        if let Some(metadata) = &metadata {
            let w = metadata["workloads"]
                .as_array()
                .unwrap()
                .iter()
                .find(|w| w["id"] == r["case_id"])
                .ok_or("Capture case missing from workloads")?;
            if !anagram_core::solve::same_request(&w["request"], &r["request"]) {
                return Err("Workload/capture request mismatch".into());
            }
            r["source_context"] = w["source_context"].clone();
            r["source_family"] = w["source_evidence"]["family_id"].clone();
            r["task_mapping"] = w["mapping"].clone();
        } else if let Some(w) = automatic
            .as_ref()
            .filter(|_| !explicit_context)
            .and_then(|m| m["workloads"].as_array())
            .and_then(|ws| {
                ws.iter().find(|w| {
                    w["id"] == r["case_id"]
                        && anagram_core::solve::same_request(&w["request"], &r["request"])
                })
            })
        {
            r["source_context"] = w["source_context"].clone();
            r["source_family"] = w["source_evidence"]["family_id"].clone();
            r["task_mapping"] = w["mapping"].clone();
        }
        runs.push(r);
    }
    if seen_lines == 0 {
        return Err("Empty capture file".into());
    }
    if runs.is_empty() {
        return Ok(0);
    }
    store(data, &runs)
}
pub fn read(data: &Path) -> Result<Value> {
    if !data.join("reviews.sqlite").exists() {
        return Ok(json!({"schema_version":1,"captures":[],"judgments":[],"preferences":[]}));
    }
    let mut c = db(data)?;
    // Commands run concurrently: captures, judgments and preferences must be
    // exported from one database snapshot, never three different moments.
    let snapshot = c.transaction().map_err(|e| e.to_string())?;
    let mut result = json!({"schema_version":1,"rubric":"standalone-usefulness-v1","clustering":"Repeated rows/trials are not independent puzzles. Compare by case/source family; unknown family is not inferred.","storage":data.join("reviews.sqlite")});
    for table in ["captures", "judgments", "preferences"] {
        let mut stmt = snapshot
            .prepare(&format!("SELECT payload FROM {table} ORDER BY rowid"))
            .map_err(|e| e.to_string())?;
        let values = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .map(|r| {
                r.map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        result[table] = json!(values);
    }
    snapshot.commit().map_err(|e| e.to_string())?;
    Ok(result)
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Judgment {
    pub id: String,
    pub verdict: Option<String>,
    pub reasons: Vec<String>,
    pub note: String,
    pub task_fit: Option<String>,
    pub looked_up: bool,
}
fn verdict(v: Option<&str>) -> bool {
    matches!(v, None | Some("useful" | "not_useful" | "unsure"))
}
pub fn judge(data: &Path, j: Judgment) -> Result<()> {
    if !verdict(j.verdict.as_deref())
        || !verdict(j.task_fit.as_deref())
        || j.note.len() > 2000
        || j.reasons.len() > REASONS.len()
        || j.reasons.iter().any(|r| !REASONS.contains(&r.as_str()))
    {
        return Err("Invalid judgment, reason or note (maximum 2000 bytes)".into());
    }
    let c = db(data)?;
    if !c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM review_rows WHERE row_id=?1 AND judgeable=1)",
            [&j.id],
            |r| r.get::<_, bool>(0),
        )
        .map_err(|e| e.to_string())?
    {
        return Err("Unknown or blank output; judgments cannot create results".into());
    }
    let value = json!({"id":j.id,"verdict":j.verdict,"reasons":j.reasons,"note":j.note,"task_fit":j.task_fit,"looked_up":j.looked_up,"recorded_at_unix_ms":now()?,"provenance":"Personal desktop review; ranks visible; not population-wide or necessarily blinded","rubric":"standalone-usefulness-v1"});
    db(data)?.execute("INSERT INTO judgments VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![value["id"].as_str(),value.to_string()]).map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preference {
    pub capture_id: String,
    pub left: String,
    pub right: String,
    pub choice: String,
}
pub fn prefer(data: &Path, p: Preference) -> Result<()> {
    if p.left == p.right
        || !["left", "right", "tie", "both_bad", "unsure"].contains(&p.choice.as_str())
    {
        return Err("Invalid pairwise preference".into());
    }
    let c = db(data)?;
    let context: String = c
        .query_row(
            "SELECT json_extract(payload,'$.context_id') FROM captures WHERE id=?1",
            [&p.capture_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or("Unknown capture")?;
    let contains = |id: &str| {
        c.query_row("SELECT EXISTS(SELECT 1 FROM review_rows WHERE context_id=?1 AND row_id=?2 AND judgeable=1)",params![context,id],|r|r.get::<_,bool>(0)).map_err(|e|e.to_string())
    };
    if !contains(&p.left)? || !contains(&p.right)? {
        return Err("Pair must belong to the same puzzle/context".into());
    }
    let mut pair = [p.left.clone(), p.right.clone()];
    pair.sort();
    let id = hash(&json!([context, pair]));
    let value = json!({"id":id,"capture_id":p.capture_id,"context_id":context,"left":p.left,"right":p.right,"choice":p.choice,"recorded_at_unix_ms":now()?,"presentation":"Explicit randomized left/right; actual identities retained","provenance":"Personal pairwise preference; separate from standalone verdicts"});
    db(data)?.execute("INSERT INTO preferences VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",params![id,value.to_string()]).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn export(data: &Path, path: &Path) -> Result<()> {
    let parent = path.parent().ok_or("Export needs a directory")?;
    let mut f = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut f, &read(data)?).map_err(|e| e.to_string())?;
    f.flush().map_err(|e| e.to_string())?;
    f.as_file().sync_all().map_err(|e| e.to_string())?;
    f.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scratch() -> tempfile::TempDir {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.codex/temp");
        fs::create_dir_all(&p).unwrap();
        tempfile::tempdir_in(p).unwrap()
    }
    fn run() -> Value {
        let mut request = crate::settings::Settings::default().request;
        request["generation"]["text"] = json!("listen");
        json!({"case_id":"six-letters","source_family":"family-one","request":request,"result":{"buckets":{"1":[{"words":["listen"],"best_order":["listen"],"display_phrase":"listen","final":5.0},{"words":["silent"],"best_order":["silent"],"display_phrase":"silent","final":9.0}]}}})
    }
    fn judgment(id: &str, verdict: Option<&str>) -> Judgment {
        Judgment {
            id: id.into(),
            verdict: verdict.map(str::to_owned),
            reasons: vec!["unfamiliar".into()],
            note: "Personal uncertainty; not a negative".into(),
            task_fit: None,
            looked_up: false,
        }
    }
    #[test]
    fn old_review_database_migrates_and_writes_use_only_indexed_rows() {
        let temp = scratch();
        let capture = capture(run()).unwrap();
        let c = Connection::open(temp.path().join("reviews.sqlite")).unwrap();
        c.execute_batch("CREATE TABLE captures(id TEXT PRIMARY KEY,payload TEXT NOT NULL);CREATE TABLE judgments(id TEXT PRIMARY KEY,payload TEXT NOT NULL);CREATE TABLE preferences(id TEXT PRIMARY KEY,payload TEXT NOT NULL);").unwrap();
        c.execute(
            "INSERT INTO captures VALUES(?1,?2)",
            params![capture["id"].as_str(), capture.to_string()],
        )
        .unwrap();
        drop(c);
        let row = capture["rows"][0]["id"].as_str().unwrap();
        judge(temp.path(), judgment(row, Some("useful"))).unwrap();
        let c = db(temp.path()).unwrap();
        assert_eq!(
            c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        // An unrelated historical payload must not be parsed to validate this ID.
        c.execute(
            "INSERT INTO captures VALUES('unrelated','invalid historical payload')",
            [],
        )
        .unwrap();
        drop(c);
        judge(temp.path(), judgment(row, Some("unsure"))).unwrap();
        prefer(
            temp.path(),
            Preference {
                capture_id: capture["id"].as_str().unwrap().into(),
                left: row.into(),
                right: capture["rows"][1]["id"].as_str().unwrap().into(),
                choice: "tie".into(),
            },
        )
        .unwrap();
    }
    #[test]
    fn ranks_context_uncertainty_notes_and_export_survive_reopen() {
        let temp = scratch();
        assert_eq!(read(temp.path()).unwrap()["captures"], json!([]));
        assert!(!temp.path().join("reviews.sqlite").exists());
        assert_eq!(store(temp.path(), &[run()]).unwrap(), 1);
        assert_eq!(store(temp.path(), &[run()]).unwrap(), 0);
        let state = read(temp.path()).unwrap();
        let rows = &state["captures"][0]["rows"];
        assert_eq!(rows[0]["rank"], 1);
        assert_eq!(rows[1]["rank"], 2);
        assert_eq!(rows[0]["score"], 5.0); // Do not re-sort by score.
        let id = rows[0]["id"].as_str().unwrap();
        judge(temp.path(), judgment(id, Some("unsure"))).unwrap();
        let saved = read(temp.path()).unwrap();
        assert_eq!(saved["judgments"][0]["verdict"], "unsure");
        assert!(saved["judgments"][0]["task_fit"].is_null());
        assert!(
            saved["judgments"][0]["note"]
                .as_str()
                .unwrap()
                .contains("uncertainty")
        );
        let mut altered = run();
        altered["request"]["generation"]["excluded"] = json!(["tinsel"]);
        store(temp.path(), &[altered]).unwrap();
        assert_ne!(
            read(temp.path()).unwrap()["captures"][1]["rows"][0]["id"],
            id
        );
        let path = temp.path().join("backup.json");
        export(temp.path(), &path).unwrap();
        let exported: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(exported["judgments"], saved["judgments"]);
        assert!(judge(temp.path(), judgment("made-up", Some("useful"))).is_err());
        assert!(judge(temp.path(), judgment(id, Some("mid"))).is_err());
    }
    #[test]
    fn pairs_are_separate_allow_both_bad_and_reject_cross_context() {
        let temp = scratch();
        store(temp.path(), &[run()]).unwrap();
        let state = read(temp.path()).unwrap();
        let c = &state["captures"][0];
        let p = Preference {
            capture_id: c["id"].as_str().unwrap().into(),
            left: c["rows"][0]["id"].as_str().unwrap().into(),
            right: c["rows"][1]["id"].as_str().unwrap().into(),
            choice: "both_bad".into(),
        };
        prefer(temp.path(), p).unwrap();
        let saved = read(temp.path()).unwrap();
        assert_eq!(saved["preferences"][0]["choice"], "both_bad");
        assert_eq!(saved["judgments"], json!([]));
        assert!(
            prefer(
                temp.path(),
                Preference {
                    capture_id: c["id"].as_str().unwrap().into(),
                    left: c["rows"][0]["id"].as_str().unwrap().into(),
                    right: "other-context".into(),
                    choice: "left".into()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn malformed_import_is_atomic_failures_and_blank_rows_are_retained() {
        let temp = scratch();
        let input = temp.path().join("runs.jsonl");
        let mut broken = run();
        broken["result"]["buckets"]["1"][0]["words"] = json!(["invalid"]);
        fs::write(&input, format!("{}\n{}", run(), broken)).unwrap();
        assert!(import(temp.path(), &input, None).is_err());
        assert_eq!(read(temp.path()).unwrap()["captures"], json!([]));
        let mut failed = run();
        failed["result"] = Value::Null;
        failed["error"] = json!("timed_out");
        store(temp.path(), &[failed]).unwrap();
        assert_eq!(read(temp.path()).unwrap()["captures"][0]["rows"], json!([]));
        let mut blank = run();
        blank["result"]["buckets"]["1"][0]["best_order"] = json!([]);
        blank["result"]["buckets"]["1"][0]["display_phrase"] = json!("");
        store(temp.path(), &[blank]).unwrap();
        let c = read(temp.path()).unwrap();
        let row = &c["captures"][1]["rows"][0];
        assert_eq!(row["judgeable"], false);
        assert!(
            judge(
                temp.path(),
                judgment(row["id"].as_str().unwrap(), Some("useful"))
            )
            .is_err()
        );
    }
    #[test]
    fn workload_mapping_is_checked_and_family_is_preserved() {
        let temp = scratch();
        let input = temp.path().join("runs.jsonl");
        let context = temp.path().join("workloads.json");
        fs::write(&input, run().to_string()).unwrap();
        let mut metadata = json!({"schema_version":1,"workloads":[{"id":"six-letters","request":run()["request"],"source_evidence":{"family_id":"public-source"},"source_context":{"original_context":"Real task"}}]});
        fs::write(&context, metadata.to_string()).unwrap();
        assert_eq!(import(temp.path(), &input, Some(&context)).unwrap(), 1);
        assert_eq!(
            read(temp.path()).unwrap()["captures"][0]["source_family"],
            "public-source"
        );
        metadata["workloads"][0]["request"]["generation"]["text"] = json!("different");
        fs::write(&context, metadata.to_string()).unwrap();
        assert!(import(temp.path(), &input, Some(&context)).is_err());
        assert_eq!(
            read(temp.path()).unwrap()["captures"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn workload_numbers_from_other_json_serializers_keep_context_and_existing_votes() {
        let temp = scratch();
        let input = temp.path().join("runs.jsonl");
        let context = temp.path().join("workloads.json");
        let mut source = run();
        source.as_object_mut().unwrap().remove("source_family");
        source["request"]["phrase_bonus_max"] = json!(6.0);
        fs::write(&input, source.to_string()).unwrap();
        let target = temp.path().join("store");
        import(&target, &input, None).unwrap();
        let row = read(&target).unwrap()["captures"][0]["rows"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        judge(&target, judgment(&row, Some("unsure"))).unwrap();
        let mut equivalent = source["request"].clone();
        equivalent["phrase_bonus_max"] = json!(6);
        let metadata = json!({"schema_version":1,"workloads":[{"id":"six-letters","request":equivalent,"source_evidence":{"family_id":"number-format-family"}}]});
        fs::write(&context, metadata.to_string()).unwrap();
        let fresh = temp.path().join("fresh");
        import(&fresh, &input, None).unwrap();
        assert_eq!(
            read(&fresh).unwrap()["captures"][0]["source_family"],
            "number-format-family"
        );
        import(&target, &input, None).unwrap();
        let state = read(&target).unwrap();
        assert_eq!(state["captures"].as_array().unwrap().len(), 1);
        assert_eq!(state["judgments"][0]["id"], row);
        assert_eq!(state["judgments"][0]["verdict"], "unsure");
        let explicit = temp.path().join("explicit");
        assert!(import(&explicit, &input, Some(&context)).is_ok());
    }
    #[test]
    fn automatic_workload_context_matches_exactly_without_blocking_quick_import() {
        let temp = scratch();
        let input = temp.path().join("runs.jsonl");
        let context = temp.path().join("workloads.json");
        let mut source = run();
        source.as_object_mut().unwrap().remove("source_family");
        fs::write(&input, source.to_string()).unwrap();
        let mut metadata = json!({"schema_version":1,"workloads":[{"id":"six-letters","request":run()["request"],"source_evidence":{"family_id":"automatic-family"}}]});
        fs::write(&context, metadata.to_string()).unwrap();
        let target = temp.path().join("matched");
        import(&target, &input, None).unwrap();
        assert_eq!(
            read(&target).unwrap()["captures"][0]["source_family"],
            "automatic-family"
        );
        metadata["workloads"][0]["request"]["generation"]["text"] = json!("wrong");
        fs::write(&context, metadata.to_string()).unwrap();
        let target = temp.path().join("unmatched");
        import(&target, &input, None).unwrap();
        assert_eq!(
            read(&target).unwrap()["captures"][0]["source_family"],
            Value::Null
        );
        fs::write(&context, "malformed").unwrap();
        let target = temp.path().join("malformed");
        assert_eq!(import(&target, &input, None).unwrap(), 1);
    }
    #[test]
    fn automatic_context_discovery_does_not_fork_existing_review_ids() {
        let temp = scratch();
        let input = temp.path().join("runs.jsonl");
        let mut source = run();
        source.as_object_mut().unwrap().remove("source_family");
        fs::write(&input, source.to_string()).unwrap();
        let target = temp.path().join("existing");
        import(&target, &input, None).unwrap();
        let before = read(&target).unwrap();
        let id = before["captures"][0]["rows"][0]["id"].as_str().unwrap();
        judge(&target, judgment(id, Some("unsure"))).unwrap();
        let metadata = json!({"schema_version":1,"workloads":[{"id":"six-letters","request":run()["request"],"source_evidence":{"family_id":"new-family"},"source_context":{"original_context":"new context"}}]});
        fs::write(temp.path().join("workloads.json"), metadata.to_string()).unwrap();
        assert_eq!(import(&target, &input, None).unwrap(), 0);
        let after = read(&target).unwrap();
        assert_eq!(after["captures"], before["captures"]);
        assert_eq!(after["judgments"][0]["id"], id);
        assert_eq!(after["judgments"][0]["verdict"], "unsure");
        // Explicit source context is a different input, not an automatic
        // enrichment to silently discard or replace with a sibling file.
        let mut contextual = run();
        contextual["source_context"] = json!({"original_context":"Different explicit task"});
        fs::write(&input, contextual.to_string()).unwrap();
        assert_eq!(import(&target, &input, None).unwrap(), 1);
        let after = read(&target).unwrap();
        assert_eq!(after["captures"][1]["source_family"], "family-one");
        assert_eq!(
            after["captures"][1]["source_context"],
            contextual["source_context"]
        );
        assert_ne!(after["captures"][1]["rows"][0]["id"], id);
    }
    #[test]
    fn old_settings_default_off_and_mode_changes_preserve_evidence() {
        let temp = scratch();
        let mut old = serde_json::to_value(crate::settings::Settings::default()).unwrap();
        old.as_object_mut().unwrap().remove("developer_mode");
        let mut s: crate::settings::Settings = serde_json::from_value(old).unwrap();
        assert!(!s.developer_mode);
        store(temp.path(), &[run()]).unwrap();
        s.developer_mode = true;
        crate::settings::save(temp.path(), &s).unwrap();
        s.developer_mode = false;
        crate::settings::save(temp.path(), &s).unwrap();
        assert_eq!(
            read(temp.path()).unwrap()["captures"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    #[ignore = "explicit local native-capture experiment; not a CI input dependency"]
    fn import_local_captures_for_rendered_review_fixture() {
        let temp = scratch();
        let runs = std::env::var("TAJS_REVIEW_QA_RUNS").expect("Set local ranked capture path");
        let workloads =
            std::env::var("TAJS_REVIEW_QA_WORKLOADS").expect("Set matching workload context path");
        let output = std::env::var("TAJS_REVIEW_QA_OUTPUT")
            .expect("Set project-local temporary export path");
        let allowed = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.codex/temp")
            .canonicalize()
            .unwrap();
        assert!(
            Path::new(&output)
                .parent()
                .unwrap()
                .canonicalize()
                .unwrap()
                .starts_with(allowed)
        );
        let n = import(temp.path(), Path::new(&runs), Some(Path::new(&workloads))).unwrap();
        assert!(n > 0);
        assert_eq!(
            import(temp.path(), Path::new(&runs), Some(Path::new(&workloads))).unwrap(),
            0
        );
        let state = read(temp.path()).unwrap();
        assert!(state["judgments"].as_array().unwrap().is_empty());
        export(temp.path(), Path::new(&output)).unwrap();
        eprintln!("Validated {n} actual capture observations, with no invented human labels");
    }
}
