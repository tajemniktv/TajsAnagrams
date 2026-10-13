//! Independent output invariants, using strings/maps rather than core Inventory,
//! lexicon, generation or request validators. No historical score/order target.
use crate::{Result, quality};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn words(v: &Value) -> Result<Vec<String>> {
    v.as_array()
        .ok_or("Expected word array")?
        .iter()
        .map(|w| Ok(quality::key(w.as_str().ok_or("Expected string word")?)))
        .collect()
}
fn multiset(words: &[String]) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for w in words {
        *m.entry(w.clone()).or_default() += 1;
    }
    m
}
pub fn output(request: &Value, result: &Value) -> Result<()> {
    let g = request.get("generation").unwrap_or(request);
    let mut available = quality::key(g["text"].as_str().ok_or("Missing request text")?)
        .chars()
        .collect::<Vec<_>>();
    if let Some(chunks) = g.get("subtract") {
        for c in words(chunks)? {
            for ch in c.chars() {
                let i = available
                    .iter()
                    .position(|v| *v == ch)
                    .ok_or("Unavailable subtract letters")?;
                available.remove(i);
            }
        }
    }
    available.sort();
    let expected = available.into_iter().collect::<String>();
    let required = multiset(
        &g.get("required")
            .map(words)
            .transpose()?
            .unwrap_or_default(),
    );
    let hints = g
        .get("hints")
        .map(words)
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let excluded = g
        .get("excluded")
        .map(words)
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let forbidden = quality::key(g["forbid_chars"].as_str().unwrap_or(""));
    let patterns = g["exclude_regex"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|p| p.as_str().ok_or("Invalid exclusion pattern"))
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();
    let regex = regex::RegexSetBuilder::new(patterns)
        .case_insensitive(true)
        .build()?;
    let bags = if let Some(bags) = result.get("bags") {
        bags.as_array()
            .ok_or("Missing bags")?
            .iter()
            .map(|b| Ok((words(b)?, None)))
            .collect::<Result<Vec<_>>>()?
    } else {
        quality::rows(&json!({"result":result}), "desktop")?
            .into_iter()
            .map(|r| Ok((words(&r["words"])?, Some(r))))
            .collect::<Result<Vec<_>>>()?
    };
    let mut seen = BTreeSet::new();
    for (bag, row) in bags {
        let counts = multiset(&bag);
        if bag.iter().any(String::is_empty)
            || quality::signature(&bag.join(" ")) != expected
            || !seen.insert(counts.clone())
        {
            return Err("Invalid letters, empty word or duplicate bag".into());
        }
        if required
            .iter()
            .any(|(w, n)| counts.get(w).copied().unwrap_or(0) < *n)
        {
            return Err("Missing required-word multiplicity".into());
        }
        if g["allow_repeat"] == false
            && counts
                .iter()
                .any(|(w, n)| n.saturating_sub(required.get(w).copied().unwrap_or(0)) > 1)
        {
            return Err("Invalid search-word repetition".into());
        }
        if bag.iter().any(|w| {
            excluded.contains(w) || w.chars().any(|c| forbidden.contains(c)) || regex.is_match(w)
        }) {
            return Err("Excluded word or character".into());
        }
        let hits = counts.keys().filter(|w| hints.contains(*w)).count();
        if !hints.is_empty()
            && (hits == 0
                || ((g["hint_mode"] == "exactly_one" || g["hint_mode"] == "exactly-one")
                    && hits != 1))
        {
            return Err("Unsatisfied hint constraint".into());
        }
        if (bag.len() as u64) < g["min_words"].as_u64().ok_or("Missing min_words")?
            || (bag.len() as u64) > g["max_words"].as_u64().ok_or("Missing max_words")?
        {
            return Err("Invalid word count".into());
        }
        if let Some(row) = row {
            let chosen = words(&row["best_order"])?;
            if !chosen.is_empty() && multiset(&chosen) != counts {
                return Err("Chosen order changes the word bag".into());
            }
            let phrase = quality::row_phrase(&row);
            if !phrase.is_empty() && quality::signature(&phrase) != expected {
                return Err("Displayed phrase changes letters".into());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_constraints_reject_mutated_results() {
        let g = json!({"text":"aabb","required":["ab"],"hints":["ab"],"excluded":["ba"],"forbid_chars":"","allow_repeat":false,"hint_mode":"any","min_words":2,"max_words":2});
        assert!(output(&g, &json!({"bags":[["ab","ab"]]})).is_ok());
        for bags in [
            json!([["ab", "ba"]]),
            json!([["ab"]]),
            json!([["a", "abb"]]),
            json!([["ab", "ab"], ["ab", "ab"]]),
        ] {
            assert!(output(&g, &json!({"bags":bags})).is_err());
        }
    }
    #[test]
    fn subtraction_and_exactly_one_are_independent() {
        let mut g = json!({"text":"aabbc","subtract":["c"],"hints":["aa","bb"],"excluded":[],"required":[],"allow_repeat":false,"hint_mode":"exactly-one","min_words":1,"max_words":3});
        assert!(output(&g, &json!({"bags":[["aa","bb"]]})).is_err());
        g["hint_mode"] = json!("any");
        assert!(output(&g, &json!({"bags":[["aa","bb"]]})).is_ok());
        g["exclude_regex"] = json!(["^a"]);
        assert!(output(&g, &json!({"bags":[["aa","bb"]]})).is_err());
    }
    #[test]
    fn chosen_order_cannot_substitute_different_words_with_same_letters() {
        let g = json!({"text":"ab","min_words":1,"max_words":2,"allow_repeat":false});
        let r =
            json!({"buckets":{"1":[{"words":["ab"],"best_order":["ba"],"display_phrase":"Ba"}]}});
        assert!(output(&g, &r).is_err());
    }
    #[test]
    fn frozen_native_baseline_is_valid_evidence_not_a_score_gate() {
        let baseline =
            crate::read_json(crate::root().join("tests/quality/native-baseline.v1.json")).unwrap();
        let observations = baseline["observations"].as_array().unwrap();
        assert_eq!(
            observations.len(),
            baseline["sample_count"].as_u64().unwrap() as usize
        );
        let mut failures = 0;
        for r in observations {
            if r.get("error").is_some() {
                failures += 1;
                continue;
            }
            let id = r["case_id"].as_str().unwrap();
            let request = &baseline["requests"][id];
            output(request, &json!({"buckets":r["buckets"]})).unwrap();
        }
        assert_eq!(failures, 2);
        assert_eq!(
            baseline["inputs"]["cli"]["sha256"].as_str().unwrap().len(),
            64
        );
    }
}
