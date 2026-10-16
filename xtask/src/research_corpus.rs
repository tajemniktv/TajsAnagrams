//! Answer-independent, deterministic sampling of a supplied Tatoeba detailed TSV.
use crate::{Result, digest};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
};

pub fn prepare(input: &str, train: &str, heldout: &str, limit: usize) -> Result<Value> {
    if limit == 0
        || limit > 200_000
        || train == heldout
        || [train, heldout]
            .iter()
            .any(|p| std::path::Path::new(p).exists())
    {
        return Err("Use distinct new outputs and 1..200000 sampled sentences".into());
    }
    let token = regex::Regex::new("[a-z]+(?:'[a-z]+)*")?;
    let mut sample = BTreeMap::<[u8; 32], (String, String, String)>::new();
    let (mut rows, mut eligible, mut duplicates) = (0usize, 0usize, 0usize);
    for line in BufReader::new(File::open(input)?).lines() {
        let line = line?;
        rows += 1;
        let f = line.split('\t').collect::<Vec<_>>();
        if f.len() != 6 || f[1] != "eng" || f[0].parse::<u64>().is_err() {
            return Err("Expected detailed English Tatoeba TSV (six fields)".into());
        }
        let lower = f[2].replace(['‘', '’'], "'").to_lowercase();
        let words = token
            .find_iter(&lower)
            .map(|m| m.as_str())
            .collect::<Vec<_>>();
        if !(2..=30).contains(&words.len()) || words.iter().any(|w| w.len() > 80) {
            continue;
        }
        eligible += 1;
        let normalized = words.join(" ");
        let hash: [u8; 32] = Sha256::digest(normalized.as_bytes()).into();
        if let Some(previous) = sample.get(&hash) {
            if previous.0 != normalized {
                return Err("Corpus digest collision".into());
            }
            duplicates += 1;
            continue;
        }
        if sample.len() == limit && hash > *sample.last_key_value().unwrap().0 {
            continue;
        }
        sample.insert(hash, (normalized, f[0].into(), f[3].into()));
        if sample.len() > limit {
            sample.pop_last();
        }
    }
    let mut train_file = BufWriter::new(File::create_new(train)?);
    let mut held_file = BufWriter::new(File::create_new(heldout)?);
    let (mut trained, mut reserved, mut tokens) = (0, 0, 0);
    let mut attribution = Vec::new();
    for (hash, (text, id, author)) in &sample {
        let held = hash[0] % 10 == 0;
        if held {
            writeln!(held_file, "{text}")?;
            reserved += 1;
        } else {
            writeln!(train_file, "{text}")?;
            trained += 1;
            tokens += text.split_whitespace().count();
        }
        attribution.push(json!({"sentence_id":id,"author":author,"normalized_sha256":hex::encode(hash),"heldout":held}));
    }
    train_file.flush()?;
    held_file.flush()?;
    Ok(
        json!({"schema_version":1,"source_sha256":digest(input)?,"rows":rows,"eligible_rows":eligible,"selected_duplicate_encounters":duplicates,"selected":sample.len(),"train_sentences":trained,"train_tokens":tokens,"heldout_sentences":reserved,
        "train_sha256":digest(train)?,"heldout_sha256":digest(heldout)?,"policy":"Lowest SHA256 normalized texts, deduplicated within selected sample; 2..30 ASCII/apostrophe words; hash-byte mod10 reserves sentences, not independent authors. No target/judgment input.","selected_attribution":attribution}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_is_deduplicated_order_independent_and_non_destructive() {
        let root = crate::root().join(".codex/temp/xtask/corpus-tests");
        std::fs::create_dir_all(&root).unwrap();
        let t = tempfile::tempdir_in(root).unwrap();
        let a = t.path().join("a.tsv");
        let b = t.path().join("b.tsv");
        let rows = [
            "1\teng\tI can’t go.\ta\tx\tx",
            "2\teng\tI can't go!\tb\tx\tx",
            "3\teng\tWe are here.\ta\tx\tx",
        ];
        std::fs::write(&a, rows.join("\n")).unwrap();
        std::fs::write(&b, rows.into_iter().rev().collect::<Vec<_>>().join("\n")).unwrap();
        let outputs = (0..4)
            .map(|i| t.path().join(format!("{i}.txt")))
            .collect::<Vec<_>>();
        let report = prepare(
            a.to_str().unwrap(),
            outputs[0].to_str().unwrap(),
            outputs[1].to_str().unwrap(),
            3,
        )
        .unwrap();
        prepare(
            b.to_str().unwrap(),
            outputs[2].to_str().unwrap(),
            outputs[3].to_str().unwrap(),
            3,
        )
        .unwrap();
        assert_eq!(report["selected"], 2);
        assert_eq!(
            std::fs::read(&outputs[0]).unwrap(),
            std::fs::read(&outputs[2]).unwrap()
        );
        assert_eq!(
            std::fs::read(&outputs[1]).unwrap(),
            std::fs::read(&outputs[3]).unwrap()
        );
        assert!(
            prepare(
                a.to_str().unwrap(),
                outputs[0].to_str().unwrap(),
                outputs[1].to_str().unwrap(),
                3
            )
            .is_err()
        );
    }
}
