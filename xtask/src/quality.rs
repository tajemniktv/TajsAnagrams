//! Independent evaluator: no production normalization, inventory or scoring calls.
use crate::{Args, Result, read_json, write_json};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Case {
    pub id: String,
    pub source: String,
    pub split: String,
    pub relevance: BTreeMap<String, u8>,
    pub provenance: String,
    #[serde(default)]
    pub negatives: Vec<String>,
}
pub type Cases = BTreeMap<String, Case>;
pub fn key(s: &str) -> String {
    s.nfkd()
        .filter(char::is_ascii)
        .map(|c| c.to_ascii_lowercase())
        .filter(char::is_ascii_lowercase)
        .collect()
}
pub fn signature(s: &str) -> String {
    let mut b = key(s).into_bytes();
    b.sort_unstable();
    String::from_utf8(b).unwrap()
}
pub fn phrase_key(s: &str) -> String {
    s.split_whitespace()
        .map(key)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn load_cases(path: impl AsRef<Path>) -> Result<Cases> {
    let v = read_json(path)?;
    if v["schema_version"] != 1 {
        return Err("Unsupported quality schema".into());
    }
    let mut cases = Cases::new();
    let mut bags = BTreeSet::new();
    for v in v["cases"].as_array().ok_or("Missing cases")? {
        let c: Case = serde_json::from_value(v.clone())?;
        let bag = signature(&c.source);
        if bag.is_empty()
            || c.id.is_empty()
            || !bags.insert(bag.clone())
            || cases.contains_key(&c.id)
        {
            return Err("Empty/duplicate case or leaking letter bag".into());
        }
        if !["train", "validation", "test"].contains(&c.split.as_str())
            || c.provenance.is_empty()
            || c.relevance.is_empty()
        {
            return Err("Missing split/provenance/relevance".into());
        }
        let mut phrases = BTreeSet::new();
        for (p, g) in &c.relevance {
            if signature(p) != bag || !(1..=3).contains(g) || !phrases.insert(phrase_key(p)) {
                return Err("Invalid/duplicate relevance phrase".into());
            }
        }
        if c.negatives.iter().any(|p| signature(p) != bag) {
            return Err("Invalid diagnostic alternative".into());
        }
        cases.insert(c.id.clone(), c);
    }
    if cases.is_empty() {
        return Err("Empty case dataset".into());
    }
    Ok(cases)
}
pub fn row_phrase(row: &Value) -> String {
    for name in ["phrase", "display_phrase"] {
        if let Some(s) = row[name].as_str().filter(|s| !s.is_empty()) {
            return s.into();
        }
    }
    row["best_order"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}
pub fn rows(run: &Value, ranking: &str) -> Result<Vec<Value>> {
    if !["desktop", "final_score"].contains(&ranking) {
        return Err("Unsupported ranking".into());
    }
    if let Some(rows) = run.get("rows") {
        return Ok(rows.as_array().ok_or("Rows must be an array")?.clone());
    }
    let buckets = run["result"]["buckets"]
        .as_object()
        .ok_or("Missing native buckets")?;
    let mut buckets = buckets
        .iter()
        .map(|(k, v)| Ok((k.parse::<u32>()?, v.as_array().ok_or("Invalid bucket")?)))
        .collect::<Result<Vec<_>>>()?;
    buckets.sort_by_key(|b| b.0);
    let mut rows = buckets
        .into_iter()
        .flat_map(|(_, v)| v.iter().cloned())
        .collect::<Vec<_>>();
    if ranking == "final_score" {
        for r in &rows {
            if r["final"].as_f64().filter(|f| f.is_finite()).is_none() {
                return Err("Invalid final score".into());
            }
        }
        rows.sort_by(|a, b| {
            b["final"]
                .as_f64()
                .unwrap()
                .total_cmp(&a["final"].as_f64().unwrap())
                .then_with(|| {
                    a["display_phrase"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(b["display_phrase"].as_str().unwrap_or(""))
                })
        });
    }
    Ok(rows)
}
const METRICS: [&str; 6] = [
    "recall_at_1",
    "recall_at_5",
    "recall_at_10",
    "mrr",
    "ndcg_at_5",
    "ndcg_at_10",
];
pub fn metrics(case: &Case, rows: &[Value]) -> Result<Value> {
    let labels: BTreeMap<_, _> = case
        .relevance
        .iter()
        .map(|(p, g)| (phrase_key(p), *g))
        .collect();
    let mut seen = BTreeSet::new();
    let mut grades = Vec::new();
    let sig = signature(&case.source);
    for row in rows {
        let p = row_phrase(row);
        if p.is_empty()
            && row["deep"] == false
            && row["words"].as_array().is_some_and(|a| !a.is_empty())
        {
            let words = row["words"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_str().ok_or("Non-string word"))
                .collect::<std::result::Result<Vec<_>, _>>()?
                .join(" ");
            if signature(&words) != sig {
                return Err("Non-anagram shallow bag".into());
            }
            grades.push(0);
            continue;
        }
        if signature(&p) != sig {
            return Err(format!("Non-anagram result: {} / {p}", case.id).into());
        }
        let p = phrase_key(&p);
        if seen.insert(p.clone()) {
            grades.push(*labels.get(&p).unwrap_or(&0));
        }
    }
    let hits = grades
        .iter()
        .enumerate()
        .filter(|(_, g)| **g > 0)
        .map(|(i, _)| i + 1)
        .collect::<Vec<_>>();
    let mut ideal = case.relevance.values().copied().collect::<Vec<_>>();
    ideal.sort_by(|a, b| b.cmp(a));
    let dcg = |v: &[u8], n: usize| {
        v.iter()
            .take(n)
            .enumerate()
            .map(|(i, g)| ((1u32 << g) - 1) as f64 / ((i + 2) as f64).log2())
            .sum::<f64>()
    };
    Ok(
        json!({"recall_at_1":hits.iter().filter(|r|**r<=1).count() as f64/ideal.len() as f64,"recall_at_5":hits.iter().filter(|r|**r<=5).count() as f64/ideal.len() as f64,"recall_at_10":hits.iter().filter(|r|**r<=10).count() as f64/ideal.len() as f64,"mrr":hits.first().map_or(0.,|r|1. / *r as f64),"ndcg_at_5":dcg(&grades,5)/dcg(&ideal,5),"ndcg_at_10":dcg(&grades,10)/dcg(&ideal,10)}),
    )
}
pub fn state(run: &Value) -> Result<&str> {
    let s = if let Some(s) = run.get("cache_state") {
        s.as_str().ok_or("Invalid cache state")?
    } else {
        match run.get("warm") {
            None => "unspecified",
            Some(Value::Bool(true)) => "disk_cache_reuse",
            Some(Value::Bool(false)) => "disk_cache_empty",
            _ => return Err("Warm must be boolean".into()),
        }
    };
    if ![
        "disk_cache_empty",
        "disk_cache_reuse",
        "in_process_warm",
        "uncached",
        "unspecified",
    ]
    .contains(&s)
    {
        return Err("Unsupported cache state".into());
    }
    Ok(s)
}
pub fn wall(run: &Value) -> Result<f64> {
    run["wall_ms"]
        .as_f64()
        .filter(|f| f.is_finite() && *f >= 0.)
        .ok_or_else(|| "Invalid wall_ms".into())
}
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v[k].as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Missing {k}").into())
}
pub fn latest(runs: &[Value]) -> Result<Vec<&Value>> {
    let mut m = BTreeMap::new();
    for run in runs {
        let trial = match run.get("trial") {
            None => 0,
            Some(v) => v.as_u64().ok_or("Invalid trial")?,
        };
        let id = (
            text(run, "variant")?,
            state(run)?,
            text(run, "case_id")?,
            trial,
        );
        let w = wall(run)?;
        if m.get(&id)
            .is_none_or(|prev: &&Value| wall(prev).unwrap() <= w)
        {
            m.insert(id, run);
        }
    }
    Ok(m.into_values().collect())
}
fn success(r: &Value) -> bool {
    r.get("error").is_none()
}
fn averages(values: &[Value]) -> Value {
    let mut v = Map::new();
    for name in METRICS {
        v.insert(
            name.into(),
            if values.is_empty() {
                Value::Null
            } else {
                json!(
                    values
                        .iter()
                        .map(|v| v[name].as_f64().unwrap())
                        .sum::<f64>()
                        / values.len() as f64
                )
            },
        );
    }
    Value::Object(v)
}
pub fn percentile(values: &[f64], q: f64) -> Value {
    if values.is_empty() {
        return Value::Null;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let x = (v.len() - 1) as f64 * q;
    let lo = x.floor() as usize;
    let hi = x.ceil() as usize;
    json!(v[lo] + (v[hi] - v[lo]) * (x - lo as f64))
}
fn case<'a>(cases: &'a Cases, r: &Value) -> Result<&'a Case> {
    cases
        .get(text(r, "case_id")?)
        .ok_or_else(|| "Unknown case ID".into())
}
fn attempt_metrics(cases: &Cases, r: &Value, ranking: &str) -> Result<Value> {
    metrics(
        case(cases, r)?,
        &if success(r) {
            rows(r, ranking)?
        } else {
            vec![]
        },
    )
}
fn groups(runs: &[Value], cases: &Cases, ranking: &str) -> Result<Vec<Value>> {
    let mut groups: BTreeMap<_, Vec<(&Value, Value)>> = BTreeMap::new();
    for r in latest(runs)? {
        let c = case(cases, r)?;
        let m = attempt_metrics(cases, r, ranking)?;
        let n = c
            .relevance
            .keys()
            .next()
            .unwrap()
            .split_whitespace()
            .count();
        let l = key(&c.source).len();
        for d in [
            "all".into(),
            format!("words:{}{}", n.min(5), if n >= 5 { "+" } else { "" }),
            format!("letters:{}-{}", l / 5 * 5, l / 5 * 5 + 4),
            c.split.clone(),
        ] {
            groups
                .entry((text(r, "variant")?.to_owned(), d, state(r)?.to_owned()))
                .or_default()
                .push((r, m.clone()));
        }
    }
    let mut out = Vec::new();
    for ((variant, dimension, state), samples) in groups {
        let good = samples
            .iter()
            .filter(|(r, _)| success(r))
            .collect::<Vec<_>>();
        let times = good
            .iter()
            .map(|(r, _)| wall(r).unwrap())
            .collect::<Vec<_>>();
        let all_times = samples
            .iter()
            .map(|(r, _)| wall(r).unwrap())
            .collect::<Vec<_>>();
        let mut v = json!({"variant":variant,"dimension":dimension,"cache_state":state,"ranking":ranking,"attempts":samples.len(),"attempted_cases":samples.iter().map(|(r,_)|r["case_id"].as_str().unwrap()).collect::<BTreeSet<_>>().len(),"completed":good.len(),"failed":samples.len()-good.len(),"coverage":good.len()as f64/samples.len()as f64,"successful_metrics":averages(&good.iter().map(|(_,m)|m.clone()).collect::<Vec<_>>()),"completed_p50_ms":percentile(&times,0.5),"completed_p95_ms":percentile(&times,0.95),"attempt_p50_ms":percentile(&all_times,0.5),"attempt_p95_ms":percentile(&all_times,0.95)});
        v.as_object_mut().unwrap().extend(
            averages(&samples.iter().map(|(_, m)| m.clone()).collect::<Vec<_>>())
                .as_object()
                .unwrap()
                .clone(),
        );
        for name in [
            "generated",
            "expanded",
            "deep_analyzed",
            "orders_evaluated",
            "peak_rss_bytes",
            "persistent_bytes",
        ] {
            let values = good
                .iter()
                .map(|(r, _)| {
                    r.get(name)
                        .or_else(|| r["result"].get(name))
                        .and_then(Value::as_f64)
                })
                .collect::<Option<Vec<_>>>();
            v[name] = values.filter(|v| !v.is_empty()).map_or(Value::Null, |v| {
                json!(v.iter().sum::<f64>() / v.len() as f64)
            });
        }
        out.push(v);
    }
    Ok(out)
}
fn budgets(runs: &[Value], cases: &Cases, ranking: &str) -> Result<Vec<Value>> {
    let mut groups: BTreeMap<_, Vec<Value>> = BTreeMap::new();
    for r in runs {
        groups
            .entry((text(r, "variant")?.to_owned(), state(r)?.to_owned()))
            .or_default()
            .push(r.clone());
    }
    let mut out = Vec::new();
    for ((variant, state), group) in groups {
        let attempts = latest(&group)?;
        for budget in [50, 200, 1000, 5000] {
            let observed = group
                .iter()
                .filter(|r| wall(r).unwrap() <= budget as f64)
                .cloned()
                .collect::<Vec<_>>();
            let delivered = observed
                .iter()
                .filter(|r| success(r))
                .cloned()
                .collect::<Vec<_>>();
            let delivered = latest(&delivered)?;
            let observed = latest(&observed)?;
            let values = delivered
                .iter()
                .map(|r| attempt_metrics(cases, r, ranking))
                .collect::<Result<Vec<_>>>()?;
            let mut delivery = values.clone();
            for _ in delivered.len()..attempts.len() {
                delivery.push(metrics(case(cases, attempts[0])?, &[])?);
            }
            let mut v = json!({"variant":variant,"cache_state":state,"ranking":ranking,"budget_ms":budget,"attempts":attempts.len(),"attempted_cases":attempts.iter().map(|r|r["case_id"].as_str().unwrap()).collect::<BTreeSet<_>>().len(),"covered_cases":delivered.iter().map(|r|r["case_id"].as_str().unwrap()).collect::<BTreeSet<_>>().len(),"covered_attempts":delivered.len(),"failed_attempts":observed.iter().filter(|r|!success(r)).count(),"unobserved_attempts":attempts.len()-observed.len(),"delivery_metrics":averages(&delivery)});
            v.as_object_mut()
                .unwrap()
                .extend(averages(&values).as_object().unwrap().clone());
            out.push(v);
        }
    }
    Ok(out)
}
pub fn read_runs(path: impl AsRef<Path>) -> Result<Vec<Value>> {
    fs::read_to_string(path)?
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|s| Ok(serde_json::from_str(s)?))
        .collect()
}
fn labels(path: Option<&str>, field: &str, cases: &Cases) -> Result<Vec<Value>> {
    let Some(path) = path else { return Ok(vec![]) };
    let v = read_json(path)?;
    if v["schema_version"] != 1 {
        return Err("Unsupported judgment schema".into());
    }
    let rows = v[field].as_array().ok_or("Missing judgments")?.clone();
    let mut seen = BTreeSet::new();
    for r in &rows {
        let c = case(cases, r)?;
        let sig = signature(&c.source);
        if field == "judgments" {
            let p = text(r, "phrase")?;
            if !r["useful"].is_boolean()
                || text(r, "provenance")?.is_empty()
                || signature(p) != sig
                || !seen.insert((c.id.clone(), phrase_key(p)))
            {
                return Err("Invalid/duplicate usefulness label".into());
            }
        } else {
            let a = text(r, "left")?;
            let b = text(r, "right")?;
            if !["left", "right"].contains(&text(r, "preferred")?)
                || a == b
                || r["presentation_seed"].is_null()
                || r["presentation_seed"] == 0
                || signature(a) != sig
                || signature(b) != sig
            {
                return Err("Invalid preference".into());
            }
        }
    }
    Ok(rows)
}
pub fn report(
    runs: &[Value],
    cases: &Cases,
    rankings: &[&str],
    preferences: &[Value],
    usefulness: Option<&[Value]>,
) -> Result<Value> {
    let attempts = latest(runs)?;
    for r in runs {
        case(cases, r)?;
    }
    let strata = attempts
        .iter()
        .map(|r| Ok((text(r, "variant")?.to_owned(), state(r)?.to_owned())))
        .collect::<Result<BTreeSet<_>>>()?;
    let mut unattempted = Vec::new();
    for (v, s) in &strata {
        let attempted = attempts
            .iter()
            .filter(|r| r["variant"] == *v && state(r).unwrap() == s)
            .map(|r| r["case_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        unattempted.push(json!({"variant":v,"cache_state":s,"case_ids":cases.keys().filter(|k|!attempted.contains(k.as_str())).collect::<Vec<_>>()}));
    }
    let failures = runs
        .iter()
        .filter(|r| !success(r))
        .map(|r| {
            let mut m = Map::new();
            for k in [
                "case_id",
                "variant",
                "cache_state",
                "warm",
                "trial",
                "wall_ms",
                "error",
            ] {
                if let Some(v) = r.get(k) {
                    m.insert(k.into(), v.clone());
                }
            }
            Value::Object(m)
        })
        .collect::<Vec<_>>();
    let mut group_rows = Vec::new();
    let mut budget_rows = Vec::new();
    let mut prefs = Vec::new();
    let mut useful = Vec::new();
    for ranking in rankings {
        group_rows.extend(groups(runs, cases, ranking)?);
        budget_rows.extend(budgets(runs, cases, ranking)?);
        for (variant, state_name) in &strata {
            let group = attempts
                .iter()
                .filter(|r| r["variant"] == *variant && state(r).unwrap() == state_name)
                .copied()
                .collect::<Vec<_>>();
            let (mut correct, mut total) = (0, 0);
            for p in preferences {
                for r in group
                    .iter()
                    .filter(|r| success(r) && r["case_id"] == p["case_id"])
                {
                    let mut ranks = BTreeMap::new();
                    for (i, row) in rows(r, ranking)?.iter().enumerate() {
                        let phrase = row_phrase(row);
                        if !phrase.is_empty() {
                            ranks.entry(phrase_key(&phrase)).or_insert(i);
                        }
                    }
                    let left = ranks
                        .get(&phrase_key(text(p, "left")?))
                        .copied()
                        .unwrap_or(usize::MAX);
                    let right = ranks
                        .get(&phrase_key(text(p, "right")?))
                        .copied()
                        .unwrap_or(usize::MAX);
                    if left != right {
                        total += 1;
                        if (left < right) == (p["preferred"] == "left") {
                            correct += 1;
                        }
                    }
                }
            }
            prefs.push(json!({"variant":variant,"cache_state":state_name,"ranking":ranking,"judgments":preferences.len(),"evaluated":total,"accuracy":if total==0{Value::Null}else{json!(correct as f64/total as f64)}}));
            if let Some(labels) = usefulness {
                let labels = labels
                    .iter()
                    .map(|l| {
                        Ok((
                            (
                                text(l, "case_id")?.to_owned(),
                                phrase_key(text(l, "phrase")?),
                            ),
                            l["useful"].as_bool().ok_or("Invalid useful label")?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                let judged = group
                    .iter()
                    .filter(|r| labels.keys().any(|(c, _)| r["case_id"] == *c))
                    .copied()
                    .collect::<Vec<_>>();
                if !judged.is_empty() {
                    for n in [1, 5, 10] {
                        let (mut yes, mut no, mut unknown) = (0, 0, 0);
                        for r in judged.iter().filter(|r| success(r)) {
                            for row in rows(r, ranking)?.iter().take(n) {
                                match labels.get(&(
                                    text(r, "case_id")?.into(),
                                    phrase_key(&row_phrase(row)),
                                )) {
                                    Some(true) => yes += 1,
                                    Some(false) => no += 1,
                                    None => unknown += 1,
                                }
                            }
                        }
                        useful.push(json!({"variant":variant,"cache_state":state_name,"ranking":ranking,"top_n":n,"judged_case_ids":judged.iter().map(|r|text(r,"case_id").unwrap()).collect::<BTreeSet<_>>(),"unique_phrase_judgments":labels.len(),"attempts":judged.len(),"completed":judged.iter().filter(|r|success(r)).count(),"useful_rows":yes,"not_useful_rows":no,"unreviewed_rows":unknown}));
                    }
                }
            }
        }
    }
    let mut out = json!({"schema_version":2,"cases":cases.len(),"snapshots":runs.len(),"attempts":attempts.len(),"failures":failures,"unattempted_cases":unattempted,"groups":group_rows,"quality_at_budgets":budget_rows,"pairwise_preference":prefs});
    if usefulness.is_some() {
        out["standalone_usefulness"] = json!(useful);
    }
    Ok(out)
}
pub fn command(args: &[String]) -> Result<()> {
    if args == ["--help"] {
        println!(
            "quality --runs FILE [--cases FILE --ranking desktop|final_score|both --preferences FILE --usefulness FILE --output FILE]\nquality --stages FILE [--cases FILE --output FILE] regrades recorded native stage captures"
        );
        return Ok(());
    }
    let a = Args::parse(
        args,
        &[
            "runs",
            "cases",
            "ranking",
            "preferences",
            "usefulness",
            "output",
            "stages",
        ],
        &[],
    )?;
    let cases = load_cases(a.value("cases", "tests/quality/cases.v1.json"))?;
    if let Some(path) = a.get("stages") {
        if a.has("runs") {
            return Err("Choose --stages or --runs".into());
        }
        return write_json(
            a.get("output"),
            &crate::stages::regrade(&read_runs(path)?, &cases)?,
        );
    }
    let runs = read_runs(a.required("runs")?)?;
    let ranking = a.value("ranking", "desktop");
    let rankings = match ranking {
        "both" => vec!["desktop", "final_score"],
        "desktop" | "final_score" => vec![ranking],
        _ => return Err("Invalid ranking".into()),
    };
    let prefs = labels(
        Some(a.value("preferences", "tests/quality/preferences.v1.json")),
        "comparisons",
        &cases,
    )?;
    let useful = a
        .get("usefulness")
        .map(|p| labels(Some(p), "judgments", &cases))
        .transpose()?;
    write_json(
        a.get("output"),
        &report(&runs, &cases, &rankings, &prefs, useful.as_deref())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cases() -> Cases {
        load_cases(crate::root().join("tests/quality/cases.v1.json")).unwrap()
    }
    fn run(phrase: &str, wall: f64) -> Value {
        json!({"case_id":"testing_anagram_solver","variant":"v","wall_ms":wall,"cache_state":"disk_cache_empty","rows":[{"phrase":phrase}]})
    }
    fn report_for(runs: &[Value]) -> Value {
        report(runs, &cases(), &["desktop"], &[], None).unwrap()
    }
    fn all(v: &Value) -> &Value {
        v["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["dimension"] == "all")
            .unwrap()
    }
    #[test]
    fn ranking_order_and_exact_letters() {
        let cases = cases();
        let c = &cases["testing_anagram_solver"];
        let v = metrics(
            c,
            &[
                json!({"phrase":"Solver Testing Anagram"}),
                json!({"phrase":"Testing Anagram Solver"}),
            ],
        )
        .unwrap();
        assert_eq!(v["recall_at_1"], 0.0);
        assert_eq!(v["mrr"], 0.5);
        assert!(metrics(c, &[json!({"phrase":"Testing Solver"})]).is_err());
        assert_eq!(key("Łß ÉX"), "ex");
    }
    #[test]
    fn desktop_not_global_score() {
        let good = json!({"display_phrase":"Testing Anagram Solver","final":1});
        let bad = json!({"display_phrase":"A German Signs To Travel","final":100});
        let r = json!({"result":{"buckets":{"5":[bad],"3":[good]}}});
        assert_eq!(rows(&r, "desktop").unwrap()[0], good);
        assert_eq!(rows(&r, "final_score").unwrap()[0], bad);
    }
    #[test]
    fn failures_remain_in_primary_denominator() {
        let a = run("Testing Anagram Solver", 100.);
        let b = json!({"case_id":"knowledge_power","variant":"v","wall_ms":10000,"cache_state":"disk_cache_empty","error":{"code":"timed_out"}});
        let v = report_for(&[a, b]);
        let g = all(&v);
        assert_eq!(g["attempts"], 2);
        assert_eq!(g["completed"], 1);
        assert_eq!(g["recall_at_1"], 0.5);
        assert_eq!(g["successful_metrics"]["recall_at_1"], 1.);
        assert_eq!(g["completed_p50_ms"], 100.);
        assert!(g["attempt_p95_ms"].as_f64().unwrap() > 9000.);
    }
    #[test]
    fn snapshots_not_attempts_partial_delivery_survives_failure() {
        let mut b = run("Testing Anagram Solver", 150.);
        b["error"] = json!({"code":"timed_out"});
        let v = report_for(&[run("Testing Anagram Solver", 40.), b]);
        assert_eq!(all(&v)["attempts"], 1);
        assert_eq!(all(&v)["recall_at_1"], 0.);
        let b = &v["quality_at_budgets"][1];
        assert_eq!(b["covered_attempts"], 1);
        assert_eq!(b["failed_attempts"], 1);
        assert_eq!(b["delivery_metrics"]["recall_at_1"], 1.);
    }
    #[test]
    fn budgets_never_invent_intermediate_rankings() {
        let v = report_for(&[
            run("Solver Testing Anagram", 40.),
            run("Testing Anagram Solver", 150.),
        ]);
        assert_eq!(v["quality_at_budgets"][0]["recall_at_1"], 0.);
        assert_eq!(v["quality_at_budgets"][1]["recall_at_1"], 1.);
        let v = report_for(&[run("Testing Anagram Solver", 100.)]);
        assert!(v["quality_at_budgets"][0]["recall_at_1"].is_null());
        assert_eq!(
            v["quality_at_budgets"][0]["delivery_metrics"]["recall_at_1"],
            0.
        );
    }
    #[test]
    fn cache_and_trials_separate_unique_cases() {
        let mut runs = vec![];
        for state in ["disk_cache_empty", "disk_cache_reuse"] {
            for trial in [0, 1] {
                let mut r = run(
                    "Testing Anagram Solver",
                    if state == "disk_cache_empty" {
                        100.
                    } else {
                        40.
                    },
                );
                r["trial"] = json!(trial);
                r["cache_state"] = json!(state);
                runs.push(r);
            }
        }
        let v = report_for(&runs);
        let g = v["quality_at_budgets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["cache_state"] == "disk_cache_reuse" && g["budget_ms"] == 50)
            .unwrap();
        assert_eq!(g["covered_cases"], 1);
        assert_eq!(g["covered_attempts"], 2);
    }
    #[test]
    fn blank_shallow_rows_keep_positions_without_inventing_order() {
        let blank = json!({"display_phrase":"","best_order":[],"deep":false,"words":["anagram","solver","testing"]});
        let c = cases();
        let c = &c["testing_anagram_solver"];
        let v = metrics(
            c,
            &[
                blank.clone(),
                blank.clone(),
                json!({"phrase":"Testing Anagram Solver"}),
            ],
        )
        .unwrap();
        assert_eq!(v["mrr"], 1. / 3.);
        let mut b = blank.clone();
        b["words"] = json!(["testing"]);
        assert!(metrics(c, &[b]).is_err());
        let mut b = blank;
        b["deep"] = json!(true);
        assert!(metrics(c, &[b]).is_err());
    }
    #[test]
    fn duplicate_phrases_deduplicated() {
        let c = cases();
        let v = metrics(
            &c["testing_anagram_solver"],
            &[
                json!({"phrase":"Testing Anagram Solver"}),
                json!({"phrase":"TESTING ANAGRAM SOLVER"}),
            ],
        )
        .unwrap();
        assert_eq!(v["recall_at_10"], 1.);
    }
    #[test]
    fn invalid_observation_metadata_rejected() {
        for bad in [json!(-1), json!(true), json!("nan"), Value::Null] {
            let mut r = run("Testing Anagram Solver", 1.);
            r["wall_ms"] = bad;
            assert!(latest(&[r]).is_err());
        }
        let mut r = run("Testing Anagram Solver", 1.);
        r["trial"] = json!(-1);
        assert!(latest(&[r]).is_err());
        let mut r = run("Testing Anagram Solver", 1.);
        r["cache_state"] = json!("warmest");
        assert!(latest(&[r]).is_err());
    }
    #[test]
    fn useful_alternatives_not_exact_target_or_unknown_negatives() {
        let c = cases();
        let labels = labels(
            Some(
                crate::root()
                    .join("tests/quality/usefulness.v1.json")
                    .to_str()
                    .unwrap(),
            ),
            "judgments",
            &c,
        )
        .unwrap();
        let r = json!({"case_id":"testing_anagram_solver","variant":"v","wall_ms":1,"cache_state":"uncached","rows":[{"phrase":"Several Migrants Tango"},{"phrase":"Solver Testing Anagram"}]});
        let v = report(&[r], &c, &["desktop"], &[], Some(&labels)).unwrap();
        assert_eq!(all(&v)["recall_at_1"], 0.);
        assert_eq!(v["standalone_usefulness"][0]["useful_rows"], 1);
        assert_eq!(v["standalone_usefulness"][1]["unreviewed_rows"], 1);
        assert_eq!(v["standalone_usefulness"][1]["not_useful_rows"], 0);
    }
    #[test]
    fn preferences_do_not_select_fastest_cache_state() {
        let c = cases();
        let mut a = run("Testing Anagram Solver", 100.);
        a["rows"] =
            json!([{"phrase":"Testing Anagram Solver"},{"phrase":"Solver Testing Anagram"}]);
        let mut b = a.clone();
        b["cache_state"] = json!("disk_cache_reuse");
        b["wall_ms"] = json!(10);
        b["rows"] =
            json!([{"phrase":"Solver Testing Anagram"},{"phrase":"Testing Anagram Solver"}]);
        let prefs = vec![
            json!({"case_id":"testing_anagram_solver","left":"Testing Anagram Solver","right":"Solver Testing Anagram","preferred":"left","presentation_seed":13}),
        ];
        let v = report(&[a, b], &c, &["desktop"], &prefs, None).unwrap();
        assert_eq!(v["pairwise_preference"][0]["accuracy"], 1.);
        assert_eq!(v["pairwise_preference"][1]["accuracy"], 0.);
    }
    #[test]
    fn missing_cases_reported_not_fabricated() {
        let v = report_for(&[run("Testing Anagram Solver", 1.)]);
        assert_eq!(
            v["unattempted_cases"][0]["case_ids"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(v["attempts"], 1);
    }
    #[test]
    fn expanded_cases_are_bag_disjoint_and_curator_not_user_labels() {
        let cases = load_cases(crate::root().join("tests/quality/cases.v2.json")).unwrap();
        assert_eq!(cases.len(), 19);
        assert_eq!(cases["river_lights"].split, "validation");
        assert!(
            cases["river_lights"]
                .provenance
                .contains("not a user judgment")
        );
    }
    #[test]
    fn malformed_or_leaking_cases_and_judgments_fail() {
        let scratch = crate::root().join(".codex/temp/xtask/quality-tests");
        fs::create_dir_all(&scratch).unwrap();
        let temp = tempfile::tempdir_in(scratch).unwrap();
        let path = temp.path().join("cases.json");
        let base = json!({"schema_version":1,"cases":[{"id":"a","source":"ab","split":"train","relevance":{"ab":3},"provenance":"synthetic"}]});
        for bad in [json!(true), json!(0), json!(4), json!(1.5)] {
            let mut v = base.clone();
            v["cases"][0]["relevance"]["ab"] = bad;
            fs::write(&path, v.to_string()).unwrap();
            assert!(load_cases(&path).is_err());
        }
        let mut v = base.clone();
        let mut duplicate = v["cases"][0].clone();
        duplicate["id"] = json!("other");
        duplicate["source"] = json!("ba");
        v["cases"].as_array_mut().unwrap().push(duplicate);
        fs::write(&path, v.to_string()).unwrap();
        assert!(load_cases(&path).is_err());
        fs::write(&path, base.to_string()).unwrap();
        let cases = load_cases(&path).unwrap();
        let label = temp.path().join("labels.json");
        fs::write(&label,json!({"schema_version":1,"judgments":[{"case_id":"a","phrase":"ab","useful":true,"provenance":"human"},{"case_id":"a","phrase":"AB","useful":false,"provenance":"human"}]}).to_string()).unwrap();
        assert!(labels(label.to_str(), "judgments", &cases).is_err());
    }
}
