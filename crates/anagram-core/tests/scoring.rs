use anagram_core::{lexicon::Unigrams, scoring::Bigrams};
use std::{
    collections::BTreeSet,
    io::{self, ErrorKind},
};

fn error<T>(result: io::Result<T>) -> io::Error {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected malformed bigram input to fail"),
    }
}

fn vocabulary() -> BTreeSet<String> {
    ["a", "b"].into_iter().map(str::to_owned).collect()
}

#[test]
fn every_negative_bigram_row_is_rejected_before_duplicate_aggregation() {
    let unigrams = Unigrams::load("a\t10\nb\t10\n".as_bytes()).unwrap();
    let error = error(Bigrams::load(
        "a b\t10\na b\t-1\n".as_bytes(),
        &unigrams,
        &vocabulary(),
    ));
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "negative bigram count");
}

#[test]
fn negative_rows_are_rejected_even_when_the_pair_is_outside_the_vocabulary() {
    let unigrams = Unigrams::load("a\t10\nb\t10\n".as_bytes()).unwrap();
    let error = error(Bigrams::load(
        "outside pair\t-1\n".as_bytes(),
        &unigrams,
        &vocabulary(),
    ));
    assert_eq!(error.to_string(), "negative bigram count");
}

#[test]
fn nonnegative_duplicate_rows_still_aggregate_and_detect_overflow() {
    let unigrams = Unigrams::load("a\t10\nb\t10\n".as_bytes()).unwrap();
    let bigrams = Bigrams::load("a b\t4\na b\t6\n".as_bytes(), &unigrams, &vocabulary()).unwrap();
    assert_eq!(bigrams.count("a", "b"), 10);

    let error = error(Bigrams::load(
        format!("a b\t{}\na b\t1\n", i64::MAX).as_bytes(),
        &unigrams,
        &vocabulary(),
    ));
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert_eq!(error.to_string(), "bigram count overflow");
}

#[test]
fn noninteger_nan_infinite_and_out_of_range_counts_preserve_skip_semantics() {
    let unigrams = Unigrams::load("a\t10\nb\t10\n".as_bytes()).unwrap();
    let mut input = "a b\t4\n".to_owned();
    for count in [
        "NaN",
        "inf",
        "-inf",
        "Infinity",
        "1e999",
        "1.5",
        "9223372036854775808",
        "-9223372036854775809",
    ] {
        input.push_str(&format!("a b\t{count}\n"));
    }
    let bigrams = Bigrams::load(input.as_bytes(), &unigrams, &vocabulary()).unwrap();
    assert_eq!(bigrams.count("a", "b"), 4);
    assert!(bigrams.edge("a", "b").is_finite());
    let minimum = format!("a b\t{}\n", i64::MIN);
    assert_eq!(
        error(Bigrams::load(minimum.as_bytes(), &unigrams, &vocabulary())).to_string(),
        "negative bigram count"
    );
}
