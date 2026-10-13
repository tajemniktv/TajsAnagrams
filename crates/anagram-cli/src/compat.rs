//! Native equivalents of useful legacy input/report/export conveniences.
use anagram_core::{control::Control, normalize_letters, request::Error};
use serde_json::{Value, json};
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Default)]
pub struct Options {
    pub subtract: Vec<String>,
    pub exclude_files: Vec<PathBuf>,
    pub stream: Option<PathBuf>,
    export: Option<PathBuf>,
    deep_export: Option<PathBuf>,
    answer: Option<String>,
    hint_stats: bool,
}

impl Options {
    pub fn validate_mode(&self, command: Option<&str>) -> Result<(), Error> {
        if self.deep_export.is_some() && command != Some("solve") {
            return Err(Error::new("usage", "--deep-export requires solve"));
        }
        if self.stream.is_some() && command != Some("generate") {
            return Err(Error::new("usage", "--stream-export requires generate"));
        }
        if !matches!(command, Some("generate" | "solve"))
            && (!self.subtract.is_empty()
                || !self.exclude_files.is_empty()
                || self.export.is_some()
                || self.answer.is_some()
                || self.hint_stats)
        {
            return Err(Error::new(
                "usage",
                "Input/report/export options require generate or solve",
            ));
        }
        Ok(())
    }
    pub fn generated_reports(
        &self,
        result: &anagram_core::request::Generated,
        request: &anagram_core::request::GenerateRequest,
        output: &mut Value,
    ) {
        if self.hint_stats {
            let counts: std::collections::BTreeMap<_, _> = request
                .hints
                .iter()
                .map(|hint| {
                    let word = normalize_letters(hint);
                    let count = result.bags.iter().filter(|bag| bag.contains(&word)).count();
                    (word, count)
                })
                .collect();
            output["hint_stats"] = json!({"scope":"returned_bags","counts":counts});
        }
        if let Some(answer) = &self.answer {
            let mut wanted: Vec<_> = answer.split_whitespace().map(normalize_letters).collect();
            wanted.sort();
            if wanted.is_empty() || wanted.iter().any(String::is_empty) {
                return;
            }
            let matches: Vec<_> = result
                .bags
                .iter()
                .enumerate()
                .filter_map(|(index, bag)| {
                    let mut words = bag.clone();
                    words.sort();
                    (words == wanted).then_some(index + 1)
                })
                .collect();
            output["benchmark"] = json!({"scope":"returned_bags","matches":matches});
        }
    }

    pub fn take(args: &mut Vec<OsString>) -> Result<Self, Error> {
        let mut options = Self::default();
        for (flag, repeat) in [
            ("--subtract", true),
            ("--exclude-file", true),
            ("--stream-export", false),
            ("--export", false),
            ("--deep-export", false),
            ("--benchmark-answer", false),
        ] {
            let mut seen = false;
            while let Some(index) = args.iter().position(|v| v == flag) {
                if seen && !repeat {
                    return Err(Error::new("usage", format!("Duplicate {flag}")));
                }
                let value = args
                    .get(index + 1)
                    .filter(|v| !v.to_string_lossy().starts_with("--"))
                    .cloned()
                    .ok_or_else(|| Error::new("usage", format!("{flag} requires a value")))?;
                args.drain(index..=index + 1);
                match flag {
                    "--subtract" => options.subtract.push(
                        value
                            .into_string()
                            .map_err(|_| Error::new("usage", "Subtraction must be UTF-8"))?,
                    ),
                    "--exclude-file" => options.exclude_files.push(value.into()),
                    "--stream-export" => options.stream = Some(value.into()),
                    "--export" => options.export = Some(value.into()),
                    "--deep-export" => options.deep_export = Some(value.into()),
                    "--benchmark-answer" => {
                        options.answer = Some(
                            value
                                .into_string()
                                .map_err(|_| Error::new("usage", "Answer must be UTF-8"))?,
                        )
                    }
                    _ => unreachable!(),
                }
                seen = true;
            }
        }
        if let Some(index) = args.iter().position(|v| v == "--hint-stats") {
            args.remove(index);
            options.hint_stats = true;
        }
        Ok(options)
    }

