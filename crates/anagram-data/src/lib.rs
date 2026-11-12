//! Typed, cancellable native preparation operations shared by desktop and CLI.
pub mod phrase_builder;
use anagram_core::{
    control::Control, policy::DeploymentLimits, provenance::CorpusReadBudget, request::Error,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashSet},
    fs::File,
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
};
fn err(e: impl std::fmt::Display) -> Error {
    Error::new("tool_error", e.to_string())
}
fn corpus_err(e: io::Error) -> Error {
    if e.kind() == io::ErrorKind::FileTooLarge {
        Error::corpus(e)
    } else {
        err(e)
    }
}
fn input<T: serde::de::DeserializeOwned>(
    control: &Control,
    source: &mut dyn Read,
) -> Result<T, Error> {
    let mut bytes = Vec::new();
    control
        .reader(source)
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(err("Tool input exceeds 16 MiB"));
    }
    serde_json::from_slice(&bytes).map_err(err)
}
fn stage(path: &Path) -> Result<tempfile::NamedTempFile, Error> {
    if path.exists() {
        return Err(err(
            "Output already exists; choose a new filename to preserve existing data",
        ));
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    tempfile::NamedTempFile::new_in(parent).map_err(err)
}

#[derive(Debug)]
pub enum Operation {
    BuildGroups {
        wordnet: PathBuf,
        phrase: Option<PathBuf>,
    },
    Train {
        output: PathBuf,
    },
    Rank {
        model: PathBuf,
    },
    Dictionary {
        base: PathBuf,
        unigrams: PathBuf,
        output: PathBuf,
    },
    Phrases {
        output: PathBuf,
        sources: Vec<PathBuf>,
    },
}

pub fn execute(
    operation: Operation,
    control: &Control,
    source: &mut dyn Read,
    limits: &DeploymentLimits,
) -> Result<Value, Error> {
    limits.validate()?;
    let mut budget = limits.corpus_read_budget();
    let result = (|| match operation {
        Operation::BuildGroups { wordnet, phrase } => {
            build_groups(&wordnet, phrase.as_deref(), control, source, &mut budget)
        }
        Operation::Train { output } => {
            #[derive(Deserialize)]
            struct Training {
                groups: Vec<anagram_core::training::Group>,
                #[serde(default)]
                options: anagram_core::training::Options,
            }
            let data: Training = input(control, source)?;
            let report = anagram_core::training::train(&data.groups, &data.options, control)?;
            let mut file = stage(&output)?;
            serde_json::to_writer_pretty(&mut file, &report.model).map_err(err)?;
            file.as_file().sync_all().map_err(err)?;
            control.check().map_err(err)?;
            file.persist_noclobber(&output).map_err(err)?;
            Ok(
                json!({"baseline":report.baseline,"held_out":report.held_out,"model_path":output.to_string_lossy(),"note":"Saved all-group fit; only held_out metrics estimate generalization"}),
            )
        }
        Operation::Rank { model } => {
            let model = anagram_core::learned::Model::load_controlled_with_budget(
                &model,
                control,
                &mut budget,
            )
            .map_err(corpus_err)?;
            let items: Vec<anagram_core::learned::Item> = input(control, source)?;
            serde_json::to_value(
                anagram_core::learned::rank_result(&items, Some(&model), control).map_err(err)?,
            )
            .map_err(err)
        }
        Operation::Dictionary {
            base,
            unigrams,
            output,
        } => {
            let unigrams = anagram_core::lexicon::Unigrams::load(BufReader::new(budget.reader(
                File::open(&unigrams).map_err(err)?,
                "unigrams",
                control,
            )))
            .map_err(corpus_err)?;
            let mut words = BTreeSet::new();
            for line in anagram_core::lexicon::decoded_lines(BufReader::new(budget.reader(
                File::open(&base).map_err(err)?,
                "dictionary",
                control,
            ))) {
                let word = anagram_core::normalize_letters(&line.map_err(corpus_err)?);
                if !word.is_empty() {
                    words.insert(word);
                }
            }
            let extra: Vec<_> = words
                .iter()
                .filter(|w| w.len() == 2 && unigrams.zipf(w) >= 5.0)
                .cloned()
                .collect();
            words.extend(
                [
                    "dont", "cant", "wont", "isnt", "arent", "wasnt", "werent", "didnt", "doesnt",
                    "couldnt", "shouldnt", "wouldnt", "hasnt", "havent", "hadnt",
                ]
                .map(str::to_owned),
            );
            let mut file = stage(Path::new(&output))?;
            for word in &words {
                control.check().map_err(err)?;
                writeln!(file, "{word}").map_err(err)?;
            }
            file.as_file().sync_all().map_err(err)?;
            control.check().map_err(err)?;
            file.persist_noclobber(&output).map_err(err)?;
            Ok(
                json!({"dictionary":output.to_string_lossy(),"words":words.len(),"extra_short_words":extra}),
            )
        }
        Operation::Phrases { output, sources } => crate::phrase_builder::build_with_budget(
            &output,
            &sources,
            crate::phrase_builder::default_workers(),
            control,
            &mut budget,
        ),
    })();
    finish_command(result, control)
}

fn finish_command(result: Result<Value, Error>, control: &Control) -> Result<Value, Error> {
    // A successful command may have already committed an output. Cancellation
    // cannot turn that committed success into a failure that cannot be retried.
    result.map_err(|error| match control.check() {
        Err(reason) => Error::new(reason, reason),
        Ok(()) => error,
    })
}

fn build_groups(
    wordnet: &Path,
    phrase_path: Option<&Path>,
    control: &Control,
    source: &mut dyn Read,
    budget: &mut CorpusReadBudget,
) -> Result<Value, Error> {
    use anagram_core::corpus_ranking::PhraseCorpus;
    #[derive(Deserialize)]
    struct Case {
        answer: String,
        acceptable_orders: Option<Vec<String>>,
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Cases {
        List(Vec<Case>),
        Configured {
            cases: Vec<Case>,
            #[serde(default)]
            options: BuildOptions,
        },
    }
    #[derive(Deserialize, serde::Serialize)]
    #[serde(default, deny_unknown_fields)]
    struct BuildOptions {
        retained_orders: usize,
        phrase_bonus_max: f64,
    }
    impl Default for BuildOptions {
        fn default() -> Self {
            Self {
                retained_orders: 56,
                phrase_bonus_max: 10.0,
            }
        }
    }
    let (cases, options) = match input(control, source)? {
        Cases::List(cases) => (cases, BuildOptions::default()),
        Cases::Configured { cases, options } => (cases, options),
    };
    if options.retained_orders < 2
        || !options.phrase_bonus_max.is_finite()
        || options.phrase_bonus_max < 0.0
    {
        return Err(err(
            "Training pool requires at least two retained orders and a finite nonnegative phrase bonus",
        ));
    }
    let lex = anagram_core::wordnet::WordNet::load_identified_with_budget(wordnet, control, budget)
        .map_err(corpus_err)?
        .0;
    let phrase = phrase_path
        .map(|p| {
            anagram_core::phrase_index::PhraseIndex::open_controlled_with_budget(p, control, budget)
        })
        .transpose()
        .map_err(corpus_err)?;
    if let Some(index) = &phrase {
        index.validate_unique_keys().map_err(corpus_err)?;
    }
    let token = regex::Regex::new(r"[A-Za-z]+(?:['’][A-Za-z]+)?").unwrap();
    let normalize = |s: &str| {
        token
            .find_iter(s)
            .map(|m| anagram_core::normalize_letters(m.as_str()))
            .collect::<Vec<_>>()
    };
    let mut groups = Vec::new();
    let mut skipped = Vec::new();
    let mut bags = HashSet::new();
    for case in cases {
        control.check().map_err(err)?;
        let words = normalize(&case.answer);
        if !(2..=6).contains(&words.len()) {
            skipped.push(format!("{}: outside exact training range 2–6", case.answer));
            continue;
        }
        let mut bag = words.clone();
        bag.sort();
        let key = bag.join(" ");
        if !bags.insert(key.clone()) {
            return Err(err(
                "Duplicate word bag; combine its acceptable_orders in one case",
            ));
        }
        let acceptable: HashSet<_> = case
            .acceptable_orders
            .unwrap_or_else(|| vec![case.answer.clone()])
            .iter()
            .map(|s| normalize(s).join(" "))
            .collect();
        let (candidates, _) = anagram_core::ordering::rank_controlled(
            &words,
            &lex,
            true,
            256,
            options.retained_orders,
            control,
        )
        .map_err(err)?;
        let mut items = Vec::new();
        for c in candidates {
            let (score, details) = if let Some(p) = &phrase {
                anagram_core::phrase_evidence::score(
                    &c.order,
                    &p.counts(&anagram_core::phrase_evidence::queries(&c.order, p.max_n()))
                        .map_err(err)?,
                    p.max_n(),
                    true,
                )
            } else {
                (0.0, Default::default())
            };
            let order = c.order.join(" ");
            items.push(anagram_core::training::Sample {
                positive: acceptable.contains(&order),
                key: order,
                features: anagram_core::learned::features(&c, score, &details, words.len()),
                baseline_score: c.objective + (options.phrase_bonus_max / 100.0) * score,
            });
        }
        if !items.iter().any(|i| i.positive) || !items.iter().any(|i| !i.positive) {
            skipped.push(format!(
                "{}: retained pool needs both positive and negative orders",
                case.answer
            ));
            continue;
        }
        groups.push(anagram_core::training::Group { key, items });
    }
    Ok(json!({"groups":groups,"skipped":skipped,"build_options":options}))
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn cancellation_after_commit_preserves_success_but_classifies_intake_errors() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.codex/temp");
        std::fs::create_dir_all(&root).unwrap();
        let directory = tempfile::tempdir_in(root).unwrap();
        let output = directory.path().join("committed.json");
        let mut file = stage(&output).unwrap();
        file.write_all(b"synthetic committed output").unwrap();
        let control = Control::default();
        control.check().unwrap();
        file.persist_noclobber(&output).unwrap();
        control.cancel();
        assert!(finish_command(Ok(json!({"model_path": output})), &control).is_ok());
        assert_eq!(
            std::fs::read(output).unwrap(),
            b"synthetic committed output"
        );
        assert_eq!(
            finish_command(Err(err("intake failure")), &control)
                .unwrap_err()
                .code,
            "cancelled"
        );
    }
}
