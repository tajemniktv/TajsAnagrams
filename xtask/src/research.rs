//! Offline experiments; no downloads or production defaults are changed.
use crate::{
    Args, Result, digest,
    quality::{self, Cases},
    read_json, root, write_json,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    path::Path,
};
type Model = BTreeMap<Vec<String>, (f64, f64)>;
fn contexts(groups: &[Value]) -> Result<BTreeSet<Vec<String>>> {
    let mut out = BTreeSet::from([vec!["<unk>".into()]]);
    for g in groups {
        for p in g["orders"].as_array().ok_or("Missing order pool")? {
            let words = p.as_str().ok_or("Non-string order")?.to_lowercase();
            let tokens = std::iter::once("<s>")
                .chain(words.split_whitespace())
                .chain(std::iter::once("</s>"))
                .map(String::from)
                .collect::<Vec<_>>();
            for i in 1..tokens.len() {
                for n in 1..=5.min(i + 1) {
                    out.insert(tokens[i + 1 - n..=i].to_vec());
                    if n > 1 {
                        out.insert(tokens[i + 1 - n..i].to_vec());
                    }
                }
            }
        }
    }
    Ok(out)
}
fn load_arpa(reader: impl BufRead, required: &BTreeSet<Vec<String>>) -> Result<Model> {
    let mut model = Model::new();
    let mut order = 0;
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.starts_with('\\') && line.ends_with("-grams:") {
            order = line[1..]
                .split('-')
                .next()
                .ok_or("Missing order")?
                .parse::<usize>()?;
            if !(1..=5).contains(&order) {
                return Err("Unsupported ARPA order".into());
            }
            continue;
        }
        if line.starts_with('\\') {
            order = 0;
            continue;
        }
        if order == 0 || line.is_empty() {
            continue;
        }
        let f = line.split_whitespace().collect::<Vec<_>>();
        if f.len() != order + 1 && f.len() != order + 2 {
            return Err("Malformed ARPA row".into());
        }
        let p = f[1..=order]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        if required.contains(&p) {
            let a: f64 = f[0].parse()?;
            let b: f64 = if f.len() == order + 2 {
                f[order + 1].parse()?
            } else {
                0.
            };
            if !a.is_finite() || !b.is_finite() {
                return Err("Nonfinite ARPA value".into());
            }
            if model.insert(p, (a, b)).is_some() {
                return Err("Duplicate queried ARPA entry".into());
            }
        }
    }
    Ok(model)
}
fn conditional(model: &Model, sequence: &[String]) -> Result<(f64, usize, usize)> {
    let mut h = &sequence[sequence.len().saturating_sub(5)..];
    let (mut backoff, mut steps) = (0., 0);
    while h.len() > 1 {
        if let Some((p, _)) = model.get(h) {
            return Ok((backoff + p, h.len(), steps));
        }
        backoff += model.get(&h[..h.len() - 1]).map_or(0., |r| r.1);
        h = &h[1..];
        steps += 1;
    }
    let p = model
        .get(h)
        .or_else(|| model.get(&vec!["<unk>".into()]))
        .ok_or("No queried unigram or <unk>")?;
    Ok((backoff + p.0, 1, steps))
}
fn score(model: &Model, phrase: &str) -> Result<Value> {
    let lower = phrase.to_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return Err("Empty order".into());
    }
    let t = std::iter::once("<s>")
        .chain(words.iter().copied())
        .chain(std::iter::once("</s>"))
        .map(String::from)
        .collect::<Vec<_>>();
    let ps = (1..t.len())
        .map(|i| conditional(model, &t[i.saturating_sub(4)..=i]))
        .collect::<Result<Vec<_>>>()?;
    let total = ps.iter().map(|p| p.0).sum::<f64>();
    Ok(
        json!({"phrase":phrase,"total_log10":total,"mean_log10_per_word":total/words.len()as f64,"worst_log10":ps.iter().map(|p|p.0).fold(f64::INFINITY,f64::min),"direct_hits":(2..=5).map(|n|(n.to_string(),json!(ps.iter().filter(|p|p.1==n).count()))).collect::<BTreeMap<_,_>>(),"backoff_steps":ps.iter().map(|p|p.2).sum::<usize>(),"bos_log10":ps[0].0,"eos_log10":ps.last().unwrap().0}),
    )
}
fn ngram(arpa: &str, groups: &str) -> Result<Value> {
    let groups = read_json(groups)?;
    let groups = groups.as_array().ok_or("Groups must be an array")?;
    let model = load_arpa(BufReader::new(File::open(arpa)?), &contexts(groups)?)?;
    let mut out = vec![];
    for g in groups {
        let mut rows = g["orders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| score(&model, p.as_str().unwrap()))
            .collect::<Result<Vec<_>>>()?;
        rows.sort_by(|a, b| {
            b["total_log10"]
                .as_f64()
                .unwrap()
                .total_cmp(&a["total_log10"].as_f64().unwrap())
                .then_with(|| {
                    a["phrase"]
                        .as_str()
                        .unwrap()
                        .cmp(b["phrase"].as_str().unwrap())
                })
        });
        if rows.len() > 1 {
            let margin =
                rows[0]["total_log10"].as_f64().unwrap() - rows[1]["total_log10"].as_f64().unwrap();
            for r in &mut rows {
                r["top_margin_log10"] = json!(margin);
            }
        }
        out.push(json!({"case_id":g["case_id"],"orders":rows}));
    }
    Ok(json!(out))
}
fn desired(cases: &Cases) -> BTreeSet<String> {
    cases
        .values()
        .flat_map(|c| c.relevance.keys())
        .flat_map(|p| p.split_whitespace())
        .map(quality::key)
        .collect()
}
fn lexicon(db: &str, current: &str, cases: &Cases) -> Result<Value> {
    let con = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    con.pragma_update(None, "query_only", true)?;
    let mut stmt=con.prepare("select word, min(size) from scowl_v0 where size <= 80 and variant_level <= 1 and spelling in ('A', '_') and region in ('', 'US') group by word order by word")?;
    let mut lexemes = BTreeMap::<String, i64>::new();
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (display, size) = row?;
        let key = quality::key(&display);
        if key.is_empty() || display.contains([' ', '-']) {
            continue;
        }
        lexemes
            .entry(key)
            .and_modify(|v| *v = (*v).min(size))
            .or_insert(size);
    }
    let mut baseline = BTreeSet::new();
    for line in BufReader::new(File::open(current)?).lines() {
        let k = quality::key(&line?);
        if !k.is_empty() {
            baseline.insert(k);
        }
    }
    let mut tiers = BTreeMap::<&str, usize>::new();
    for size in lexemes.values() {
        *tiers
            .entry(if *size <= 60 {
                "core"
            } else if *size <= 70 {
                "extended"
            } else {
                "obscure"
            })
            .or_default() += 1;
    }
    let words = desired(cases);
    let shared = lexemes.keys().filter(|k| baseline.contains(*k)).count();
    Ok(
        json!({"source_db_sha256":digest(db)?,"baseline_sha256":digest(current)?,"esdb_keys":lexemes.len(),"baseline_keys":baseline.len(),"tiers":tiers,"shared":shared,"esdb_only":lexemes.len()-shared,"baseline_only":baseline.len()-shared,"gold_words":words.len(),"gold_covered_esdb":words.iter().filter(|k|lexemes.contains_key(*k)).count(),"gold_covered_baseline":words.intersection(&baseline).count(),"gold_missing_esdb":words.iter().filter(|k|!lexemes.contains_key(*k)).collect::<Vec<_>>()}),
    )
}
struct LimitedWriter<W> {
    inner: W,
    left: u64,
}
struct LimitedReader<R> {
    inner: R,
    left: u64,
}
impl<R: Read> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.left == 0 {
            let mut probe = [0; 1];
            return if self.inner.read(&mut probe)? == 0 {
                Ok(0)
            } else {
                Err(io::Error::other(
                    "Research input exceeds --max-bytes during actual read",
                ))
            };
        }
        let size = buf.len().min(self.left.min(usize::MAX as u64) as usize);
        let n = self.inner.read(&mut buf[..size])?;
        self.left -= n as u64;
        Ok(n)
    }
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if b.len() as u64 > self.left {
            return Err(io::Error::other(
                "Expanded research input exceeds --max-bytes",
            ));
        }
        let n = self.inner.write(b)?;
        self.left -= n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