    pub fn apply(&self, input: &str, ranked: bool, control: &Control) -> Result<Value, Error> {
        let mut request: Value =
            serde_json::from_str(input).map_err(|e| Error::new("invalid_json", e.to_string()))?;
        let generation = if ranked {
            &mut request["generation"]
        } else {
            &mut request
        };
        if !generation.is_object() {
            return Err(Error::new("invalid_json", "Expected a generation object"));
        }
        let append = |target: &mut Value, name: &str, words: Vec<String>| -> Result<(), Error> {
            if words.is_empty() {
                return Ok(());
            }
            let value = target
                .as_object_mut()
                .unwrap()
                .entry(name)
                .or_insert_with(|| json!([]));
            let array = value
                .as_array_mut()
                .ok_or_else(|| Error::new("invalid_json", format!("{name} must be an array")))?;
            array.extend(words.into_iter().map(Value::String));
            Ok(())
        };
        append(generation, "subtract", self.subtract.clone())?;
        let mut total = 0usize;
        for path in &self.exclude_files {
            let file = File::open(path).map_err(|e| Error::new("input_error", e.to_string()))?;
            let mut bytes = Vec::new();
            control
                .reader(file)
                .take((1_048_577 - total) as u64)
                .read_to_end(&mut bytes)
                .map_err(|e| Error::new("input_error", e.to_string()))?;
            total += bytes.len();
            if total > 1_048_576 {
                return Err(Error::new(
                    "request_too_large",
                    "Exclusion files exceed 1 MiB in aggregate",
                ));
            }
            append(
                generation,
                "excluded",
                String::from_utf8_lossy(&bytes)
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect(),
            )?;
        }
        Ok(request)
    }

    pub fn finish(&self, result: &mut Value) -> Result<(), Error> {
        // Reports are computed after solving; labels never influence scoring.
        if self.hint_stats && result["kind"] == "ranked" {
            let mut counts = std::collections::BTreeMap::<String, usize>::new();
            if let Some(buckets) = result["buckets"].as_object() {
                for rows in buckets.values().filter_map(Value::as_array) {
                    for row in rows {
                        for hint in row["hints"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                        {
                            *counts.entry(hint.to_owned()).or_default() += 1;
                        }
                    }
                }
            }
            result["hint_stats"] = json!({"scope":"returned_rows", "counts":counts});
        }
        if let Some(answer) = &self.answer {
            let wanted = normalize_letters(answer);
            if wanted.is_empty() {
                return Err(Error::new(
                    "invalid_word",
                    "Benchmark answer contains no A-Z letters",
                ));
            }
            let mut matches = Vec::new();
            if let Some(buckets) = result["buckets"].as_object() {
                for (count, rows) in buckets {
                    for (index, row) in rows.as_array().into_iter().flatten().enumerate() {
                        if normalize_letters(row["display_phrase"].as_str().unwrap_or("")) == wanted
                        {
                            matches.push(json!({"word_count":count,"rank":index+1}));
                        }
                    }
                }
            }
            if result["kind"] == "ranked" {
                result["benchmark"] = json!({"scope":"returned_rows","matches":matches});
            }
        }
        if let Some(path) = &self.export {
            write_new(path, result)?;
        }
        if let Some(path) = &self.deep_export {
            if result["kind"] != "ranked" {
                return Err(Error::new("usage", "--deep-export requires solve"));
            }
            let mut deep = result.clone();
            if let Some(buckets) = deep["buckets"].as_object_mut() {
                for rows in buckets.values_mut().filter_map(Value::as_array_mut) {
                    rows.retain(|row| row["deep"] == true);
                }
            }
            // An export filter is not a recomputed solve; retain original counters.
            deep["export_scope"] = "deep_returned_rows".into();
            write_new(path, &deep)?;
        }
        Ok(())
    }
}

pub fn open_new(path: &PathBuf) -> Result<File, Error> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| Error::new("export_error", e.to_string()))
}
fn write_new(path: &PathBuf, value: &Value) -> Result<(), Error> {
    let mut file = open_new(path)?;
    serde_json::to_writer(&mut file, value)
        .map_err(|e| Error::new("export_error", e.to_string()))?;
    file.write_all(b"\n")
        .and_then(|_| file.sync_all())
        .map_err(|e| Error::new("export_error", e.to_string()))
}
