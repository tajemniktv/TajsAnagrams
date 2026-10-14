//! Human-facing snapshot review; never invents judgments or reruns the solver.
use crate::{Args, Result, digest, quality, read_json, root, write_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn sha(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace('`', "&#96;")
        .replace(['\r', '\n'], " ")
}
fn write_changed(path: &Path, s: &str) -> Result<()> {
    if fs::read_to_string(path).ok().as_deref() == Some(s) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, s)?;
    Ok(())
}
fn reference_case(w: &Value) -> Result<quality::Case> {
    let e = &w["source_evidence"];
    let mut relevance = BTreeMap::new();
    let mut seen = BTreeSet::new();
    if let Some(v) = e["relevance"].as_object() {
        for p in v.keys() {
            relevance.insert(p.clone(), 1);
        }
    } else {
        let v = crate::puzzles::verify(e)?;
        for a in v["answers"].as_array().unwrap() {
            if a["exact_letters"] == true
                && a["constraint_violations"].as_array().unwrap().is_empty()
            {
                let p = a["text"].as_str().unwrap();
                if seen.insert(quality::phrase_key(p)) {
                    relevance.insert(p.to_owned(), 1);
                }
            }
        }
    }
    Ok(quality::Case {
        id: w["id"].as_str().ok_or("Missing workload id")?.into(),
        source: w["request"]["generation"]["text"]
            .as_str()
            .ok_or("Missing workload text")?
            .into(),
        split: "train".into(),
        relevance,
        provenance: "Exposed reconstruction references; not human quality labels".into(),
        negatives: vec![],
    })
}
fn context(w: &Value) -> Value {
    json!({"rubric":"standalone-usefulness-v1; coherent sentences/titles/noun phrases; source-task fit separate",
        "case_id":w["id"],"original_input":w["source_evidence"].get("original_input").or_else(||w["source_evidence"].get("source")).cloned().unwrap_or(w["request"]["generation"]["text"].clone()),
        "constraints":w["source_evidence"]["constraints"],"source_context":w["source_context"],"mapping":w["mapping"].get("task")})
}
type Pool = BTreeMap<(String, String), (String, String, String)>; // case/render -> id, rendered phrase, context hash
fn pool(runs: &[Value], workloads: &BTreeMap<String, Value>) -> Result<Pool> {
    let mut pool = Pool::new();
    for r in quality::latest(runs)? {
        let id = r["case_id"].as_str().ok_or("Missing case id")?;
        let w = workloads.get(id).ok_or("Unknown captured case")?;
        if r["request"] != w["request"] {
            return Err("Captured request differs from workload".into());
        }
        if r.get("error").is_some_and(|e| !e.is_null()) {
            continue;
        }
        crate::verify::output(&r["request"], &r["result"])?;
        let hash = sha(&context(w).to_string());
        for row in quality::rows(r, "desktop")?.iter().take(10) {
            let phrase = quality::row_phrase(row);
            if phrase.is_empty() {
                continue;
            }
            let key = quality::rendered_key(&phrase);
            let review_id = sha(&format!("{id}\n{hash}\n{key}"))[..16].to_owned();
            pool.entry((id.to_owned(), key))
                .or_insert((review_id, phrase, hash.clone()));
        }
    }
    Ok(pool)
}
const HEADERS: [&str; 7] = [
    "review_id",
    "case_id",
    "context_sha256",
    "phrase",
    "standalone",
    "source_task",
    "notes",
];
fn annotations(path: &Path, pool: &Pool) -> Result<Vec<Value>> {
    let mut records = Vec::new();
    let mut seen = BTreeSet::new();
    let mut labels = Vec::new();
    if path.exists() {
        let mut reader = csv::Reader::from_path(path)?;
        if reader.headers()?.iter().collect::<Vec<_>>() != HEADERS {
            return Err("Unexpected judgment CSV headers".into());
        }
        for r in reader.records() {
            let r = r?;
            if !seen.insert(r[0].to_owned()) {
                return Err("Duplicate review judgment id".into());
            }
            for i in [4, 5] {
                if !["", "useful", "not_useful", "unsure"].contains(&&r[i]) {
                    return Err("Judgment must be blank/useful/not_useful/unsure".into());
                }
            }
            if let Some((id, _, hash)) = pool.get(&(r[1].into(), quality::rendered_key(&r[3])))
                && r[0] == *id
                && r[2] == *hash
                && ["useful", "not_useful"].contains(&&r[4])
            {
                labels.push(json!({"case_id":r[1],"phrase":r[3],"useful":&r[4]=="useful","provenance":"Human-filled personal review CSV; blinding must be established separately; no population generalization"}));
            }
            records.push(r.iter().map(str::to_owned).collect::<Vec<_>>());
        }
    }
    // Preserve every previous human cell, including rows no longer in this pool.
    for ((case, _), (id, phrase, hash)) in pool {
        if seen.insert(id.clone()) {
            records.push(vec![
                id.clone(),
                case.clone(),
                hash.clone(),
                phrase.clone(),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }
    }
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(HEADERS)?;
    for r in records {
        writer.write_record(r)?;
    }
    write_changed(path, &String::from_utf8(writer.into_inner()?)?)?;
    Ok(labels)
}
fn summary(
    runs: &[Value],
    workloads: &BTreeMap<String, Value>,
    dictionaries: &Value,
    labels: &[Value],
) -> Result<Value> {
    let attempts = quality::latest(runs)?;
    let variants = dictionaries["variants"]
        .as_array()
        .ok_or("Missing variants")?;
    let baseline = variants
        .iter()
        .find(|v| v["id"] == "baseline")
        .ok_or("Missing baseline dictionary")?;
    let base = fs::read_to_string(baseline["path"].as_str().ok_or("Missing dictionary path")?)?
        .lines()
        .map(quality::key)
        .collect::<BTreeSet<_>>();
    let mut panels = Vec::new();
    for v in variants {
        let id = v["id"].as_str().ok_or("Missing variant id")?;
        for state in ["disk_cache_empty", "disk_cache_reuse"] {
            let group = attempts
                .iter()
                .copied()
                .filter(|r| r["dictionary_variant"] == id && r["cache_state"] == state)
                .collect::<Vec<_>>();
            let mut diagnostics = Vec::new();
            let mut latency = Vec::new();
            let mut completed_latency = Vec::new();
            let (mut tokens, mut added, mut caps, mut blank, mut visible) = (0, 0, 0, 0, 0);
            for r in &group {
                if r["dictionary_sha256"] != v["sha256"] {
                    return Err("Captured dictionary hash mismatch".into());
                }
                latency.push(quality::wall(r)?);
                let w = &workloads[r["case_id"].as_str().unwrap()];
                let failed = r.get("error").is_some_and(|e| !e.is_null());
                let c = reference_case(w)?;
                let rows = if failed {
                    vec![]
                } else {
                    quality::rows(r, "desktop")?
                };
                if !failed {
                    completed_latency.push(quality::wall(r)?);
                }
                let target = if c.relevance.is_empty() {
                    Value::Null
                } else {
                    quality::metrics(&c, &rows)?
                };
                for row in rows.iter().take(10) {
                    visible += 1;
                    blank += usize::from(quality::row_phrase(row).is_empty());
                    for t in row["words"].as_array().ok_or("Missing result words")? {
                        tokens += 1;
                        added += usize::from(
                            !base
                                .contains(&quality::key(t.as_str().ok_or("Invalid result token")?)),
                        );
                    }
                }
                caps += usize::from(r["result"]["generation_stop"] == "candidate_cap");
                diagnostics.push(json!({"case_id":c.id,"trial":r["trial"],"completed":!failed,"wall_ms":r["wall_ms"],"reference_recovery":target,
                    "generated":r["result"]["generated"],"stop":r["result"]["generation_stop"]}));
            }
            let usefulness = [1, 5, 10]
                .into_iter()
                .map(|k| quality::usefulness_panel(&group, labels, "desktop", id, state, k))
                .collect::<Result<Vec<_>>>()?;
            panels.push(json!({"variant":id,"cache_state":state,"attempts":group.len(),"cases":group.iter().map(|r|&r["case_id"]).collect::<Vec<_>>().into_iter().map(Value::to_string).collect::<BTreeSet<_>>().len(),
                "completed":completed_latency.len(),"attempt_p50_ms":quality::percentile(&latency,0.5),"attempt_p95_ms":quality::percentile(&latency,0.95),
                "completed_p50_ms":quality::percentile(&completed_latency,0.5),"candidate_cap_attempts":caps,
                "visible_top10_positions":visible,"blank_positions":blank,"added_token_exposure":if tokens==0 {Value::Null}else{json!(added as f64/tokens as f64)},
                "usefulness":usefulness,"per_attempt":diagnostics}));
        }
    }
    let mut paired = Vec::new();
    for v in variants.iter().filter(|v| v["id"] != "baseline") {
        for state in ["disk_cache_empty", "disk_cache_reuse"] {
            let mut cases = Vec::new();
            for id in workloads.keys() {
                let med = |variant: &str| {
                    let times = attempts
                        .iter()
                        .filter(|r| {
                            r["case_id"] == *id
                                && r["dictionary_variant"] == variant
                                && r["cache_state"] == state
                        })
                        .map(|r| quality::wall(r))
                        .collect::<Result<Vec<_>>>()?;
                    Ok::<_, Box<dyn std::error::Error>>(quality::percentile(&times, 0.5))
                };
                let before = med("baseline")?;
                let after = med(v["id"].as_str().unwrap())?;
                cases.push(json!({"case_id":id,"baseline_median_attempt_ms":before,"candidate_median_attempt_ms":after,
                    "delta_ms":after.as_f64().zip(before.as_f64()).map(|(a,b)|a-b)}));
            }
            paired.push(json!({"variant":v["id"],"cache_state":state,"per_case":cases,"interpretation":"Three timing repeats per bag are not independent quality samples; failed attempts retained. No confidence or semantic-noise win inferred."}));
        }
    }
    let cardinality=variants.iter().map(|v|Ok(json!({"variant":v["id"],"diagnostics":crate::lexicon::single_word_diagnostics(v["path"].as_str().ok_or("Missing dictionary")?)?}))).collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"schema_version":1,"human_judgments":labels.len(),"scope":"Exposed development pool, terminal CLI delivery; target recovery and added-token exposure are not human quality/noise labels",
        "groups":panels,"paired_latency":paired,"single_word_cardinality":cardinality}),
    )
}
fn historical(out: &mut String) -> Result<()> {
    let path = root().join("tests/quality/native-baseline.v1.json");
    let baseline = read_json(&path)?;
    out.push_str(&format!("\n## Historical native pilot: actual wording, not English quality labels\n\nSource SHA256: `{}`. Dirty native working-tree capture, top three rows per bucket only.\nThese partial lists cannot reconstruct full top-ten delivery. Scores are model scores, not human judgments. Failed solves remain failures.\n",digest(path)?));
    let cases = quality::load_cases(root().join("tests/quality/cases.v2.json"))?;
    let mut pool = Pool::new();
    for r in baseline["observations"]
        .as_array()
        .ok_or("Missing pilot observations")?
        .iter()
        .filter(|r| r["cache_state"] == "disk_cache_empty" && r["error"].is_null())
    {
        let id = r["case_id"].as_str().unwrap();
        let hash = sha(&format!(
            "historical-pilot-standalone-v1:{id}:{}",
            cases[id].source
        ));
        for row in quality::rows(&json!({"result":{"buckets":r["buckets"]}}), "desktop")? {
            let p = quality::row_phrase(&row);
            if p.is_empty() {
                continue;
            }
            let k = quality::rendered_key(&p);
            let review_id = sha(&format!("{id}\n{hash}\n{k}"))[..16].to_owned();
            pool.entry((id.to_owned(), k))
                .or_insert((review_id, p, hash.clone()));
        }
    }
    let judgment_path = root().join(".codex/temp/tajsbench-pilot-judgments.csv");
    let labels = annotations(&judgment_path, &pool)?;
    out.push_str(&format!("\nPilot judgment CSV: `{}`. All retained nonblank phrases can be judged; the partial projection cannot provide full-list success@K.\n",judgment_path.display()));
    for r in baseline["observations"]
        .as_array()
        .ok_or("Missing pilot observations")?
        .iter()
        .filter(|r| r["cache_state"] == "disk_cache_empty")
    {
        let id = r["case_id"].as_str().ok_or("Missing pilot case")?;
        out.push_str(&format!(
            "\n<details><summary>{}</summary>\n\nInput letters: **{}**.\n\n",
            escape(id),
            escape(&cases[id].source)
        ));
        if !r["error"].is_null() {
            out.push_str(&format!(
                "Failed attempt: {}\n",
                escape(&r["error"].to_string())
            ));
        } else {
            let run = json!({"result":{"buckets":r["buckets"]}});
            out.push_str("| Retained projection position (not original rank) | Review ID | Actual displayed phrase | Model score (not quality) | Human useful label |\n| --- | --- | --- | --- | --- |\n");
            for (i, row) in quality::rows(&run, "desktop")?.iter().enumerate() {
                let p = quality::row_phrase(row);
                let entry = pool.get(&(id.to_owned(), quality::rendered_key(&p)));
                let label = labels.iter().find(|l| {
                    l["case_id"] == id
                        && quality::rendered_key(l["phrase"].as_str().unwrap())
                            == quality::rendered_key(&p)
                });
                out.push_str(&format!(
                    "| {} | {} | {} | {:.2} | {} |\n",
                    i + 1,
                    entry.map_or("no delivered phrase", |(id, _, _)| id.as_str()),
                    escape(&p),
                    row["final"].as_f64().unwrap_or(0.),
                    label.map_or("unjudged / unsure", |l| if l["useful"] == true {
                        "useful"
                    } else {
                        "not useful"
                    })
                ));
            }
        }
        out.push_str("\n</details>\n");
    }
    Ok(())
}
pub fn refresh(config: Option<&Value>, output: &Path) -> Result<()> {
    let mut out = String::from(
        "# TajsBench: human review of captured output\n\nGenerated from stored captures, not a new solver run. Refreshed by native tests/checks and normal build scripts.\nDo not edit this generated file: enter human judgments in the separate CSV; they survive refresh.\nAn exact anagram or a high model score is not proof of useful English. Sentences, coherent titles and noun phrases can all be useful.\n",
    );
    if let Some(mut config) = config.cloned() {
        // Unit tests run from the crate directory; resolve registered paths
        // against the workspace without changing global process cwd.
        for key in ["runs", "workloads", "dictionaries", "judgments", "summary"] {
            if let Some(s) = config[key].as_str()
                && Path::new(s).is_relative()
            {
                config[key] = json!(root().join(s).to_string_lossy());
            }
        }
        let runs_path = config["runs"].as_str().ok_or("Missing review runs")?;
        let runs = quality::read_runs(runs_path)?;
        let data = read_json(
            config["workloads"]
                .as_str()
                .ok_or("Missing review workloads")?,
        )?;
        let mut workloads = BTreeMap::new();
        for w in data["workloads"].as_array().ok_or("Missing workloads")? {
            if workloads
                .insert(
                    w["id"].as_str().ok_or("Missing workload id")?.to_owned(),
                    w.clone(),
                )
                .is_some()
            {
                return Err("Duplicate workload id".into());
            }
        }
        let dictionaries = read_json(
            config["dictionaries"]
                .as_str()
                .ok_or("Missing review dictionaries")?,
        )?;
        for v in dictionaries["variants"]
            .as_array()
            .ok_or("Missing dictionaries")?
        {
            if digest(v["path"].as_str().ok_or("Missing dictionary path")?)? != v["sha256"] {
                return Err("Review dictionary changed".into());
            }
        }
        let pool = pool(&runs, &workloads)?;
        let csv = Path::new(
            config["judgments"]
                .as_str()
                .ok_or("Missing judgment path")?,
        );
        let mut labels = annotations(csv, &pool)?;
        let existing = read_json(root().join("tests/quality/usefulness.v2.json"))?;
        for l in existing["judgments"]
            .as_array()
            .ok_or("Missing usefulness labels")?
        {
            if workloads.contains_key(l["case_id"].as_str().unwrap())
                && !labels.iter().any(|p| {
                    p["case_id"] == l["case_id"]
                        && quality::rendered_key(p["phrase"].as_str().unwrap())
                            == quality::rendered_key(l["phrase"].as_str().unwrap())
                })
            {
                labels.push(l.clone());
            }
        }
        let summary = summary(&runs, &workloads, &dictionaries, &labels)?;
        write_json(config["summary"].as_str(), &summary)?;
        out.push_str(&format!("\n## Current variant-blind judgment pool\n\nCapture SHA256: `{}`. {} bags, {} unique rendered case/phrase pairs. All development-exposed; this is personal, not population-wide validation.\n\nJudgment CSV: `{}`. Fill `standalone` and optionally `source_task` with `useful`, `not_useful` or `unsure`; blank stays unknown. Preserve IDs, phrases and context hashes.\nSource-task fit and standalone usefulness are separate. Do not inspect the comparison panel before judging if you want variant-blind presentation.\n",digest(runs_path)?,workloads.len(),pool.len(),csv.display()));
        for (id, w) in &workloads {
            let context = context(w);
            let c = &context["source_context"];
            out.push_str(&format!("\n<details><summary>{}</summary>\n\nOriginal input: **{}**\n\nContext: {}\n\nSource/theme constraints: {}\n\nLiteral constraints: {}\n\nNative solve: {}–{} words; {}; unsupported structural rules: {}.\n\n| Review ID | Actual rendered output (order randomized, duplicates pooled) |\n| --- | --- |\n",escape(id),escape(context["original_input"].as_str().unwrap_or("")),escape(c["original_context"].as_str().unwrap_or("Any useful exact phrase; exposed authored control")),escape(&c["constraints"].to_string()),escape(&context["constraints"].to_string()),w["request"]["generation"]["min_words"],w["request"]["generation"]["max_words"],escape(w["mapping"]["task"].as_str().unwrap_or("standalone phrase task")),escape(&w["mapping"]["unsupported_structural_constraints"].to_string())));
            let mut entries = pool
                .iter()
                .filter(|((c, _), _)| c == id)
                .map(|(_, v)| v)
                .collect::<Vec<_>>();
            entries.sort_by_key(|(review_id, _, _)| {
                sha(&format!("blind-presentation-v1-{review_id}"))
            });
            for (review_id, p, _) in entries {
                out.push_str(&format!("| {} | {} |\n", review_id, escape(p)));
            }
            out.push_str("\n</details>\n");
        }
        out.push_str("\n<details><summary>Unblinded comparison: open after judgments</summary>\n\nAdded-token exposure is an observable vocabulary-change diagnostic, **not a semantic noise score**. Unknown quality labels remain unknown. Latency includes failures and capture overhead; OS pages are uncontrolled.\n\n| Variant | Cache | Cases / attempts / completed | Attempt p50 / p95 ms | Known useful-success @10 bounds | Added-token exposure | Candidate-cap attempts |\n| --- | --- | --- | --- | --- | --- | --- |\n");
        for g in summary["groups"].as_array().unwrap() {
            let p = &g["usefulness"][2];
            out.push_str(&format!(
                "| {} | {} | {} / {} / {} | {} / {} | {} – {} | {} | {} |\n",
                g["variant"].as_str().unwrap(),
                g["cache_state"].as_str().unwrap(),
                g["cases"],
                g["attempts"],
                g["completed"],
                g["attempt_p50_ms"],
                g["attempt_p95_ms"],
                p["useful_success_lower_bound"],
                p["useful_success_upper_bound"],
                g["added_token_exposure"],
                g["candidate_cap_attempts"]
            ));
        }
        out.push_str(&format!("\nFull diagnostic JSON: `{}`. No adequately judged quality/noise winner is established by automatic metrics.\n\n</details>\n",config["summary"].as_str().unwrap_or("")));
    } else {
        out.push_str("\nNo completed local lexicon experiment registered yet. The historical panel below is still readable.\n");
    }
    historical(&mut out)?;
    let feedback = read_json(root().join("tests/quality/usefulness.v2.json"))?;
    out.push_str("\n## Recorded personal human feedback\n\nUnblinded individual judgments, not calibrated population labels. Plausible/middling/unsure feedback is not converted into a binary negative or an invented relative-preference gate.\n\n| Case | Phrase | Judgment |\n| --- | --- | --- |\n");
    for r in feedback["judgments"]
        .as_array()
        .ok_or("Missing human judgments")?
    {
        out.push_str(&format!(
            "| {} | {} | useful (see versioned provenance) |\n",
            escape(r["case_id"].as_str().unwrap()),
            escape(r["phrase"].as_str().unwrap())
        ));
    }
    for r in feedback["qualitative_judgments"]
        .as_array()
        .ok_or("Missing qualitative judgments")?
    {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            escape(r["case_id"].as_str().unwrap()),
            escape(r["phrase"].as_str().unwrap()),
            escape(r["assessment"].as_str().unwrap())
        ));
    }
    write_changed(output, &out)
}
pub fn command(args: &[String]) -> Result<()> {
    if args == ["--help"] {
        println!(
            "review [--runs FILE --workloads FILE --dictionaries FILE --judgments CSV --summary FILE --output FILE]\nNo arguments refresh .codex/docs/TAJSBENCH.md from pilot and registered local captures; never reruns solver or overwrites human judgments."
        );
        return Ok(());
    }
    let a = Args::parse(
        args,
        &[
            "runs",
            "workloads",
            "dictionaries",
            "judgments",
            "summary",
            "output",
        ],
        &[],
    )?;
    let path = root().join(".codex/temp/tajsbench-review-inputs.json");
    let config = if a.has("runs") {
        Some(
            json!({"runs":a.required("runs")?,"workloads":a.required("workloads")?,"dictionaries":a.required("dictionaries")?,
            "judgments":a.value("judgments",".codex/temp/tajsbench-judgments.csv"),"summary":a.value("summary",".codex/temp/tajsbench-review-summary.json")}),
        )
    } else if path.exists() {
        Some(read_json(&path)?)
    } else {
        None
    };
    let output = a
        .get("output")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root().join(".codex/docs/TAJSBENCH.md"));
    if a.has("runs") {
        refresh(config.as_ref(), &output)?;
    } else {
        refresh_registered(config.as_ref(), &output)?;
    }
    if a.has("runs") {
        write_json(path.to_str(), config.as_ref().unwrap())?;
    }
    Ok(())
}
fn refresh_registered(config: Option<&Value>, output: &Path) -> Result<()> {
    if let Err(error) = refresh(config, output) {
        if config.is_none() {
            return Err(error);
        }
        // Ignored/local research artifacts must not become required build inputs.
        // Explicit --runs still fails closed; automatic refresh labels the loss.
        eprintln!("Local experiment review unavailable: {error}; refreshing historical pilot only");
        refresh(None, output)?;
        let text = fs::read_to_string(output)?;
        write_changed(
            output,
            &format!(
                "**Local experiment unavailable: {}. No current comparison metrics regenerated. Historical pilot only.**\n\n{text}",
                escape(&error.to_string())
            ),
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn human_cells_survive_refresh_unknowns_are_not_invented_labels() {
        let scratch = root().join(".codex/temp/xtask/review-tests");
        fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let path = dir.path().join("judgments.csv");
        let pool = Pool::from([(
            ("a".into(), "it's".into()),
            ("id".into(), "It's".into(), "context".into()),
        )]);
        assert!(annotations(&path, &pool).unwrap().is_empty());
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer.write_record(HEADERS).unwrap();
        writer
            .write_record([
                "id",
                "a",
                "context",
                "It's",
                "useful",
                "unsure",
                "human note | keep",
            ])
            .unwrap();
        fs::write(&path, writer.into_inner().unwrap()).unwrap();
        let before = fs::read_to_string(&path).unwrap();
        assert_eq!(annotations(&path, &pool).unwrap()[0]["useful"], true);
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
        let changed = Pool::from([(
            ("a".into(), "it's".into()),
            ("different".into(), "It's".into(), "newcontext".into()),
        )]);
        assert!(annotations(&path, &changed).unwrap().is_empty());
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("human note | keep")
        );
        assert_eq!(escape("<img>|`x`"), "&lt;img&gt;&#124;&#96;x&#96;");
    }
    #[test]
    fn refresh_local_human_view_from_stored_evidence() {
        // Requested local test hook: refresh only, never launch solver benchmarks.
        command(&[]).unwrap();
        let view = fs::read_to_string(root().join(".codex/docs/TAJSBENCH.md")).unwrap();
        assert!(view.contains("not proof of useful English"));
        assert!(view.contains("another mariner striving"));
    }
    #[test]
    fn missing_local_capture_is_not_a_build_dependency_or_a_fake_success() {
        let scratch = root().join(".codex/temp/xtask/review-tests");
        fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let output = dir.path().join("view.md");
        let config = json!({"runs":dir.path().join("absent.jsonl"),"workloads":"absent.json","dictionaries":"absent.json"});
        assert!(refresh(Some(&config), &output).is_err());
        refresh_registered(Some(&config), &output).unwrap();
        let text = fs::read_to_string(output).unwrap();
        assert!(text.contains("Local experiment unavailable"));
        assert!(text.contains("No current comparison metrics regenerated"));
        assert!(text.contains("another mariner striving"));
    }
}
