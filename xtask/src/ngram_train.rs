//! Research-only interpolated modified Kneser-Ney training. Input is one
//! whitespace-tokenized sentence per line; never reads benchmark answers.
use crate::{Result, digest};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
};
type Counts = BTreeMap<Vec<String>, u64>;

struct Level {
    counts: Counts,
    contexts: BTreeMap<Vec<String>, (u64, f64)>,
    discounts: [f64; 3],
    fallback: bool,
}
fn discount(c: u64, d: &[f64; 3]) -> f64 {
    d[(c.min(3) - 1) as usize]
}
fn level(counts: Counts) -> Level {
    let mut n = [0f64; 4];
    for &c in counts.values() {
        if c <= 4 {
            n[c as usize - 1] += 1.;
        }
    }
    let fallback = n.contains(&0.);
    let discounts = if fallback {
        [0.75; 3]
    } else {
        let y = n[0] / (n[0] + 2. * n[1]);
        [
            (1. - 2. * y * n[1] / n[0]).clamp(0.01, 0.99),
            (2. - 3. * y * n[2] / n[1]).clamp(0.01, 1.99),
            (3. - 4. * y * n[3] / n[2]).clamp(0.01, 2.99),
        ]
    };
    let mut contexts = BTreeMap::<Vec<String>, (u64, f64)>::new();
    for (g, &c) in &counts {
        let entry = contexts.entry(g[..g.len() - 1].to_vec()).or_default();
        entry.0 += c;
        entry.1 += discount(c, &discounts);
    }
    Level {
        counts,
        contexts,
        discounts,
        fallback,
    }
}
fn probability(levels: &[Level], g: &[String]) -> f64 {
    let l = &levels[g.len() - 1];
    if g.len() == 1 {
        return l.counts.get(g).copied().unwrap_or(0) as f64 / l.contexts[&Vec::new()].0 as f64;
    }
    let lower = probability(levels, &g[1..]);
    let Some(&(total, removed)) = l.contexts.get(&g[..g.len() - 1]) else {
        return lower;
    };
    let c = l.counts.get(g).copied().unwrap_or(0);
    let direct = if c == 0 {
        0.
    } else {
        c as f64 - discount(c, &l.discounts)
    };
    (direct + removed * lower) / total as f64
}
fn train(sentences: &[Vec<String>]) -> Result<Vec<Level>> {
    if sentences.is_empty() {
        return Err("Empty training corpus".into());
    }
    let mut frequencies = BTreeMap::<String, u64>::new();
    for s in sentences {
        for w in s {
            *frequencies.entry(w.clone()).or_default() += 1;
        }
    }
    let mut raw: Vec<Counts> = (0..5).map(|_| Counts::new()).collect();
    for s in sentences {
        let t: Vec<_> = std::iter::once("<s>".to_owned())
            .chain(s.iter().map(|w| {
                if frequencies[w] < 2 {
                    "<unk>".into()
                } else {
                    w.clone()
                }
            }))
            .chain(std::iter::once("</s>".into()))
            .collect();
        for n in 1..=5 {
            for g in t.windows(n) {
                *raw[n - 1].entry(g.to_vec()).or_default() += 1;
            }
        }
    }
    // Continuation counts, not token counts, below the highest order.
    let mut effective = vec![Counts::new(); 5];
    effective[4] = raw[4].clone();
    for n in 1..5 {
        for g in raw[n].keys() {
            *effective[n - 1].entry(g[1..].to_vec()).or_default() += 1;
        }
        // A sentence-initial history has no left context; retain its raw count.
        for (g, &c) in &raw[n - 1] {
            if g[0] == "<s>" && n > 1 {
                effective[n - 1].insert(g.clone(), c);
            }
        }
    }
    effective[0].remove(&vec!["<s>".into()]);
    if !effective[0].contains_key(&vec!["<unk>".into()]) {
        return Err(
            "Training needs singleton words to estimate <unk>; no fabricated probability".into(),
        );
    }
    Ok(effective.into_iter().map(level).collect())
}
pub fn command(input: &str, output: &str) -> Result<Value> {
    if std::path::Path::new(output).exists() {
        return Err("Refusing to overwrite an existing model".into());
    }
    if std::fs::metadata(input)?.len() > 32 * 1024 * 1024 {
        return Err("Training text exceeds 32 MiB research bound".into());
    }
    let sentences = BufReader::new(File::open(input)?).lines().map(|line| {
        let line = line?;
        let s:Vec<_> = line.split_whitespace().map(String::from).collect();
        if s.len()>100 || s.iter().any(|w| w.len()>80 || !w.bytes().any(|c|c.is_ascii_lowercase()) || !w.chars().all(|c| c.is_ascii_lowercase() || c=='\'')) {
            return Err("Training requires lowercase ASCII/apostrophe tokens; reserved symbols prohibited".into());
        }
        Ok(s)
    }).collect::<Result<Vec<_>>>()?.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>();
    let start = std::time::Instant::now();
    let levels = train(&sentences)?;
    let mut f = BufWriter::new(File::create_new(output)?);
    writeln!(f, "\\data\\")?;
    for (i, l) in levels.iter().enumerate() {
        writeln!(
            f,
            "ngram {}={}",
            i + 1,
            l.counts.len() + usize::from(i == 0)
        )?;
    }
    for (i, l) in levels.iter().enumerate() {
        writeln!(f, "\n\\{}-grams:", i + 1)?;
        if i == 0 {
            let (n, d) = levels[1]
                .contexts
                .get(&vec!["<s>".into()])
                .ok_or("No BOS context")?;
            writeln!(f, "-99\t<s>\t{}", (d / *n as f64).log10())?;
        }
        for g in l.counts.keys() {
            write!(
                f,
                "{:.10}\t{}",
                probability(&levels, g).log10(),
                g.join(" ")
            )?;
            if let Some(next) = levels.get(i + 1) {
                let bow = next
                    .contexts
                    .get(g)
                    .map_or(0., |&(n, d)| (d / n as f64).log10());
                write!(f, "\t{bow:.10}")?;
            }
            writeln!(f)?;
        }
    }
    writeln!(f, "\n\\end\\")?;
    f.flush()?;
    Ok(
        json!({"schema_version":1,"research_only":true,"method":"interpolated modified Kneser-Ney, order 5; singleton words mapped to UNK; no pruning",
        "input_sha256":digest(input)?,"model_sha256":digest(output)?,"model_bytes":std::fs::metadata(output)?.len(),"sentences":sentences.len(),"tokens":sentences.iter().map(Vec::len).sum::<usize>(),"training_ms":start.elapsed().as_millis(),
        "levels":levels.iter().enumerate().map(|(i,l)|json!({"order":i+1,"ngrams":l.counts.len(),"discounts":l.discounts,"discount_fallback":l.fallback})).collect::<Vec<_>>()}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distributions_normalize_including_unseen_contexts() {
        let sentences = [
            "a cat sat",
            "a cat slept",
            "the dog sat",
            "the dog slept",
            "a bird flew",
        ]
        .map(|s| s.split_whitespace().map(String::from).collect::<Vec<_>>());
        let m = train(&sentences).unwrap();
        for h in [
            vec![],
            vec!["<s>"],
            vec!["a"],
            vec!["never"],
            vec!["<s>", "a"],
            vec!["<s>", "a", "cat"],
            vec!["<s>", "a", "cat", "sat"],
        ] {
            let sum: f64 = m[0]
                .counts
                .keys()
                .map(|w| {
                    let g = h
                        .iter()
                        .map(|s| s.to_string())
                        .chain(w.iter().cloned())
                        .collect::<Vec<_>>();
                    probability(&m, &g)
                })
                .sum();
            assert!((sum - 1.).abs() < 1e-10, "{h:?}: {sum}");
        }
        assert!(
            probability(&m, &["a".into(), "cat".into()])
                > probability(&m, &["cat".into(), "a".into()])
        );
    }
    #[test]
    fn empty_or_no_unknown_training_is_rejected() {
        assert!(train(&[]).is_err());
        assert!(train(&[vec!["a".into(), "a".into()]]).is_err());
    }
}
