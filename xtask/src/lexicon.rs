//! Answer-independent lexical interventions. Never reads puzzles or target lists.
use crate::{Result, digest, quality, write_json};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn tokens(path: &str) -> Result<BTreeSet<String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .map(quality::key)
        .filter(|s| !s.is_empty())
        .collect())
}
fn annotated(db: &str) -> Result<(BTreeSet<String>, Value)> {
    let con = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    con.pragma_update(None, "query_only", true)?;
    let mut stmt = con.prepare("select distinct word, pos, pos_class from scowl_v0 where size <= 80 and variant_level <= 1 and spelling in ('A','_') and region in ('','US') order by word, pos, pos_class")?;
    let mut words = BTreeSet::new();
    let mut classes = BTreeMap::<String, usize>::new();
    let mut aliases = BTreeMap::<String, BTreeSet<String>>::new();
    for r in stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (display, pos, class) = r?;
        if pos != "s"
            && !["person", "surname", "place", "name", "trademark", "demonym"]
                .contains(&class.as_str())
        {
            continue;
        }
        if pos == "abbr" || display.contains([' ', '-']) {
            continue;
        }
        let k = quality::key(&display);
        if !(3..=24).contains(&k.len()) {
            continue;
        }
        *classes
            .entry(if pos == "s" {
                "contraction".into()
            } else {
                class
            })
            .or_default() += 1;
        if display.contains('\'') {
            aliases.entry(k.clone()).or_default().insert(display);
        }
        words.insert(k);
    }
    Ok((
        words,
        json!({"selected_source_rows":classes,"apostrophe_aliases":aliases,
        "limitations":"ESDB class tags are incomplete; 3-24 letters, no open/hyphenated compounds or abbreviations; apostrophe aliases do not change native rendering"}),
    ))
}
fn frequency(path: &str) -> Result<BTreeSet<String>> {
    let mut counts = BTreeMap::<String, u64>::new();
    let mut total = 0u64;
    for line in fs::read_to_string(path)?.lines() {
        let (raw, n) = line
            .rsplit_once('\t')
            .ok_or("Expected TSV unigram counts")?;
        let n: u64 = n.parse()?;
        // Match production's normalized-token frequency denominator, including
        // tokens excluded from this intervention, rather than inflate Zipf.
        let k = quality::key(raw);
        if k.is_empty() {
            continue;
        }
        total = total.checked_add(n).ok_or("Count overflow")?;
        // Only literal alphabetic/contraction tokens, not URLs/digits or compounds.
        if !(3..=24).contains(&k.len())
            || !raw.chars().all(|c| c.is_ascii_alphabetic() || c == '\'')
        {
            continue;
        }
        let entry = counts.entry(k).or_default();
        *entry = entry.checked_add(n).ok_or("Count overflow")?;
    }
    if total == 0 {
        return Err("Empty unigram model".into());
    }
    Ok(counts
        .into_iter()
        .filter(|(_, n)| *n > 0 && ((*n as f64 / total as f64) * 1e9).log10() >= 3.0)
        .map(|(k, _)| k)
        .collect())
}
pub(crate) fn single_word_diagnostics(path: &str) -> Result<Value> {
    let words = tokens(path)?;
    let mut signatures = BTreeMap::<String, Vec<String>>::new();
    for w in &words {
        signatures
            .entry(quality::signature(w))
            .or_default()
            .push(w.clone());
    }
    let mut lengths = BTreeMap::<usize, (usize, usize, usize)>::new();
    for (s, words) in &signatures {
        let e = lengths.entry(s.len()).or_default();
        e.0 += words.len();
        e.1 += 1;
        e.2 += usize::from(words.len() == 1);
    }
    let examples = ["zzaj", "listen", "jazzman", "rhythm", "queue", "sphinx"]
        .into_iter()
        .map(|input| {
            let solutions = signatures
                .get(&quality::signature(input))
                .cloned()
                .unwrap_or_default();
            json!({"input":input,"normalized_one_word_count":solutions.len(),"words":solutions})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"dictionary_sha256":digest(path)?,"domain":"Exactly one normalized dictionary token, before request admission filters; punctuation/case aliases collapse. No semantic or multiword uniqueness claim.",
        "by_length":lengths.into_iter().map(|(n,(words,bags,singletons))|json!({"letters":n,"words":words,"bags":bags,"singleton_bags":singletons,"singleton_bag_fraction":singletons as f64/bags as f64})).collect::<Vec<_>>(),"examples":examples}),
    )
}
pub fn export(db: &str, current: &str, unigrams: &str, directory: &str) -> Result<Value> {
    let dir = Path::new(directory);
    fs::create_dir_all(dir)?;
    let base = tokens(current)?;
    let (names, annotations) = annotated(db)?;
    let frequent = frequency(unigrams)?;
    let named = base.union(&names).cloned().collect::<BTreeSet<_>>();
    let broad = named.union(&frequent).cloned().collect::<BTreeSet<_>>();
    let mut variants = Vec::new();
    for (id, words, policy) in [
        (
            "baseline",
            base.clone(),
            "Existing native dictionary, normalized and sorted; no vocabulary additions",
        ),
        (
            "names80",
            named,
            "Baseline union ESDB <=80 US/A variants<=1 tagged names and contractions; no target words consulted",
        ),
        (
            "frequency3",
            broad,
            "names80 union literal alphabetic/apostrophe unigrams at normalized Zipf >=3; frequency is not lexical validity",
        ),
    ] {
        let path = dir.join(format!("{id}.txt"));
        fs::write(
            &path,
            words
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
        )?;
        variants.push(
            json!({"id":id,"path":path.canonicalize()?.to_string_lossy(),"sha256":digest(&path)?,
            "keys":words.len(),"added_keys":words.difference(&base).count(),"policy":policy}),
        );
    }
    let manifest = json!({"schema_version":1,"purpose":"Research only; vocabulary policy chosen independently of benchmark answers; no production promotion",
        "inputs":{"db_sha256":digest(db)?,"baseline_sha256":digest(current)?,"unigrams_sha256":digest(unigrams)?},
        "annotations":annotations,"variants":variants});
    write_json(dir.join("variants.json").to_str(), &manifest)?;
    Ok(manifest)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broad_policy_does_not_require_targets_and_preserves_base() {
        let scratch = crate::root().join(".codex/temp/xtask/lexicon-tests");
        fs::create_dir_all(&scratch).unwrap();
        let dir = tempfile::tempdir_in(scratch).unwrap();
        let db = dir.path().join("db.sqlite");
        let con = Connection::open(&db).unwrap();
        con.execute_batch("create table scowl_v0(word text,pos text,pos_class text,size integer,variant_level integer,spelling text,region text);insert into scowl_v0 values ('Alex','n','person',70,1,'A','US'),('we''re','s','',60,0,'_',''),('Wrong','n','name?',60,0,'A','US'),('Too-Rare','n','person',85,0,'A','US');").unwrap();
        drop(con);
        let base = dir.path().join("base.txt");
        let unigram = dir.path().join("counts.tsv");
        fs::write(&base, "cat\n").unwrap();
        fs::write(&unigram, "cat\t10000\nother\t100\nurl2\t100000000\n").unwrap();
        let report = export(
            db.to_str().unwrap(),
            base.to_str().unwrap(),
            unigram.to_str().unwrap(),
            dir.path().join("out").to_str().unwrap(),
        )
        .unwrap();
        assert_eq!(report["variants"][1]["added_keys"], 2);
        let named = fs::read_to_string(report["variants"][1]["path"].as_str().unwrap()).unwrap();
        assert_eq!(named, "alex\ncat\nwere\n");
        assert!(
            !frequency(unigram.to_str().unwrap())
                .unwrap()
                .contains("url")
        );
    }
}
