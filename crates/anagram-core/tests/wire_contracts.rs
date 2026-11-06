use anagram_core::{contracts::JobStatus, request::GenerateRequest, solve::Request};
use serde_json::Value;

#[test]
fn request_identity_normalizes_numeric_spelling_without_weakening_the_schema() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../../contracts/fixtures.json")).unwrap();
    let mut a = fixtures
        .iter()
        .find(|f| f["schema"] == "SolveRequest" && f["valid"] == true)
        .unwrap()["value"]
        .clone();
    a["phrase_bonus_max"] = serde_json::json!(6.0);
    let mut b = a.clone();
    b["phrase_bonus_max"] = serde_json::json!(6);
    assert_ne!(a, b);
    assert!(anagram_core::solve::same_request(&a, &b));
    b["workers"] = serde_json::json!(1);
    b["refine"] = serde_json::json!(false);
    assert!(anagram_core::solve::same_request(&a, &b));
    for (key, value) in [
        ("phrase_bonus_max", serde_json::json!(5)),
        ("workers", serde_json::json!(2)),
        ("unknown", serde_json::json!(1)),
    ] {
        let mut bad = b.clone();
        bad[key] = value;
        assert!(!anagram_core::solve::same_request(&a, &bad), "{key}");
    }
    b["generation"]["text"] = serde_json::json!("changed");
    assert!(!anagram_core::solve::same_request(&a, &b));
    assert!(!anagram_core::solve::same_request(
        &serde_json::json!({}),
        &serde_json::json!({})
    ));
}

#[test]
fn shared_request_and_progress_fixtures_match_rust_deserialization() {
    let fixtures: Vec<Value> =
        serde_json::from_str(include_str!("../../../contracts/fixtures.json")).unwrap();
    let mut checked = std::collections::BTreeMap::new();
    for fixture in fixtures {
        let value = fixture["value"].clone();
        let accepted = match fixture["schema"].as_str().unwrap() {
            "WordleRequest" => {
                serde_json::from_value::<anagram_core::wordle::AnalysisRequest>(value).is_ok()
            }
            "DeploymentLimits" => {
                serde_json::from_value::<anagram_core::policy::DeploymentLimits>(value)
                    .is_ok_and(|limits| limits.validate().is_ok())
            }
            "GenerateRequest" => serde_json::from_value::<GenerateRequest>(value)
                .is_ok_and(|request| anagram_core::request::validate(&request).is_ok()),
            "SolveRequest" => serde_json::from_value::<Request>(value)
                .is_ok_and(|request| anagram_core::solve::validate(&request).is_ok()),
            "JobStatus" => serde_json::from_value::<JobStatus>(value)
                .is_ok_and(|status| status.validate().is_ok()),
            // Output-only types are validated from actual CLI serialization by
            // the independent AJV contract consumer, not deserialized by the engine.
            _ => continue,
        };
        assert_eq!(accepted, fixture["valid"].as_bool().unwrap(), "{fixture}");
        *checked
            .entry(fixture["schema"].as_str().unwrap().to_owned())
            .or_insert(0usize) += 1;
    }
    for (schema, count) in [
        ("DeploymentLimits", 6),
        ("GenerateRequest", 5),
        ("SolveRequest", 1),
        ("JobStatus", 1),
        ("WordleRequest", 7),
    ] {
        assert_eq!(
            checked.get(schema),
            Some(&count),
            "Missing fixture coverage for {schema}"
        );
    }
}
