//! Offline comparison of the production WordNet parser on alternative WNDB data.
use anagram_core::wordnet::WordNet;
use serde_json::json;
use std::path::Path;

fn main() {
    let mut args = std::env::args_os().skip(1);
    let names = ["princeton31", "oewn2025", "oewn2025_plus"];
    let directories: Vec<_> = args.by_ref().take(3).collect();
    assert_eq!(directories.len(), 3, "provide three WNDB directories");
    let words: Vec<String> = args
        .map(|word| word.to_string_lossy().into_owned())
        .collect();
    let words = if words.is_empty() {
        vec![
            "testing",
            "anagram",
            "solver",
            "shakira",
            "voldemort",
            "books",
            "running",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    } else {
        words
    };
    for (name, directory) in names.iter().zip(directories) {
        let net = WordNet::load(Path::new(&directory)).expect("WNDB load");
        let rows: Vec<_> = words
            .iter()
            .map(|word| {
                let features = net.features(word);
                json!({"word": word, "features": features,
                   "frames": net.frames_for(word),
                   "family": net.morphology_family_word(word)})
            })
            .collect();
        println!("{}", json!({"source": name, "words": rows}));
    }
}
