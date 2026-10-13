//! Deliberately simple independent verifier. No production inventory subtraction,
//! pruning, normalization, traversal or hint-matching logic is used for expectations.
use anagram_core::{
    Inventory,
    generation::{Candidate, HintMode, Options, Stop, Strategy, search},
};
use std::{collections::BTreeSet, sync::atomic::AtomicBool};

fn counts(words: &[String]) -> [u32; 3] {
    let mut c = [0; 3];
    for word in words {
        for b in word.bytes() {
            c[(b - b'a') as usize] += 1;
        }
    }
    c
}
fn enumerate(words: &[String], target: [u32; 3], options: &Options) -> BTreeSet<Vec<String>> {
    fn visit(
        words: &[String],
        target: [u32; 3],
        o: &Options,
        start: usize,
        path: &mut Vec<String>,
        out: &mut BTreeSet<Vec<String>>,
    ) {
        let c = counts(path);
        if c == target && path.len() >= o.min_words {
            let mut matched = o
                .initial_clues
                .intersection(&o.clues)
                .cloned()
                .collect::<BTreeSet<_>>();
            matched.extend(path.iter().filter(|w| o.clues.contains(*w)).cloned());
            if o.clues.is_empty()
                || match o.hint_mode {
                    HintMode::Any => !matched.is_empty(),
                    HintMode::ExactlyOne => matched.len() == 1,
                }
            {
                let mut p = path.clone();
                p.sort();
                out.insert(p);
            }
        }
        if path.len() == o.max_words || c.iter().zip(target).any(|(a, b)| *a > b) {
            return;
        }
        for i in start..words.len() {
            path.push(words[i].clone());
            visit(
                words,
                target,
                o,
                if o.allow_repeat { i } else { i + 1 },
                path,
                out,
            );
            path.pop();
        }
    }
    let mut out = BTreeSet::new();
    visit(words, target, options, 0, &mut vec![], &mut out);
    out
}
fn next(seed: &mut u64) -> usize {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 32) as usize
}
#[test]
fn optimized_search_agrees_with_independent_exhaustive_enumerator() {
    let mut seed = 20261013;
    let mut vocab = vec![];
    for length in 1..=3 {
        for n in 0..3usize.pow(length) {
            let mut n = n;
            let mut w = String::new();
            for _ in 0..length {
                w.push((b'a' + (n % 3) as u8) as char);
                n /= 3;
            }
            vocab.push(w);
        }
    }
    for index in 0..1000 {
        let size = 1 + next(&mut seed) % 12;
        let mut words = BTreeSet::new();
        while words.len() < size {
            words.insert(vocab[next(&mut seed) % vocab.len()].clone());
        }
        let mut words = words.into_iter().collect::<Vec<_>>();
        for i in (1..words.len()).rev() {
            let j = next(&mut seed) % (i + 1);
            words.swap(i, j);
        }
        let mut text = String::new();
        for _ in 0..1 + next(&mut seed) % 7 {
            text.push((b'a' + (next(&mut seed) % 3) as u8) as char);
        }
        let max = 1 + next(&mut seed) % 4;
        let min = 1 + next(&mut seed) % max;
        let mut options = Options {
            min_words: min,
            max_words: max,
            max_results: 0,
            allow_repeat: next(&mut seed).is_multiple_of(2),
            clues: BTreeSet::new(),
            initial_clues: BTreeSet::new(),
            hint_mode: if next(&mut seed).is_multiple_of(2) {
                HintMode::Any
            } else {
                HintMode::ExactlyOne
            },
            strategy: Strategy::Prefix,
        };
        for _ in 0..next(&mut seed) % 4 {
            options
                .clues
                .insert(words[next(&mut seed) % words.len()].clone());
        }
        if next(&mut seed).is_multiple_of(3) {
            options
                .initial_clues
                .extend(options.clues.iter().take(1).cloned());
        }
        let expected = enumerate(&words, counts(&[text.clone()]), &options);
        let candidates = words
            .iter()
            .map(|w| Candidate::new(w).unwrap())
            .collect::<Vec<_>>();
        for strategy in [Strategy::Prefix, Strategy::Diverse] {
            options.strategy = strategy;
            for cap in [0, 1, 3] {
                options.max_results = cap;
                let result = search(
                    Inventory::from_text(&text),
                    &candidates,
                    &options,
                    &AtomicBool::new(false),
                    None,
                )
                .unwrap();
                let actual = result
                    .bags
                    .iter()
                    .map(|b| {
                        let mut b = b.clone();
                        b.sort();
                        b
                    })
                    .collect::<BTreeSet<_>>();
                assert_eq!(
                    actual.len(),
                    result.bags.len(),
                    "duplicate bag case {index}"
                );
                assert!(actual.is_subset(&expected), "invalid bag case {index}");
                if cap == 0 || expected.len() <= cap {
                    assert_eq!(actual, expected, "exhaustive mismatch case {index}");
                    assert_eq!(result.stop, Stop::Exhausted);
                } else {
                    assert_eq!(actual.len(), cap);
                    assert_eq!(result.stop, Stop::CandidateCap);
                }
                let repeat = search(
                    Inventory::from_text(&text),
                    &candidates,
                    &options,
                    &AtomicBool::new(false),
                    None,
                )
                .unwrap();
                assert_eq!(result.bags, repeat.bags, "nondeterministic case {index}");
            }
        }
    }
}
