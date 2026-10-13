//! Differential trainer probe; no model artifact publication.
use anagram_core::{control::Control, training};
use serde::Deserialize;
use std::io::{self, BufRead};

#[derive(Deserialize)]
struct Input {
    groups: Vec<training::Group>,
    options: training::Options,
}
fn main() {
    for line in io::stdin().lock().lines() {
        let input: Input = serde_json::from_str(&line.unwrap()).unwrap();
        let report = training::train(&input.groups, &input.options, &Control::default()).unwrap();
        println!("{}", serde_json::to_string(&report).unwrap());
    }
}
