use crate::{
    Result,
    quality::{self, Case, Cases},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn bag(v: &Value) -> Result<Vec<String>> {
    let mut words = v
        .as_array()
        .ok_or("Missing words")?
        .iter()
        .map(|w| w.as_str().map(String::from).ok_or("Invalid word"))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    words.sort();
    Ok(words)
}
pub fn evaluate(case: &Case, phase: &str, run: &Value, display_limit: usize) -> Result<Value> {
    if let Some(error) = run.get("error") {
        return Ok(json!({"error":error}));
    }
    let result = &run["result"];
    let signature = quality::signature(&case.source);
    if ["admission", "generation"].contains(&phase) {
        let bags = result["bags"].as_array().ok_or("Missing bags")?;
        for b in bags {
            if quality::signature(&bag(b)?.join(" ")) != signature {
                return Err("Non-anagram generated bag".into());
            }
        }
        let mut positions = BTreeMap::new();
        for gold in case.relevance.keys() {
            let goldbag = bag(&json!(
                quality::phrase_key(gold)
                    .split_whitespace()
                    .collect::<Vec<_>>()
            ))?;
            let position = bags
                .iter()
                .position(|b| bag(b).ok().as_ref() == Some(&goldbag))
                .map(|i| i + 1);
            positions.insert(gold, position);
        }
        return Ok(
            json!({"generated":result.get("generated").ok_or("Missing generated count")?,"stop":result.get("stop").ok_or("Missing stop")?,"gold_positions":positions}),
        );
    }
    if phase == "ordering" {
        return Ok(json!({"ablations":ablations(case,result)?}));
    }
    let rows = quality::rows(run, "desktop")?;
    for row in &rows {
        if quality::signature(&bag(&row["words"])?.join(" ")) != signature {
            return Err("Non-anagram candidate bag".into());
        }
        let phrase = quality::row_phrase(row);
        if !phrase.is_empty() && quality::signature(&phrase) != signature {
            return Err("Non-anagram displayed phrase".into());
        }
    }
    let generated = result["generated"]
        .as_u64()
        .ok_or("Missing generated count")?;
    let complete = rows.len() as u64 == generated;
    let buckets = result["buckets"].as_object().ok_or("Missing buckets")?;
    let mut sorted = buckets
        .iter()
        .map(|(k, v)| Ok((k.parse::<usize>()?, v.as_array().ok_or("Bad bucket")?)))
        .collect::<Result<Vec<_>>>()?;
    sorted.sort_by_key(|(n, _)| *n);
    let displayed = sorted
        .iter()
        .flat_map(|(_, v)| v.iter().take(display_limit))
        .collect::<Vec<_>>();
    let mut metrics = vec![];
    for gold in case.relevance.keys() {
        let words = quality::phrase_key(gold)
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        let wanted = bag(&json!(words))?;
        let row = rows
            .iter()
            .find(|r| bag(&r["words"]).ok().as_ref() == Some(&wanted));
        let chosen = row.map(quality::row_phrase);
        let bucket = buckets
            .get(&words.len().to_string())
            .and_then(Value::as_array);
        metrics.push(json!({"gold":gold,"gold_bag_in_pool":if row.is_some(){json!(true)}else if complete{json!(false)}else{Value::Null},"deep_selected":row.map(|r|r["deep"].clone()),"pre_score":row.map(|r|r["pre_score"].clone()),"chosen_order":chosen,"chosen_order_matches_gold":chosen.as_ref().filter(|s|!s.is_empty()).map(|s|quality::phrase_key(s)==quality::phrase_key(gold)),"bag_bucket_rank":bucket.and_then(|b|b.iter().position(|r|bag(&r["words"]).ok().as_ref()==Some(&wanted))).map(|i|i+1),"displayed_gold_rank":displayed.iter().position(|r|quality::phrase_key(&quality::row_phrase(r))==quality::phrase_key(gold)).map(|i|i+1)}));
    }
    Ok(
        json!({"all_rows_available":complete,"generated":generated,"deep_analyzed":result.get("deep_analyzed").ok_or("Missing deep count")?,"stop":result.get("generation_stop").ok_or("Missing stop")?,"blank_display_rows":rows.iter().filter(|r|quality::row_phrase(r).is_empty()).count(),"display_limit_projection":display_limit,"gold":metrics}),
    )
}
fn ablations(case: &Case, result: &Value) -> Result<Vec<Value>> {
    let orders = result["orders"]
        .as_array()
        .ok_or("Missing retained orders")?;
    let fields = [
        ("grammar_norm", 0.38),
        ("structure_norm", 0.44),
        ("valency_norm", 0.12),
        ("syntax_coverage", 0.06),
    ];
    let ties = [
        "structure_norm",
        "grammar_norm",
        "valency_norm",
        "syntax_coverage",
    ];
    for o in orders {
        if quality::signature(&bag(&o["order"])?.join(" ")) != quality::signature(&case.source) {
            return Err("Non-anagram retained order".into());
        }
        for name in [
            "objective",
            "grammar_norm",
            "structure_norm",
            "valency_norm",
            "syntax_coverage",
        ] {
            if o[name].as_f64().filter(|f| f.is_finite()).is_none() {
                return Err("Missing order feature".into());
            }
        }
    }
    let mut out = vec![];
    for omitted in std::iter::once(None).chain(fields.iter().map(|(k, _)| Some(*k))) {
        let score = |o: &Value| {
            if omitted.is_none() {
                o["objective"].as_f64().unwrap()
            } else {
                fields
                    .iter()
                    .filter(|(k, _)| Some(*k) != omitted)
                    .map(|(k, w)| w * o[k].as_f64().unwrap())
                    .sum()
            }
        };
        let mut ranked = orders.iter().collect::<Vec<_>>();
        // Lexical tie-break is ordered words, not a sorted bag.
        ranked.sort_by(|a, b| {
            let mut c = score(b).total_cmp(&score(a));
            for k in ties {
                if Some(k) != omitted {
                    c = c.then_with(|| b[k].as_f64().unwrap().total_cmp(&a[k].as_f64().unwrap()));
                }
            }
            c.then_with(|| a["order"].to_string().cmp(&b["order"].to_string()))
        });
        let phrase = |o: &Value| {
            o["order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let gold = case
            .relevance
            .keys()
            .map(|g| {
                (
                    g,
                    ranked
                        .iter()
                        .position(|r| quality::phrase_key(&phrase(r)) == quality::phrase_key(g))
                        .map(|i| i + 1),
                )
            })
            .collect::<BTreeMap<_, _>>();
        out.push(json!({"variant":omitted.map(|s|format!("without_{s}")).unwrap_or("native_objective".into()),"evaluated_orders":result.get("evaluated").ok_or("Missing evaluated count")?,"retained_orders":orders.len(),"complete_order_pool":Some(orders.len()as u64)==result["evaluated"].as_u64(),"top":ranked.first().map(|r|phrase(r)),"gold_ranks":gold}));
    }
    Ok(out)
}
pub fn annotate(case: &Case, phase: &str, run: &mut Value, display_limit: usize) {
    let value = evaluate(case, phase, run, display_limit).unwrap_or_else(|e| {
        let error = json!({"code":"invalid_diagnostic_result","message":e.to_string()});
        run["error"] = error.clone();
        json!({"error":error})
    });
    let mut summary = json!({"case_id":run["case_id"],"variant":run["variant"],"phase":phase,"trial":run["trial"],"wall_ms":run["wall_ms"]});
    summary
        .as_object_mut()
        .unwrap()
        .extend(value.as_object().unwrap().clone());
    run["summary"] = summary;
}
pub fn regrade(runs: &[Value], cases: &Cases) -> Result<Value> {
    let mut out = vec![];
    for r in runs {
        let id = r["case_id"].as_str().ok_or("Missing case")?;
        let owned;
        if let Some(word) = id.strip_prefix("word:") {
            owned = Case {
                id: id.into(),
                source: word.into(),
                split: "train".into(),
                relevance: BTreeMap::from([(word.into(), 3)]),
                provenance: "Single-word admission intervention".into(),
                negatives: vec![],
            };
        } else {
            owned = cases.get(id).ok_or("Unknown case")?.clone();
        }
        let phase = r["phase"].as_str().ok_or("Missing phase")?;
        let limit = r["summary"]["display_limit_projection"]
            .as_u64()
            .or_else(|| r["request"]["result_limit_per_group"].as_u64())
            .unwrap_or(10) as usize;
        let mut r = r.clone();
        annotate(&owned, phase, &mut r, limit);
        out.push(r["summary"].clone());
    }
    Ok(json!(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn case() -> Case {
        Case {
            id: "sanity".into(),
            source: "testinganagramsolver".into(),
            split: "train".into(),
            relevance: BTreeMap::from([("Testing Anagram Solver".into(), 3)]),
            provenance: "synthetic".into(),
            negatives: vec![],
        }
    }
    #[test]
    fn shortlist_and_display_are_distinct() {
        let run = json!({"result":{"generated":2,"deep_analyzed":1,"generation_stop":"candidate_cap","buckets":{"3":[{"words":["given","games","translator"],"deep":true,"display_phrase":"Given Games Translator","pre_score":90},{"words":["anagram","solver","testing"],"deep":false,"display_phrase":"","pre_score":50}]}}});
        let v = evaluate(&case(), "shortlist", &run, 1).unwrap();
        assert_eq!(v["gold"][0]["gold_bag_in_pool"], true);
        assert_eq!(v["gold"][0]["deep_selected"], false);
        assert_eq!(v["gold"][0]["bag_bucket_rank"], 2);
        assert!(v["gold"][0]["displayed_gold_rank"].is_null());
    }
    #[test]
    fn absence_is_unknown_when_truncated() {
        let r = json!({"result":{"generated":10,"deep_analyzed":0,"generation_stop":"candidate_cap","buckets":{}}});
        assert!(
            evaluate(&case(), "shortlist", &r, 10).unwrap()["gold"][0]["gold_bag_in_pool"]
                .is_null()
        );
        let mut r = r;
        r["result"]["generated"] = json!(0);
        assert_eq!(
            evaluate(&case(), "shortlist", &r, 10).unwrap()["gold"][0]["gold_bag_in_pool"],
            false
        );
    }
    #[test]
    fn invalid_diagnostic_retains_raw_result() {
        let mut r = json!({"case_id":"sanity","variant":"v","trial":0,"wall_ms":1,"result":{"bags":[["testing"]],"generated":1,"stop":"exhausted"}});
        let raw = r["result"].clone();
        annotate(&case(), "generation", &mut r, 10);
        assert_eq!(r["error"]["code"], "invalid_diagnostic_result");
        assert_eq!(raw, r["result"]);
    }
    #[test]
    fn ablation_removes_tie_influence_and_preserves_pool() {
        let o = json!({"order":["testing","anagram","solver"],"grammar_norm":1,"structure_norm":0,"valency_norm":0,"syntax_coverage":0,"objective":0.38});
        let mut b = o.clone();
        b["order"] = json!(["anagram", "solver", "testing"]);
        b["grammar_norm"] = json!(0);
        b["objective"] = json!(0);
        let v = ablations(&case(), &json!({"evaluated":6,"orders":[o,b]})).unwrap();
        assert_eq!(v[0]["top"], "testing anagram solver");
        assert_eq!(v[1]["top"], "anagram solver testing");
        assert_eq!(v[0]["complete_order_pool"], false);
    }
}
