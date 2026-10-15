//! Offline, provenance-preserving model-review intake. No quality-label promotion.
use crate::{Args, Result, digest, read_json, write_json};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};

const RUBRIC: &str = "standalone-usefulness-v1";
const REASONS: &[&str] = &[
    "grammar",
    "meaning",
    "incomplete_meaning",
    "semantic_anomaly",
    "contrived_context",
    "unfamiliar",
    "unverified_name",
    "spelling_rendering",
    "unspecified",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Useful,
    NotUseful,
    Unsure,
}
impl Verdict {
    fn index(self) -> usize {
        match self {
            Self::Useful => 0,
            Self::NotUseful => 1,
            Self::Unsure => 2,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Useful => "Yes",
            Self::NotUseful => "No",
            Self::Unsure => "Maybe",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Judgment {
    item_id: String,
    verdict: Verdict,
    reasons: Vec<String>,
    explanation: String,
    verification_needed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phrase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    puzzle: Option<usize>,
}
#[derive(Clone, Debug, Serialize)]
struct Item {
    item_id: String,
    case_id: String,
    puzzle: usize,
    phrase: String,
    input: Option<String>,
    source_family: Option<String>,
    human: Verdict,
    models: BTreeMap<String, Judgment>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reviewer {
    id: String,
    path: String,
    reported_model: String,
    #[serde(default)]
    reported_reasoning_effort: Option<String>,
    provenance_note: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    packet: String,
    mapping: String,
    human_export: String,
    #[serde(default)]
    workloads: Option<String>,
    reviewers: Vec<Reviewer>,
}
fn array<'a>(v: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    v[key]
        .as_array()
        .ok_or_else(|| format!("Missing array {key}").into())
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("Missing string {key}").into())
}
fn counts(votes: impl Iterator<Item = Verdict>) -> [usize; 3] {
    let mut c = [0; 3];
    for v in votes {
        c[v.index()] += 1;
    }
    c
}
fn count_json(c: [usize; 3]) -> Value {
    json!({"useful":c[0],"not_useful":c[1],"unsure":c[2]})
}
fn comparison(pairs: impl Iterator<Item = (Verdict, Verdict)>) -> Value {
    let mut matrix = [[0usize; 3]; 3];
    let mut total = 0;
    for (a, b) in pairs {
        matrix[a.index()][b.index()] += 1;
        total += 1;
    }
    let both = matrix[0][0] + matrix[0][1] + matrix[1][0] + matrix[1][1];
    let agree = matrix[0][0] + matrix[1][1];
    json!({"matrix_order":["useful","not_useful","unsure"],"matrix":matrix,
        "total":total,"exact_three_way_agreement":agree+matrix[2][2],
        "both_definite":both,"agreement_when_both_definite":agree,
        "opposite_definite_verdicts":both-agree})
}
fn parse_packet(packet: &str) -> Result<BTreeMap<String, (usize, String)>> {
    let mut out = BTreeMap::new();
    let mut puzzle = 0;
    for line in packet.lines() {
        if let Some(n) = line.strip_prefix("## Puzzle ") {
            puzzle = n.parse()?;
        }
        if !line.starts_with("| item_") {
            continue;
        }
        let fields: Vec<_> = line.split('|').map(str::trim).collect();
        if fields.len() != 4 || puzzle == 0 || fields[2].is_empty() {
            return Err("Malformed packet row/puzzle".into());
        }
        if out
            .insert(fields[1].into(), (puzzle, fields[2].into()))
            .is_some()
        {
            return Err("Duplicate packet item".into());
        }
    }
    if out.is_empty() {
        return Err("Empty review packet".into());
    }
    Ok(out)
}
fn load_items(
    packet: &str,
    mapping: &Value,
    human: &Value,
    workloads: Option<&Value>,
) -> Result<BTreeMap<String, Item>> {
    if human["schema_version"] != 1 {
        return Err("Unsupported human export schema".into());
    }
    let packet = parse_packet(packet)?;
    let mut judgments = BTreeMap::new();
    for j in array(human, "judgments")? {
        if judgments.insert(string(j, "id")?, j).is_some() {
            return Err("Duplicate human judgment".into());
        }
    }
    let mut captured = BTreeMap::new();
    for c in array(human, "captures")? {
        for row in array(c, "rows")? {
            let id = string(row, "id")?;
            let phrase = string(row, "phrase")?;
            let case = string(c, "case_id")?;
            if let Some((old_phrase, old_case, _)) = captured.get(id)
                && (*old_phrase != phrase || *old_case != case)
            {
                return Err("Conflicting captured row identity".into());
            }
            captured.insert(id, (phrase, case, c));
        }
    }
    let mut items = BTreeMap::new();
    let mut used_rows = std::collections::BTreeSet::new();
    for m in array(mapping, "mapping")? {
        let id = string(m, "item_id")?;
        let row_id = string(m, "row_id")?;
        let (puzzle, phrase) = packet.get(id).ok_or("Mapping item not in packet")?;
        let (captured_phrase, case, c) = captured.get(row_id).ok_or("Mapping row not captured")?;
        if phrase != captured_phrase || string(m, "case_id")? != *case || !used_rows.insert(row_id)
        {
            return Err("Mapping phrase/case mismatch or duplicate human row".into());
        }
        let human = serde_json::from_value(
            judgments.get(row_id).ok_or("Missing human judgment")?["verdict"].clone(),
        )?;
        let enriched = workloads
            .and_then(|w| w["workloads"].as_array())
            .and_then(|ws| {
                ws.iter()
                    .find(|w| w["id"] == *case && w["request"] == c["request"])
            })
            .and_then(|w| w["source_evidence"]["family_id"].as_str());
        let family = c["source_family"].as_str().or(enriched).map(String::from);
        if items
            .insert(
                id.into(),
                Item {
                    item_id: id.into(),
                    case_id: (*case).into(),
                    puzzle: *puzzle,
                    phrase: phrase.clone(),
                    input: c["input"]
                        .as_str()
                        .or_else(|| c["request"]["generation"]["text"].as_str())
                        .map(String::from),
                    source_family: family,
                    human,
                    models: BTreeMap::new(),
                },
            )
            .is_some()
        {
            return Err("Duplicate mapping item".into());
        }
    }
    if items.len() != packet.len() {
        return Err("Incomplete packet mapping".into());
    }
    Ok(items)
}
fn model_judgments(
    source: &Value,
    items: &BTreeMap<String, Item>,
    packet_hash: &str,
) -> Result<Vec<Judgment>> {
    let meta = source.get("metadata").unwrap_or(source);
    if meta["reviewer_source"] != "llm" || meta["rubric_version"] != RUBRIC {
        return Err("Model source/rubric mismatch".into());
    }
    if let Some(hash) = meta.get("input_sha256")
        && hash.as_str() != Some(packet_hash)
    {
        return Err("Model packet hash mismatch".into());
    }
    let rows: Vec<Judgment> = serde_json::from_value(source["judgments"].clone())?;
    let mut seen = std::collections::BTreeSet::new();
    for row in &rows {
        let expected = items.get(&row.item_id).ok_or("Unknown model item ID")?;
        if !seen.insert(&row.item_id)
            || row.phrase.as_ref().is_some_and(|p| p != &expected.phrase)
            || row.puzzle.is_some_and(|p| p != expected.puzzle)
        {
            return Err("Duplicate/mismatched model item".into());
        }
        if row.explanation.trim().is_empty()
            || row.reasons.iter().any(|r| !REASONS.contains(&r.as_str()))
        {
            return Err("Invalid model explanation/reason".into());
        }
    }
    if rows.len() != items.len() {
        return Err("Incomplete model coverage".into());
    }
    let summary = &source["summary"];
    let n = summary
        .get("item_count")
        .or_else(|| summary.get("total_items"));
    let puzzles = items
        .values()
        .map(|r| r.puzzle)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let p = summary
        .get("puzzle_count")
        .or_else(|| summary.get("total_puzzles"));
    if n.and_then(Value::as_u64) != Some(rows.len() as u64)
        || p.and_then(Value::as_u64) != Some(puzzles as u64)
        || summary["verdict_counts"] != count_json(counts(rows.iter().map(|r| r.verdict)))
        || summary["verification_needed_count"].as_u64()
            != Some(rows.iter().filter(|r| r.verification_needed).count() as u64)
    {
        return Err("Model summary disagrees with judgments".into());
    }
    Ok(rows)
}
fn analyze(items: &BTreeMap<String, Item>, reviewers: &[String]) -> Value {
    let mut comparisons = BTreeMap::new();
    for id in reviewers {
        let definite: Vec<_> = items
            .values()
            .filter(|r| r.human != Verdict::Unsure)
            .collect();
        comparisons.insert(id.clone(),json!({
            "verdict_counts":count_json(counts(items.values().map(|r|r.models[id].verdict))),
            "all_human":comparison(items.values().map(|r|(r.human,r.models[id].verdict))),
            "definite_human":comparison(definite.iter().map(|r|(r.human,r.models[id].verdict))),
            "model_abstentions_on_definite_human":definite.iter().filter(|r|r.models[id].verdict==Verdict::Unsure).count(),
            "uncertain_human_model_votes":count_json(counts(items.values().filter(|r|r.human==Verdict::Unsure).map(|r|r.models[id].verdict)))
        }));
    }
    let mut pairs = BTreeMap::new();
    for (i, a) in reviewers.iter().enumerate() {
        for b in &reviewers[i + 1..] {
            pairs.insert(
                format!("{a} / {b}"),
                comparison(
                    items
                        .values()
                        .map(|r| (r.models[a].verdict, r.models[b].verdict)),
                ),
            );
        }
    }
    let mut groups: BTreeMap<&str, Vec<&Item>> = BTreeMap::new();
    for r in items.values() {
        groups.entry(&r.case_id).or_default().push(r);
    }
    let per_puzzle: Vec<_> = groups.values().map(|rows| {
        let mut votes = BTreeMap::from([("human".to_string(),count_json(counts(rows.iter().map(|r|r.human))))]);
        for id in reviewers { votes.insert(id.clone(),count_json(counts(rows.iter().map(|r|r.models[id].verdict)))); }
        json!({"puzzle":rows[0].puzzle,"case_id":rows[0].case_id,"input":rows[0].input,"source_family":rows[0].source_family,"items":rows.len(),"verdicts":votes})
    }).collect();
    let consensus_against_human: Vec<_> = items
        .values()
        .filter(|r| {
            let v = r.models[&reviewers[0]].verdict;
            reviewers.len() > 1
                && r.human != Verdict::Unsure
                && v != Verdict::Unsure
                && v != r.human
                && reviewers.iter().all(|id| r.models[id].verdict == v)
        })
        .map(|r| &r.item_id)
        .collect();
    json!({"items":items.len(),"puzzles":groups.len(),"human_verdict_counts":count_json(counts(items.values().map(|r|r.human))),
        "known_source_families":items.values().filter_map(|r|r.source_family.as_deref()).collect::<std::collections::BTreeSet<_>>().len(),
        "unknown_family_puzzles":groups.values().filter(|rows|rows.iter().any(|r|r.source_family.is_none())).count(),
        "reviewers":comparisons,"model_pairs":pairs,"per_puzzle":per_puzzle,
        "consensus_against_definite_human":consensus_against_human,
        "verification_needed":items.values().filter(|r|r.models.values().any(|j|j.verification_needed)).map(|r|&r.item_id).collect::<Vec<_>>()})
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('|', "&#124;")
        .replace(['\r', '\n'], " ")
}
fn readable(report: &Value, items: &BTreeMap<String, Item>, ids: &[String]) -> String {
    let mut out = String::from(
        "# TajsBench: returned model review comparison\n\nGenerated by `cargo xtask review models`. These are proposals, not human labels or a quality gate.\nUnranked packet items; this is not delivered-list quality or independent per-row evidence.\nMaybe is uncertainty, never No. Model identities/effort and blinding are reported, not authenticated.\n\n## Provenance\n\n",
    );
    for id in ids {
        let p = &report["provenance"][id];
        out.push_str(&format!(
            "- **{}**: {}. {}\n",
            escape(id),
            escape(p["reported_model"].as_str().unwrap()),
            escape(p["provenance_note"].as_str().unwrap())
        ));
    }
    out.push_str("\n## Model-to-model agreement\n\n");
    for (pair, p) in report["analysis"]["model_pairs"].as_object().unwrap() {
        out.push_str(&format!("- {}: {} / {} identical three-way verdicts; {} opposite definite verdicts; {} items definite for both. This is agreement, not independent validation.\n",escape(pair),p["exact_three_way_agreement"],p["total"],p["opposite_definite_verdicts"],p["both_definite"]));
    }
    out.push_str(&format!("\n## Puzzle clusters\n\n{} puzzles, {} known source families, {} puzzles with unknown family. Counts are descriptive; no row-independent confidence intervals. Counts below are Yes / No / Maybe.\n\n| Puzzle | Family | Human | {} |\n| --- | --- | --- | {} |\n",report["analysis"]["puzzles"],report["analysis"]["known_source_families"],report["analysis"]["unknown_family_puzzles"],ids.iter().map(|id|escape(id)).collect::<Vec<_>>().join(" | "),vec!["---";ids.len()].join(" | ")));
    let votes_text = |v: &Value| format!("{} / {} / {}", v["useful"], v["not_useful"], v["unsure"]);
    for p in report["analysis"]["per_puzzle"].as_array().unwrap() {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            p["puzzle"],
            escape(p["source_family"].as_str().unwrap_or("unknown")),
            votes_text(&p["verdicts"]["human"]),
            ids.iter()
                .map(|id| votes_text(&p["verdicts"][id]))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    out.push_str("\n<details><summary>Original metadata and input hashes</summary>\n\n<pre>");
    out.push_str(&escape(
        &serde_json::to_string_pretty(
            &json!({"provenance":report["provenance"],"input_sha256":report["input_sha256"]}),
        )
        .unwrap(),
    ));
    out.push_str("</pre>\n\n</details>\n\n");
    out.push_str("## Agreement on definite human judgments\n\nConditional agreement excludes model abstentions; coverage includes them in the denominator. Neither is accuracy against objective truth.\n\n| Reviewer | Agree | Opposite verdict | Abstain | Definite human total |\n| --- | ---: | ---: | ---: | ---: |\n");
    for id in ids {
        let r = &report["analysis"]["reviewers"][id];
        let d = &r["definite_human"];
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape(id),
            d["agreement_when_both_definite"],
            d["opposite_definite_verdicts"],
            r["model_abstentions_on_definite_human"],
            d["total"]
        ));
    }
    out.push_str("\n## Definite consensus against human: adjudicate, do not overwrite\n\n");
    for id in report["analysis"]["consensus_against_definite_human"]
        .as_array()
        .unwrap()
    {
        let r = &items[id.as_str().unwrap()];
        out.push_str(&format!(
            "- **{}** — human {}, models {}.\n",
            escape(&r.phrase),
            r.human.label(),
            r.models[&ids[0]].verdict.label()
        ));
    }
    out.push_str("\n## Actual proposals grouped by puzzle\n\nEach row is sampled once. Original per-observation ranks remain in the human export, not this blinded packet. Reasons are reviewer explanations, not established facts.\n\n");
    let mut rows: Vec<_> = items.values().collect();
    rows.sort_by_key(|r| (r.puzzle, &r.item_id));
    let mut puzzle = 0;
    for r in rows {
        if r.puzzle != puzzle {
            puzzle = r.puzzle;
            out.push_str(&format!("\n### Puzzle {} — {}\n\nSource family: {}.\n\n| Exact output | Human | {} |\n| --- | --- | {} |\n",puzzle,escape(r.input.as_deref().unwrap_or(&r.case_id)),escape(r.source_family.as_deref().unwrap_or("unknown")),ids.iter().map(|id|escape(id)).collect::<Vec<_>>().join(" | "),vec!["---";ids.len()].join(" | ")));
        }
        let votes = ids
            .iter()
            .map(|id| {
                let j = &r.models[id];
                format!(
                    "{}{}: {}",
                    j.verdict.label(),
                    if j.verification_needed {
                        " [lookup]"
                    } else {
                        ""
                    },
                    escape(&j.explanation)
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            escape(&r.phrase),
            r.human.label(),
            votes
        ));
    }
    out
}
pub fn command(args: &[String]) -> Result<()> {
    if args == ["--help"] {
        println!(
            "review models --manifest FILE --output JSON --readable MARKDOWN\nManifest: packet, mapping, human_export, optional workloads, reviewers [id,path,reported_model,optional reported_reasoning_effort,provenance_note]. Paths relative to manifest. Inputs never overwritten; model reviews stay separate."
        );
        return Ok(());
    }
    let a = Args::parse(args, &["manifest", "output", "readable"], &[])?;
    let manifest_path = Path::new(a.required("manifest")?).canonicalize()?;
    let base = manifest_path.parent().unwrap();
    let manifest: Manifest = serde_json::from_value(read_json(&manifest_path)?)?;
    if manifest.reviewers.is_empty() {
        return Err("No model reviewers".into());
    }
    let mut inputs = vec![manifest_path.clone()];
    let mut hashes = BTreeMap::from([("manifest".to_string(), digest(&manifest_path)?)]);
    let mut resolve = |label: &str, p: &str| -> Result<std::path::PathBuf> {
        let path = base.join(p).canonicalize()?;
        hashes.insert(label.to_string(), digest(&path)?);
        inputs.push(path.clone());
        Ok(path)
    };
    let packet_path = resolve("packet", &manifest.packet)?;
    let mapping_path = resolve("mapping", &manifest.mapping)?;
    let human_path = resolve("human_export", &manifest.human_export)?;
    let workloads = manifest
        .workloads
        .as_ref()
        .map(|p| resolve("workloads", p).and_then(read_json))
        .transpose()?;
    let mut model_sources = Vec::new();
    for reviewer in &manifest.reviewers {
        model_sources.push(read_json(resolve(
            &format!("model:{}", reviewer.id),
            &reviewer.path,
        )?)?);
    }
    let mapping = read_json(mapping_path)?;
    if mapping["export_sha256"].as_str() != Some(&hashes["human_export"]) {
        return Err("Mapping human-export hash mismatch".into());
    }
    let mut items = load_items(
        &fs::read_to_string(packet_path)?,
        &mapping,
        &read_json(human_path)?,
        workloads.as_ref(),
    )?;
    let mut provenance = BTreeMap::new();
    let mut ids = Vec::new();
    for (r, source) in manifest.reviewers.iter().zip(&model_sources) {
        if r.id == "human" || r.id.trim().is_empty() || provenance.contains_key(&r.id) {
            return Err("Invalid/duplicate reviewer ID".into());
        }
        for j in model_judgments(source, &items, &hashes["packet"])? {
            items
                .get_mut(&j.item_id)
                .unwrap()
                .models
                .insert(r.id.clone(), j);
        }
        let mut metadata = source.clone();
        metadata
            .as_object_mut()
            .ok_or("Invalid model root")?
            .remove("judgments");
        provenance.insert(r.id.clone(),json!({"reported_model":r.reported_model,"reported_reasoning_effort":r.reported_reasoning_effort,"provenance_note":r.provenance_note,"original_metadata":metadata}));
        ids.push(r.id.clone());
    }
    // Resolve both outputs before writing either: never replace evidence or each other.
    let output = Path::new(a.required("output")?);
    let md = Path::new(a.required("readable")?);
    for p in [output, md] {
        if p.exists() && inputs.contains(&p.canonicalize()?) {
            return Err("Output aliases an input".into());
        }
    }
    let absolute = |p: &Path| -> Result<std::path::PathBuf> {
        if p.exists() {
            return Ok(p.canonicalize()?);
        }
        Ok(p.parent()
            .unwrap_or(Path::new("."))
            .canonicalize()?
            .join(p.file_name().ok_or("Output filename missing")?))
    };
    let output_id = absolute(output)?;
    let md_id = absolute(md)?;
    if output_id == md_id
        || cfg!(windows)
            && output_id
                .to_string_lossy()
                .eq_ignore_ascii_case(&md_id.to_string_lossy())
    {
        return Err("Outputs must differ".into());
    }
    let report = json!({"schema_version":1,"rubric_version":RUBRIC,"input_sha256":hashes,"provenance":provenance,
        "limits":["Model proposals never replace human judgments","Self-reported identities and blinding are not provider authentication","Development-exposed convenience sample, not held-out calibration","No row-independent uncertainty intervals or corpus/ranking promotion","Uncertain human votes stay separate; conditional agreement requires coverage"],
        "analysis":analyze(&items,&ids),"rows":items.values().collect::<Vec<_>>()});
    let text = readable(&report, &items, &ids);
    write_json(output.to_str(), &report)?;
    fs::write(md, text)?;
    println!(
        "Validated {} proposals per reviewer across {} puzzles; human evidence unchanged",
        items.len(),
        report["analysis"]["puzzles"]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn items() -> BTreeMap<String, Item> {
        BTreeMap::from([(
            "item_a".into(),
            Item {
                item_id: "item_a".into(),
                case_id: "case".into(),
                puzzle: 1,
                phrase: "valid phrase".into(),
                input: None,
                source_family: None,
                human: Verdict::Useful,
                models: BTreeMap::new(),
            },
        )])
    }
    fn source() -> Value {
        json!({"reviewer_source":"llm","rubric_version":RUBRIC,"summary":{"item_count":1,"puzzle_count":1,"verdict_counts":{"useful":1,"not_useful":0,"unsure":0},"verification_needed_count":0},"judgments":[{"item_id":"item_a","phrase":"valid phrase","puzzle":1,"verdict":"useful","reasons":[],"explanation":"Test explanation","verification_needed":false}]})
    }
    #[test]
    fn abstentions_never_become_negative_or_inflate_conditional_coverage() {
        let v = comparison(
            [
                (Verdict::Useful, Verdict::Useful),
                (Verdict::NotUseful, Verdict::Unsure),
                (Verdict::Unsure, Verdict::NotUseful),
            ]
            .into_iter(),
        );
        assert_eq!(v["total"], 3);
        assert_eq!(v["both_definite"], 1);
        assert_eq!(v["agreement_when_both_definite"], 1);
        assert_eq!(v["opposite_definite_verdicts"], 0);
        assert_eq!(v["matrix"][1][2], 1);
        assert_eq!(v["matrix"][2][1], 1);
    }
    #[test]
    fn accepts_both_metadata_layouts_without_inventing_model_identity() {
        let s = source();
        assert_eq!(model_judgments(&s, &items(), "hash").unwrap().len(), 1);
        let wrapped = json!({"metadata":{"reviewer_source":"llm","rubric_version":RUBRIC,"input_sha256":"hash"},"summary":s["summary"],"judgments":s["judgments"]});
        assert!(model_judgments(&wrapped, &items(), "hash").is_ok());
        assert!(model_judgments(&wrapped, &items(), "other").is_err());
    }
    #[test]
    fn rejects_unknown_duplicate_missing_changed_and_invalid_review_rows() {
        let original = source();
        for (key, value) in [
            ("item_id", json!("unknown")),
            ("phrase", json!("phrase valid")),
            ("puzzle", json!(2)),
            ("verdict", json!("maybe")),
            ("reasons", json!(["task_mismatch"])),
            ("explanation", json!(" ")),
            ("verification_needed", json!("false")),
        ] {
            let mut s = original.clone();
            s["judgments"][0][key] = value;
            assert!(model_judgments(&s, &items(), "hash").is_err(), "{key}");
        }
        let mut s = original.clone();
        let duplicate = s["judgments"][0].clone();
        s["judgments"].as_array_mut().unwrap().push(duplicate);
        assert!(model_judgments(&s, &items(), "hash").is_err());
        s["judgments"] = json!([]);
        assert!(model_judgments(&s, &items(), "hash").is_err());
        let mut s = original;
        s["summary"]["verdict_counts"]["useful"] = json!(2);
        assert!(model_judgments(&s, &items(), "hash").is_err());
    }
    #[test]
    fn packet_requires_unique_items_in_puzzles() {
        assert!(parse_packet("| item_a | phrase |").is_err());
        assert!(parse_packet("## Puzzle 1\n| item_a | phrase |\n| item_a | phrase |").is_err());
        assert_eq!(
            parse_packet("## Puzzle 1\n| item_a | phrase |").unwrap()["item_a"],
            (1, "phrase".into())
        );
    }

    #[test]
    fn human_mapping_requires_real_rows_and_exact_request_family_enrichment() {
        let packet = "## Puzzle 1\n| item_a | valid phrase |";
        let mapping = json!({"mapping":[{"item_id":"item_a","row_id":"row","case_id":"case"}]});
        let human = json!({"schema_version":1,"judgments":[{"id":"row","verdict":"unsure"}],
            "captures":[{"case_id":"case","request":{"text":"letters"},"rows":[{"id":"row","phrase":"valid phrase"}]}]});
        let mut workloads = json!({"workloads":[{"id":"case","request":{"text":"different"},"source_evidence":{"family_id":"family"}}]});
        let unknown = load_items(packet, &mapping, &human, Some(&workloads)).unwrap();
        assert!(unknown["item_a"].source_family.is_none());
        assert_eq!(unknown["item_a"].human, Verdict::Unsure);
        workloads["workloads"][0]["request"] = human["captures"][0]["request"].clone();
        assert_eq!(
            load_items(packet, &mapping, &human, Some(&workloads)).unwrap()["item_a"]
                .source_family
                .as_deref(),
            Some("family")
        );
        let mut bad = mapping.clone();
        bad["mapping"][0]["row_id"] = json!("unknown");
        assert!(load_items(packet, &bad, &human, None).is_err());
        let mut bad = human.clone();
        bad["judgments"][0]["verdict"] = Value::Null;
        assert!(load_items(packet, &mapping, &bad, None).is_err());
        assert!(
            load_items(
                "## Puzzle 1\n| item_a | changed phrase |",
                &mapping,
                &human,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn consensus_is_only_an_adjudication_queue_and_uncertain_humans_stay_separate() {
        let mut rows = items();
        rows.get_mut("item_a").unwrap().human = Verdict::NotUseful;
        let j: Judgment = serde_json::from_value(source()["judgments"][0].clone()).unwrap();
        for id in ["first", "second"] {
            rows.get_mut("item_a")
                .unwrap()
                .models
                .insert(id.into(), j.clone());
        }
        let ids = vec!["first".into(), "second".into()];
        assert_eq!(
            analyze(&rows, &ids)["consensus_against_definite_human"],
            json!(["item_a"])
        );
        assert_eq!(rows["item_a"].human, Verdict::NotUseful);
        rows.get_mut("item_a").unwrap().human = Verdict::Unsure;
        let result = analyze(&rows, &ids);
        assert_eq!(result["consensus_against_definite_human"], json!([]));
        assert_eq!(result["reviewers"]["first"]["definite_human"]["total"], 0);
        assert_eq!(
            result["reviewers"]["first"]["uncertain_human_model_votes"]["useful"],
            1
        );
    }

    #[test]
    fn command_preserves_inputs_and_rejects_hash_mismatch_and_output_aliases() {
        let scratch = crate::root().join(".codex/temp/xtask/review-model-tests");
        fs::create_dir_all(&scratch).unwrap();
        let temp = tempfile::tempdir_in(scratch).unwrap();
        let dir = temp.path();
        fs::write(
            dir.join("packet.md"),
            "## Puzzle 1\n| item_a | valid phrase |\n",
        )
        .unwrap();
        let human = json!({"schema_version":1,"judgments":[{"id":"row","verdict":"useful"}],"captures":[{"case_id":"case","rows":[{"id":"row","phrase":"valid phrase"}]}]});
        fs::write(dir.join("human.json"), human.to_string()).unwrap();
        let original_hash = digest(dir.join("human.json")).unwrap();
        let mapping = json!({"export_sha256":original_hash,"mapping":[{"item_id":"item_a","row_id":"row","case_id":"case"}]});
        fs::write(dir.join("mapping.json"), mapping.to_string()).unwrap();
        fs::write(dir.join("model.json"), source().to_string()).unwrap();
        let manifest = json!({"packet":"packet.md","mapping":"mapping.json","human_export":"human.json","reviewers":[{"id":"model","path":"model.json","reported_model":"unknown","provenance_note":"synthetic fixture"}]});
        fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
        let mut args = vec![
            "--manifest".into(),
            dir.join("manifest.json").to_str().unwrap().into(),
            "--output".into(),
            dir.join("report.json").to_str().unwrap().into(),
            "--readable".into(),
            dir.join("report.md").to_str().unwrap().into(),
        ];
        command(&args).unwrap();
        assert_eq!(
            read_json(dir.join("report.json")).unwrap()["analysis"]["items"],
            1
        );
        assert!(
            fs::read_to_string(dir.join("report.md"))
                .unwrap()
                .contains("valid phrase")
        );
        args[3] = dir.join("human.json").to_str().unwrap().into();
        assert!(
            command(&args)
                .unwrap_err()
                .to_string()
                .contains("aliases an input")
        );
        assert_eq!(digest(dir.join("human.json")).unwrap(), original_hash);
        args[3] = args[5].clone();
        assert!(
            command(&args)
                .unwrap_err()
                .to_string()
                .contains("Outputs must differ")
        );
        fs::write(dir.join("human.json"), "{}").unwrap();
        assert!(
            command(&args)
                .unwrap_err()
                .to_string()
                .contains("hash mismatch")
        );
    }
}