fn tubelex(path: &str, unigrams: &str, cases: &Cases, max: u64) -> Result<Value> {
    let input = Path::new(path);
    if input.metadata()?.len() > max {
        return Err("Research input exceeds --max-bytes".into());
    }
    let scratch = root().join(".codex/temp/xtask/research");
    std::fs::create_dir_all(&scratch)?;
    let mut expanded = tempfile::NamedTempFile::new_in(scratch)?;
    let mut reader: Box<dyn Read> = if input.extension().is_some_and(|s| s == "xz") {
        lzma_rs::xz_decompress(
            &mut BufReader::new(File::open(input)?),
            &mut LimitedWriter {
                inner: expanded.as_file_mut(),
                left: max,
            },
        )?;
        Box::new(File::open(expanded.path())?)
    } else {
        Box::new(File::open(input)?)
    };
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(LimitedReader {
            inner: &mut reader,
            left: max,
        });
    let headers = csv.headers()?.clone();
    let positions = ["word", "count", "videos", "channels"]
        .iter()
        .map(|n| {
            headers
                .iter()
                .position(|h| h == *n)
                .ok_or("Missing TUBELEX column")
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut tube = BTreeMap::new();
    for r in csv.records() {
        let r = r?;
        let word = &r[positions[0]];
        if word == "[TOTAL]" || quality::key(word) != word {
            continue;
        }
        let values = positions[1..]
            .iter()
            .map(|i| r[*i].parse::<u64>())
            .collect::<std::result::Result<Vec<_>, _>>()?;
        tube.insert(word.to_string(), values);
    }
    let mut old = BTreeMap::<String, u64>::new();
    for line in BufReader::new(File::open(unigrams)?).lines() {
        let line = line?;
        let mut f = line.rsplitn(2, char::is_whitespace);
        let count = f.next().unwrap_or("");
        let word = f.next().unwrap_or("").trim_end();
        if let Ok(n) = count.parse::<u64>()
            && !word.is_empty()
            && quality::key(word) == word
        {
            let v = old.entry(word.into()).or_default();
            *v = v.checked_add(n).ok_or("Unigram overflow")?;
        }
    }
    let words = desired(cases);
    let features=words.iter().map(|w|{let values=tube.get(w).cloned().unwrap_or(vec![0,0,0]);(w,json!({"old_count":old.get(w).copied().unwrap_or(0),"log_count":(values[0]as f64).ln_1p(),"log_videos":(values[1]as f64).ln_1p(),"log_channels":(values[2]as f64).ln_1p()}))}).collect::<BTreeMap<_,_>>();
    Ok(
        json!({"tubelex_sha256":digest(path)?,"old_sha256":digest(unigrams)?,"tubelex_tokens":tube.len(),"old_tokens":old.len(),"gold_words":words.len(),"tubelex_covered":words.iter().filter(|w|tube.contains_key(*w)).count(),"old_covered":words.iter().filter(|w|old.contains_key(*w)).count(),"features":features}),
    )
}
pub fn command(args: &[String]) -> Result<()> {
    let Some(name) = args.first() else {
        return Err("Expected research lexicon|ngram|tubelex".into());
    };
    if args.get(1).is_some_and(|s| s == "--help") {
        println!(
            "research lexicon --db ESDB.sqlite [--current DICTIONARY --cases FILE --output FILE]\nresearch ngram --arpa FILE --groups FILE [--output FILE]\nresearch tubelex --tubelex FILE.tsv[.xz] [--unigrams FILE --cases FILE --max-bytes N --output FILE]\nOffline reports only; no data/model promotion. ESDB database creation is upstream work, not a dependency on its Python builder."
        );
        return Ok(());
    }
    let a = Args::parse(
        &args[1..],
        &[
            "db",
            "current",
            "cases",
            "output",
            "arpa",
            "groups",
            "tubelex",
            "unigrams",
            "max-bytes",
        ],
        &[],
    )?;
    let result = match name.as_str() {
        "ngram" => ngram(a.required("arpa")?, a.required("groups")?)?,
        "lexicon" => lexicon(
            a.required("db")?,
            a.value("current", ".anagram_data/dictionary/normal_user_v2.txt"),
            &quality::load_cases(a.value("cases", "tests/quality/cases.v1.json"))?,
        )?,
        "tubelex" => tubelex(
            a.required("tubelex")?,
            a.value("unigrams", ".anagram_data/ngrams/count_1w.txt"),
            &quality::load_cases(a.value("cases", "tests/quality/cases.v1.json"))?,
            a.number("max-bytes", 2 * 1024 * 1024 * 1024)?,
        )?,
        _ => return Err("Unknown research command".into()),
    };
    write_json(a.get("output"), &result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_backoff_unknown_and_invalid_arpa() {
        let arpa = "\\1-grams:\n-1 <s> -0.1\n-1 </s>\n-3 <unk>\n-1 good -0.2\n-1 bad -0.2\n\\2-grams:\n-0.1 <s> good -0.1\n-1 <s> bad -0.1\n-0.2 good </s>\n\\end\\\n";
        let required = contexts(&[json!({"orders":["good","bad","unknown"]})]).unwrap();
        let model = load_arpa(BufReader::new(arpa.as_bytes()), &required).unwrap();
        let good = score(&model, "good").unwrap();
        let bad = score(&model, "bad").unwrap();
        assert!((good["total_log10"].as_f64().unwrap() + 0.4).abs() < 1e-12);
        assert_eq!(good["direct_hits"]["2"], 2);
        assert!(bad["backoff_steps"].as_u64().unwrap() > 0);
        assert!(score(&model, "unknown").is_ok());
        assert!(load_arpa(BufReader::new(b"\\6-grams:\n".as_slice()), &required).is_err());
        assert!(
            load_arpa(
                BufReader::new(b"\\1-grams:\nNaN good\n".as_slice()),
                &required
            )
            .is_err()
        );
    }
    #[test]
    fn expansion_limit_is_actual_write_limit() {
        let mut bytes = vec![];
        let mut writer = LimitedWriter {
            inner: &mut bytes,
            left: 3,
        };
        writer.write_all(b"abc").unwrap();
        assert!(writer.write_all(b"d").is_err());
        assert_eq!(bytes, b"abc");
        let mut bytes = vec![];
        LimitedReader {
            inner: b"abc".as_slice(),
            left: 3,
        }
        .read_to_end(&mut bytes)
        .unwrap();
        assert_eq!(bytes, b"abc");
        assert!(
            LimitedReader {
                inner: b"abcd".as_slice(),
                left: 3
            }
            .read_to_end(&mut vec![])
            .is_err()
        );
    }

    #[test]
    fn read_only_esdb_tiers_filters_and_normalization_collisions() {
        let scratch = root().join(".codex/temp/xtask/research-tests");
        std::fs::create_dir_all(&scratch).unwrap();
        let temp = tempfile::tempdir_in(scratch).unwrap();
        let db = temp.path().join("esdb.sqlite");
        let dictionary = temp.path().join("dictionary.txt");
        let con = Connection::open(&db).unwrap();
        con.execute_batch("create table scowl_v0(word text,size integer,spelling text,variant_level integer,region text);insert into scowl_v0 values ('testing',50,'A',0,'US'),('TESTING',70,'A',0,'US'),('anagram',70,'_',1,''),('solver',80,'A',0,'US'),('excluded',20,'B',0,'US'),('open compound',30,'A',0,'US'),('hyphen-word',30,'A',0,'US');").unwrap();
        drop(con);
        std::fs::write(&dictionary, "testing\nsolver\nother\n").unwrap();
        let before = digest(&db).unwrap();
        let cases = quality::load_cases(root().join("tests/quality/cases.v1.json")).unwrap();
        let v = lexicon(db.to_str().unwrap(), dictionary.to_str().unwrap(), &cases).unwrap();
        assert_eq!(v["esdb_keys"], 3);
        assert_eq!(v["shared"], 2);
        assert_eq!(v["tiers"], json!({"core":1,"extended":1,"obscure":1}));
        assert_eq!(digest(&db).unwrap(), before);
    }
    #[test]
    fn compressed_tubelex_fields_aggregation_and_expansion_rejection() {
        let scratch = root().join(".codex/temp/xtask/research-tests");
        std::fs::create_dir_all(&scratch).unwrap();
        let temp = tempfile::tempdir_in(scratch).unwrap();
        let tsv = temp.path().join("tokens.tsv.xz");
        let old = temp.path().join("unigrams.txt");
        std::fs::write(&old, "testing 2\ntesting 3\nother 10\n").unwrap();
        let data=b"word\tcount\tvideos\tchannels\ntesting\t10\t3\t2\nsolver\t4\t2\t1\n[TOTAL]\t14\t5\t3\nBad\t1\t1\t1\n";
        let mut compressed = vec![];
        lzma_rs::xz_compress(&mut BufReader::new(data.as_slice()), &mut compressed).unwrap();
        std::fs::write(&tsv, compressed).unwrap();
        let cases = quality::load_cases(root().join("tests/quality/cases.v1.json")).unwrap();
        let v = tubelex(tsv.to_str().unwrap(), old.to_str().unwrap(), &cases, 4096).unwrap();
        assert_eq!(v["tubelex_tokens"], 2);
        assert_eq!(v["features"]["testing"]["old_count"], 5);
        assert!(
            (v["features"]["testing"]["log_count"].as_f64().unwrap() - 11f64.ln()).abs() < 1e-12
        );
        let mut compressed = vec![];
        lzma_rs::xz_compress(
            &mut BufReader::new(vec![b'x'; 20000].as_slice()),
            &mut compressed,
        )
        .unwrap();
        std::fs::write(&tsv, compressed).unwrap();
        assert!(tubelex(tsv.to_str().unwrap(), old.to_str().unwrap(), &cases, 4096).is_err());
    }
}
