use super::{ProfileKind, VocabularyProfile};
use crate::{contracts::DataIdentity, control::Control, lexicon::Unigrams};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};

pub fn validate_word(text: &str) -> Result<String, String> {
    if !(4..=8).contains(&text.len()) || !text.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err("Use 4–8 ASCII English letters without punctuation".into());
    }
    Ok(text.to_ascii_lowercase())
}

/// Validate original bytes. Never pass through the anagram lossy decoder/normalizer.
pub fn parse_words(bytes: &[u8], control: &Control) -> Result<BTreeSet<String>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| {
        let line = bytes[..e.valid_up_to()]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
            + 1;
        format!("Line {line}: malformed UTF-8")
    })?;
    let mut words = BTreeSet::new();
    for (i, line) in text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .lines()
        .enumerate()
    {
        control.check().map_err(str::to_owned)?;
        let word = line.trim();
        if word.is_empty() {
            continue;
        }
        if !word.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(format!("Line {}: expected English letters only", i + 1));
        }
        // Dictionaries may contain other lengths; eligibility is a separate policy.
        if (4..=8).contains(&word.len()) {
            words.insert(word.to_ascii_lowercase());
        }
    }
    Ok(words)
}

#[derive(Clone)]
pub struct Entry {
    pub word: String,
    pub frequency: f64,
}
pub struct Vocabulary {
    pub profile: VocabularyProfile,
    pub words: Vec<Entry>,
    pub answers: Vec<usize>,
    pub(crate) lookup: HashMap<String, usize>,
}
impl Vocabulary {
    pub fn load(
        kind: ProfileKind,
        guesses: &[u8],
        answers: Option<&[u8]>,
        frequencies: Option<&Unigrams>,
        sources: Vec<DataIdentity>,
        control: &Control,
    ) -> Result<Self, String> {
        let mut allowed = parse_words(guesses, control)?;
        let eligible = match kind {
            ProfileKind::Broad => allowed.clone(),
            ProfileKind::Common => {
                let frequencies = frequencies.ok_or("Common English requires unigram frequency data. Choose Broad English or configure frequencies.")?;
                allowed
                    .iter()
                    .filter(|word| frequencies.zipf(word) >= 4.0)
                    .cloned()
                    .collect()
            }
            ProfileKind::Custom => {
                parse_words(answers.ok_or("Choose a custom answer list")?, control)?
            }
        };
        allowed.extend(eligible.iter().cloned());
        if eligible.is_empty() {
            return Err("The selected profile has no eligible 4–8 letter answers".into());
        }
        let words: Vec<_> = allowed
            .into_iter()
            .map(|word| Entry {
                frequency: frequencies.map_or(0.0, |f| f.zipf(&word)),
                word,
            })
            .collect();
        let lookup: HashMap<_, _> = words
            .iter()
            .enumerate()
            .map(|(i, w)| (w.word.clone(), i))
            .collect();
        let answers: Vec<_> = eligible.iter().map(|w| lookup[w]).collect();
        let mut hash = Sha256::new();
        hash.update(format!("wordle-v1:{kind:?}:uniform\n"));
        for w in &words {
            hash.update(format!("g:{}:{:016x}\n", w.word, w.frequency.to_bits()));
        }
        for &i in &answers {
            hash.update(format!("a:{}\n", words[i].word));
        }
        hash.update(serde_json::to_vec(&sources).map_err(|e| e.to_string())?);
        let profile = VocabularyProfile {
            schema_version: 1,
            kind,
            name: match kind {
                ProfileKind::Broad => "Broad English",
                ProfileKind::Common => "Common English · Zipf ≥4",
                ProfileKind::Custom => "Custom",
            }
            .into(),
            fingerprint: hex::encode(hash.finalize()),
            sources,
            probability_model: "uniform".into(),
            allowed_guesses: words.len(),
            eligible_answers: answers.len(),
            frequency_available: frequencies.is_some(),
        };
        Ok(Self {
            profile,
            words,
            answers,
            lookup,
        })
    }
    pub fn contains(&self, word: &str) -> bool {
        self.lookup.contains_key(&word.to_ascii_lowercase())
    }
}
