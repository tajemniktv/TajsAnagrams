use anagram_core::{
    control::Control,
    request::{GenerateRequest, generate_controlled, validate},
};
use serde_json::json;

fn request(subtract: serde_json::Value) -> GenerateRequest {
    serde_json::from_value(
        json!({"schema_version":1,"text":"ateXYZate","subtract":subtract,
        "required":["eat"],"hints":[],"excluded":[],"min_words":2,"max_words":2,
        "min_word_length":1,"max_word_length":8,"min_zipf":0.0,
        "candidate_budget":10,"allow_repeat":true,"strategy":"prefix","hint_mode":"any"}),
    )
    .unwrap()
}

#[test]
fn subtraction_removes_letters_without_adding_words_and_preserves_required_multiplicity() {
    let req = request(json!(["X y Z"]));
    let result = generate_controlled(
        &req,
        "ate\neat\ntea\n".as_bytes(),
        None,
        &Control::default(),
    )
    .unwrap();
    assert_eq!(result.normalized_input, "ateate");
    assert_eq!(result.bags.len(), 3);
    for bag in result.bags {
        assert_eq!(bag[0], "eat");
        assert_eq!(bag.len(), 2);
    }
}

#[test]
fn invalid_or_overlapping_subtraction_is_rejected_before_corpus_io() {
    for chunks in [json!(["!!!"]), json!(["zz"]), json!(["xyz", "xyz"])] {
        assert!(validate(&request(chunks)).is_err());
    }
    let mut req = request(json!(["atexyz"]));
    req.required = vec!["eat".into(), "eat".into()];
    assert_eq!(validate(&req).err().unwrap().code, "unavailable_letters");
}
