//! Research packet intake, not a gold-set importer. No attached code is executed.
//! ASCII inventories are independent of production NFKD normalization.
use crate::{
    Result, benchmark, digest,
    quality::{self, Cases},
    read_json, write_json,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

type Counts = BTreeMap<char, usize>;
fn counts(s: &str) -> Counts {
    let mut out = Counts::new();
    for c in s.chars().filter(char::is_ascii_alphabetic) {
        *out.entry(c.to_ascii_lowercase()).or_default() += 1;
    }
    out
}
fn characters(s: &str) -> Counts {
    let mut out = Counts::new();
    for c in s.chars() {
        *out.entry(c).or_default() += 1;
    }
    out
}
fn difference(a: &Counts, b: &Counts) -> Counts {
    a.iter()
        .filter_map(|(c, n)| {
            let d = n.saturating_sub(*b.get(c).unwrap_or(&0));
            (d > 0).then_some((*c, d))
        })
        .collect()
}
fn total(v: &Counts) -> usize {
    v.values().sum()
}
fn bag(v: &Counts) -> String {
    v.iter()
        .flat_map(|(c, n)| std::iter::repeat_n(*c, *n))
        .collect()
}
fn text<'a>(v: &'a Value, field: &str) -> Result<&'a str> {
    v[field]
        .as_str()
        .ok_or_else(|| format!("Missing string {field}").into())
}
fn array<'a>(v: &'a Value, field: &str) -> Result<&'a [Value]> {
    match v.get(field) {
        None => Ok(&[]),
        Some(v) => v
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| format!("Invalid array {field}").into()),
    }
}
fn delta(answer: &str, input: &Counts) -> Value {
    let a = counts(answer);
    json!({"text":answer,"exact_letters":a==*input,"extra_letters":difference(&a,input),"missing_letters":difference(input,&a)})
}
fn verify(row: &Value) -> Result<Value> {
    let original = text(row, "original_input")?;
    let input = counts(original);
    let rules = row["constraints"]
        .as_object()
        .ok_or("Missing constraint object")?;
    // New constraints must be reviewed, not silently ignored by inventory checks.
    for k in rules.keys() {
        if ![
            "remove_texts",
            "input_letter_count",
            "answer_letter_count",
            "answer_word_count",
            "answer_word_lengths",
            "answer_token_shapes",
            "answer_initial",
            "answer_character_count_including_spaces",
            "preserve_all_characters",
        ]
        .contains(&k.as_str())
        {
            return Err(format!("Unrecognized constraint {k}").into());
        }
    }
    for k in [
        "input_letter_count",
        "answer_letter_count",
        "answer_word_count",
        "answer_character_count_including_spaces",
    ] {
        if rules.get(k).is_some_and(|v| v.as_u64().is_none()) {
            return Err(format!("Invalid {k}").into());
        }
    }
    if rules
        .get("preserve_all_characters")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err("Invalid preserve_all_characters".into());
    }
    for v in array(&row["constraints"], "answer_word_lengths")? {
        if v.as_u64().is_none() {
            return Err("Invalid word length".into());
        }
    }
    for v in array(&row["constraints"], "answer_token_shapes")? {
        if v.as_str().is_none() {
            return Err("Invalid token shape".into());
        }
    }
    if let Some(v) = rules.get("answer_initial")
        && v.as_str()
            .is_none_or(|s| s.len() != 1 || !s.as_bytes()[0].is_ascii_alphabetic())
    {
        return Err("Invalid initial".into());
    }
    let mut removals = Counts::new();
    for t in array(&row["constraints"], "remove_texts")? {
        for (c, n) in counts(t.as_str().ok_or("Non-string removal")?) {
            *removals.entry(c).or_default() += n;
        }
    }
    let over = difference(&removals, &input);
    let residual = difference(&input, &removals);
    let mut violations = Vec::new();
    if !over.is_empty() {
        violations.push("removal exceeds original inventory");
    }
    if rules
        .get("input_letter_count")
        .is_some_and(|v| *v != json!(total(&input)))
    {
        violations.push("input_letter_count");
    }
    let mut answers = Vec::new();
    for a in array(row, "published_answers")? {
        let s = text(a, "text")?;
        let av = counts(s);
        let tokens = s.split_whitespace().collect::<Vec<_>>();
        let lens = tokens
            .iter()
            .map(|s| total(&counts(s)))
            .filter(|n| *n > 0)
            .collect::<Vec<_>>();
        let shapes = tokens
            .iter()
            .map(|s| {
                s.chars()
                    .map(|c| if c.is_ascii_alphabetic() { '#' } else { c })
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let initial = s
            .chars()
            .find(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_lowercase().to_string())
            .unwrap_or_default();
        let mut cv = Vec::new();
        for (k, observed) in [
            ("answer_letter_count", json!(total(&av))),
            ("answer_word_count", json!(lens.len())),
            ("answer_word_lengths", json!(lens)),
            ("answer_token_shapes", json!(shapes)),
            (
                "answer_character_count_including_spaces",
                json!(s.chars().count()),
            ),
        ] {
            if rules.get(k).is_some_and(|v| *v != observed) {
                cv.push(k);
            }
        }
        if rules
            .get("answer_initial")
            .is_some_and(|v| v.as_str().unwrap().to_ascii_lowercase() != initial)
        {
            cv.push("answer_initial");
        }
        if rules.get("preserve_all_characters") == Some(&json!(true))
            && characters(s) != characters(original)
        {
            cv.push("preserve_all_characters");
        }
        let mut result = delta(s, &residual);
        result["letter_count"] = json!(total(&av));
        result["word_lengths"] = json!(lens);
        result["constraint_violations"] = json!(cv);
        answers.push(result);
    }
    let equivalent = array(row, "equivalent_source_inputs")?
        .iter()
        .map(|s| Ok(delta(s.as_str().ok_or("Invalid equivalent input")?, &input)))
        .collect::<Result<Vec<_>>>()?;
    let variants = array(row, "proposed_input_variants")?.iter().map(|v| {
        let s = text(v,"original_input")?;
        let v = counts(s);
        Ok(json!({"input":s,"answers":array(row,"published_answers")?.iter().map(|a|Ok(delta(text(a,"text")?,&v))).collect::<Result<Vec<_>>>()?}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"id":row.get("candidate_id").or_else(||row.get("quarantine_id")).ok_or("Missing puzzle id")?,
        "input_letter_count":total(&input),"removed_letter_counts":removals,"over_removal":over,
        "residual_letter_count":total(&residual),"residual_bag":bag(&residual),"residual_counts":residual,
        "input_constraint_violations":violations,"answers":answers,"equivalent_inputs":equivalent,"proposed_input_variants":variants}),
    )
}
fn jsonl(path: impl AsRef<Path>) -> Result<Vec<Value>> {
    fs::read_to_string(path)?
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|s| Ok(serde_json::from_str(s)?))
        .collect()
}
fn check_manifest(dir: &Path) -> Result<Value> {
    let dir = dir.canonicalize()?;
    if fs::metadata(dir.join("MANIFEST.json"))?.len() > 2 * 1024 * 1024 {
        return Err("Manifest exceeds read limit".into());
    }
    let manifest = read_json(dir.join("MANIFEST.json"))?;
    let mut size = 0;
    let mut files = BTreeSet::new();
    for f in array(&manifest, "files")? {
        let name = text(f, "path")?;
        let path = Path::new(name);
        if path.components().count() != 1
            || !matches!(path.components().next(), Some(Component::Normal(_)))
            || !files.insert(name)
        {
            return Err("Unsafe/duplicate manifest filename".into());
        }
        let path = dir.join(path).canonicalize()?;
        if !path.starts_with(&dir) {
            return Err("Manifest path escapes packet".into());
        }
        let bytes = fs::metadata(&path)?.len();
        size += bytes;
        if bytes > 2 * 1024 * 1024
            || size > 8 * 1024 * 1024
            || f["bytes"].as_u64() != Some(bytes)
            || digest(&path)? != text(f, "sha256")?
        {
            return Err(format!("Size/hash check failed: {name}").into());
        }
    }
    for required in [
        "candidates.jsonl",
        "quarantine.jsonl",
        "sources.jsonl",
        "javascript-inventory-checks.json",
    ] {
        if !files.contains(required) {
            return Err(format!("Manifest missing {required}").into());
        }
    }
    Ok(manifest)
}
fn deliveries(entries: &[Value], runs: &[Value]) -> Result<Value> {
    let mut observations = Vec::new();
    for r in quality::latest(runs)? {
        let id = text(r, "case_id")?;
        let entry = entries
            .iter()
            .find(|e| e["id"] == id && e["candidate"] == true)
            .ok_or("Run not in candidate pool")?;
        if r["request"] != entry["execution_mapping"]["request"] {
            return Err(format!("Run request differs from documented projection: {id}").into());
        }
        let completed = r.get("error").is_none_or(Value::is_null);
        let mut rows = Vec::new();
        if completed {
            crate::verify::output(&r["request"], &r["result"])?;
            let references = entry["independent_verification"]["answers"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| {
                    a["exact_letters"] == true
                        && a["constraint_violations"].as_array().unwrap().is_empty()
                })
                .map(|a| quality::phrase_key(a["text"].as_str().unwrap()))
                .collect::<BTreeSet<_>>();
            for (i, row) in quality::rows(r, "desktop")?.iter().enumerate() {
                let phrase = quality::row_phrase(row);
                let mut source = entry["source_evidence"].clone();
                source["published_answers"] = json!([{"text":phrase}]);
                let check = verify(&source)?;
                let check = &check["answers"][0];
                rows.push(json!({"display_position":i+1,"phrase":phrase,"phrase_delivered":!phrase.is_empty(),
                    "published_reference_order_match":references.contains(&quality::phrase_key(&phrase)),
                    "structural_check":check,"semantic_usefulness":"unjudged"}));
            }
        }
        let top = [1,5,10].into_iter().map(|n| json!({"top_n":n,
            "published_reference_order_hits":rows.iter().take(n).filter(|a|a["published_reference_order_match"]==true).count(),
            "delivered_phrases":rows.iter().take(n).filter(|a|a["phrase_delivered"]==true).count(),
            "structurally_valid_phrases":rows.iter().take(n).filter(|a|a["structural_check"]["exact_letters"]==true && a["structural_check"]["constraint_violations"].as_array().unwrap().is_empty()).count()
        })).collect::<Vec<_>>();
        observations.push(json!({"case_id":id,"family_id":entry["source_evidence"]["family_id"],"input_origin":entry["source_evidence"]["input_origin"],
            "variant":r["variant"],"cache_state":quality::state(r)?,"trial":r.get("trial").cloned().unwrap_or(json!(0)),
            "completed":completed,"wall_ms":r["wall_ms"],"error":r["error"],"top_k":top,"rows":rows}));
    }
    Ok(
        json!({"scope":"terminal desktop-order bag projections; published-reference order recovery is not usefulness or semantic-task success",
        "ranking_match":"legacy normalized word order; punctuation equivalence only for reconstruction diagnostics; structural checks remain literal",
        "observations":observations}),
    )
}
pub fn intake(
    packet: &str,
    fixtures: &Cases,
    selected: Option<&str>,
    workloads_path: Option<&str>,
    runs_path: Option<&str>,
) -> Result<Value> {
    if selected.is_some() != workloads_path.is_some() {
        return Err("Use --case-ids with --workloads-output together".into());
    }
    let dir = Path::new(packet);
    let manifest = check_manifest(dir)?;
    let sources = jsonl(dir.join("sources.jsonl"))?;
    let mut source_map = BTreeMap::new();
    for s in &sources {
        let id = text(s, "source_id")?;
        if id.is_empty() || source_map.insert(id, s).is_some() {
            return Err("Duplicate source id".into());
        }
    }
    let mut evidence = jsonl(dir.join("candidates.jsonl"))?;
    let candidates = evidence.len();
    evidence.extend(jsonl(dir.join("quarantine.jsonl"))?);
    let checks = read_json(dir.join("javascript-inventory-checks.json"))?;
    let checks = checks.as_array().ok_or("Invalid inventory check array")?;
    if checks.len() != evidence.len() {
        return Err("Inventory check count mismatch".into());
    }
    let mut ids = BTreeSet::new();
    let mut bags = BTreeSet::new();
    let mut families = BTreeMap::<String, usize>::new();
    let mut origins = BTreeMap::<String, usize>::new();
    let mut entries = Vec::new();
    let mut workloads = Vec::new();
    let mut remaining = BTreeSet::new();
    if let Some(selected) = selected {
        for id in selected.split(',') {
            if id.is_empty() || !remaining.insert(id.to_owned()) {
                return Err("Empty/duplicate selected id".into());
            }
        }
    }
    for (i, row) in evidence.iter().enumerate() {
        let candidate = i < candidates;
        let id = text(
            row,
            if candidate {
                "candidate_id"
            } else {
                "quarantine_id"
            },
        )?;
        let source = source_map
            .get(text(row, "source_id")?)
            .ok_or("Missing source reference")?;
        if id.is_empty() || !ids.insert(id) || row["family_id"] != source["family_id"] {
            return Err("Duplicate id or inconsistent source family".into());
        }
        let check = verify(row)?;
        // Compare independently derived fields, not the packet's implementation attribution.
        if check != checks[i] {
            return Err(format!("Independent check disagrees with attachment: {id}").into());
        }
        for (k, v) in check.as_object().unwrap() {
            if row["inventory_verification"][k] != *v {
                return Err(format!("Embedded check disagrees: {id}/{k}").into());
            }
        }
        let origin = row["input_origin"]
            .as_str()
            .unwrap_or("quarantined/unspecified");
        if candidate {
            if ![
                "source_supplied_text",
                "source_supplied_letter_count_vector",
                "source_words_paired_by_published_answer",
                "inventory_reconstructed_from_published_answer_set",
            ]
            .contains(&origin)
            {
                return Err("Unrecognized input origin".into());
            }
            if text(&check, "residual_bag")?.is_empty()
                || !bags.insert(text(&check, "residual_bag")?.to_owned())
            {
                return Err("Duplicate candidate residual bag".into());
            }
            *families.entry(text(row, "family_id")?.into()).or_default() += 1;
            *origins.entry(origin.into()).or_default() += 1;
        }
        let original = text(row, "original_input")?;
        let residual = text(&check, "residual_bag")?;
        let mut request = benchmark::desktop_request(original, 10_000);
        let mut transformations = Vec::new();
        if !array(&row["constraints"], "remove_texts")?.is_empty() {
            request["generation"]["text"] = json!(residual);
            transformations
                .push("subtract explicit removal inventories; pass sorted residual letters");
        }
        let word_count = row["constraints"]["answer_word_count"]
            .as_u64()
            .or_else(|| {
                row["constraints"]["answer_word_lengths"]
                    .as_array()
                    .map(|a| a.len() as u64)
            });
        if let Some(n) = word_count {
            request["generation"]["min_words"] = json!(n);
            request["generation"]["max_words"] = json!(n);
            transformations.push("apply source-specified word count (not answer-derived)");
        }
        let extended = residual.len() > 40 || word_count.is_some_and(|n| n > 6);
        let unsupported = row["constraints"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| {
                ![
                    "remove_texts",
                    "input_letter_count",
                    "answer_letter_count",
                    "answer_word_count",
                ]
                .contains(&k.as_str())
            })
            .collect::<Vec<_>>();
        let valid = check["input_constraint_violations"]
            .as_array()
            .unwrap()
            .is_empty()
            && check["answers"].as_array().unwrap().iter().any(|a| {
                a["exact_letters"] == true
                    && a["constraint_violations"].as_array().unwrap().is_empty()
            });
        let overlap = fixtures
            .values()
            .filter(|c| quality::signature(&c.source) == residual)
            .map(|c| &c.id)
            .collect::<Vec<_>>();
        let mapping = json!({"request":request,"transformations":transformations,"unsupported_structural_constraints":unsupported,
            "semantic_context_passed_to_solver":false,"task":"bag-only projection; source context retained for judgment, not solved by CLI",
            "identity_policy":"unadjudicated; do not credit unchanged input or mere reordering without task review",
            "profile":if extended {"extended/stress"} else {"normal"}});
        if remaining.remove(id) {
            if !candidate
                || !valid
                || extended
                || counts(original).is_empty()
                || quality::signature(original) != bag(&counts(original))
            {
                return Err(format!("Not eligible for normal dry run: {id}").into());
            }
            workloads.push(json!({"id":id,"provenance":format!("Echo packet {}: unjudged development-exposed bag projection",digest(dir.join("MANIFEST.json"))?),
                "request":request,"source_evidence":row,"source_context":source,"mapping":mapping}));
        }
        entries.push(json!({"id":id,"candidate":candidate,"source_evidence":row,"source_context":source,"independent_verification":check,
            "has_structurally_valid_published_answer":valid,"existing_fixture_overlap":overlap,"execution_mapping":mapping}));
    }
    if !remaining.is_empty() {
        return Err(format!("Unknown/unselected ids: {remaining:?}").into());
    }
    if let Some(path) = workloads_path {
        write_json(
            Some(path),
            &json!({"schema_version":1,"quality_labels":false,"exposure":"development; inputs and answers inspected by Echo and Codex", "workloads":workloads}),
        )?;
    }
    let mut report = json!({"schema_version":1,"manifest":manifest,"manifest_sha256":digest(dir.join("MANIFEST.json"))?,
        "candidate_count":candidates,"quarantine_record_count":evidence.len()-candidates,"source_count":sources.len(),
        "source_family_counts":families,"input_origin_counts":origins,"fixture_count_checked":fixtures.len(),
        "verification":"independent Rust ASCII inventories and explicit structural constraints agree with all attached records; no semantic/source truth inferred",
        "exposure":"development only; all inputs and published answers inspected; no human labels or holdout inferred",
        "entries":entries});
    if let Some(path) = runs_path {
        report["delivery_diagnostics"] = deliveries(&entries, &quality::read_runs(path)?)?;
        report["runs_sha256"] = json!(digest(path)?);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projections_preserve_evidence_without_promoting_quarantine_or_answers() {
        let scratch = crate::root().join(".codex/temp/xtask/puzzle-tests");
        fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let row = json!({"candidate_id":"a","source_id":"s","family_id":"f","input_origin":"source_supplied_text",
            "original_input":"abcat","constraints":{"remove_texts":["ab"],"answer_word_count":1},"published_answers":[{"text":"cat"}]});
        let mut row = row;
        row["inventory_verification"] = verify(&row).unwrap();
        let mut q = row.clone();
        q.as_object_mut().unwrap().remove("candidate_id");
        q["quarantine_id"] = json!("q");
        q["inventory_verification"] = verify(&q).unwrap();
        let files = [
            ("candidates.jsonl", row.to_string()),
            ("quarantine.jsonl", q.to_string()),
            (
                "sources.jsonl",
                json!({"source_id":"s","family_id":"f","constraints":{"semantic_target":"animal"}})
                    .to_string(),
            ),
            (
                "javascript-inventory-checks.json",
                json!([verify(&row).unwrap(), verify(&q).unwrap()]).to_string(),
            ),
        ];
        let mut manifest = Vec::new();
        for (name, s) in files {
            fs::write(dir.path().join(name), &s).unwrap();
            manifest.push(json!({"path":name,"bytes":s.len(),"sha256":digest(dir.path().join(name)).unwrap()}));
        }
        fs::write(
            dir.path().join("MANIFEST.json"),
            json!({"files":manifest}).to_string(),
        )
        .unwrap();
        let output = dir.path().join("workloads.json");
        let report = intake(
            dir.path().to_str().unwrap(),
            &Cases::new(),
            Some("a"),
            output.to_str(),
            None,
        )
        .unwrap();
        assert_eq!(report["quarantine_record_count"], 1);
        let w = read_json(&output).unwrap();
        let w = &w["workloads"][0];
        assert_eq!(w["request"]["generation"]["text"], "act");
        assert_eq!(w["request"]["generation"]["max_words"], 1);
        assert_eq!(w["request"]["generation"]["required"], json!([]));
        assert_eq!(w["request"]["generation"]["hints"], json!([]));
        assert_eq!(w["source_evidence"]["original_input"], "abcat");
        assert_eq!(
            w["source_context"]["constraints"]["semantic_target"],
            "animal"
        );
        assert!(
            intake(
                dir.path().to_str().unwrap(),
                &Cases::new(),
                Some("q"),
                output.to_str(),
                None
            )
            .is_err()
        );
        assert!(
            intake(
                dir.path().to_str().unwrap(),
                &Cases::new(),
                Some("unknown"),
                output.to_str(),
                None
            )
            .is_err()
        );
        fs::write(dir.path().join("sources.jsonl"), "changed").unwrap();
        assert!(
            intake(
                dir.path().to_str().unwrap(),
                &Cases::new(),
                None,
                None,
                None
            )
            .is_err()
        );
    }
    #[test]
    fn diagnostic_order_hits_do_not_claim_literal_constraints_or_usefulness() {
        let source = json!({"candidate_id":"a","original_input":"ab","constraints":{"answer_token_shapes":["#'#"]},"published_answers":[{"text":"A'b"}]});
        let request = benchmark::desktop_request("ab", 10_000);
        let entry = json!({"id":"a","candidate":true,"source_evidence":source,"independent_verification":verify(&source).unwrap(),"execution_mapping":{"request":request}});
        let mut run = json!({"case_id":"a","variant":"v","cache_state":"uncached","wall_ms":1,"request":request,
            "result":{"buckets":{"1":[{"words":["ab"],"best_order":["ab"],"display_phrase":"ab","deep":true}]}}});
        let entries = [entry];
        let report = deliveries(&entries, &[run.clone()]).unwrap();
        assert_eq!(
            report["observations"][0]["top_k"][0]["published_reference_order_hits"],
            1
        );
        assert_eq!(
            report["observations"][0]["top_k"][0]["structurally_valid_phrases"],
            0
        );
        run["error"] = json!({"code":"timed_out"});
        let report = deliveries(&entries, &[run.clone()]).unwrap();
        assert_eq!(
            report["observations"][0]["top_k"][0]["delivered_phrases"],
            0
        );
        run["request"]["generation"]["hints"] = json!(["ab"]);
        assert!(deliveries(&entries, &[run]).is_err());
    }
    #[test]
    fn independent_inventory_removal_and_constraint_failures() {
        let row = json!({"candidate_id":"a","original_input":"TéA cats!", "constraints":{"remove_texts":["ta"],"answer_word_count":1,"answer_initial":"C","preserve_all_characters":true},"published_answers":[{"text":"cats"},{"text":"cat's s"}]});
        let v = verify(&row).unwrap();
        assert_eq!(v["residual_bag"], "acst");
        assert_eq!(v["answers"][0]["exact_letters"], true);
        assert_eq!(
            v["answers"][0]["constraint_violations"],
            json!(["preserve_all_characters"])
        );
        assert_eq!(v["answers"][1]["extra_letters"], json!({"s":1}));
        assert_eq!(
            v["answers"][1]["constraint_violations"],
            json!(["answer_word_count", "preserve_all_characters"])
        );
        let row =
            json!({"candidate_id":"a","original_input":"a","constraints":{"remove_texts":["aa"]}});
        assert_eq!(verify(&row).unwrap()["over_removal"], json!({"a":1}));
        assert!(verify(&json!({"candidate_id":"a","original_input":"a","constraints":{"new_constraint":true}})).is_err());
    }
    #[test]
    fn literal_shapes_codepoint_length_and_equivalent_inputs() {
        let row = json!({"candidate_id":"a","original_input":"ab","constraints":{"answer_word_lengths":[2],"answer_token_shapes":["#'#"],"answer_character_count_including_spaces":3},"published_answers":[{"text":"A'b"},{"text":"ab"}],"equivalent_source_inputs":["BA","abc"],"proposed_input_variants":[{"original_input":"abc"}]});
        let v = verify(&row).unwrap();
        assert_eq!(v["answers"][0]["constraint_violations"], json!([]));
        assert_eq!(
            v["answers"][1]["constraint_violations"],
            json!([
                "answer_token_shapes",
                "answer_character_count_including_spaces"
            ])
        );
        assert_eq!(v["equivalent_inputs"][1]["extra_letters"], json!({"c":1}));
        assert_eq!(
            v["proposed_input_variants"][0]["answers"][0]["missing_letters"],
            json!({"c":1})
        );
    }
    #[test]
    fn unsafe_and_corrupt_manifest_fails_closed() {
        let scratch = crate::root().join(".codex/temp/xtask/puzzle-tests");
        fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        fs::write(
            dir.path().join("MANIFEST.json"),
            json!({"files":[{"path":"../escape","bytes":0,"sha256":""}]}).to_string(),
        )
        .unwrap();
        assert!(check_manifest(dir.path()).is_err());
        fs::write(dir.path().join("test.json"), "{}").unwrap();
        fs::write(
            dir.path().join("MANIFEST.json"),
            json!({"files":[{"path":"test.json","bytes":2,"sha256":"incorrect"}]}).to_string(),
        )
        .unwrap();
        assert!(check_manifest(dir.path()).is_err());
    }
}
