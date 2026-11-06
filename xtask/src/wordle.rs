//! Reproducible native Wordle policy comparison; no desktop or external solver.
use crate::{Args, Result, write_json};
use anagram_core::{control::Control, lexicon::Unigrams, provenance::Snapshot, wordle::*};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Cursor, path::Path, time::Instant};
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    Some(
        v[((v.len() as f64 * p).ceil() as usize)
            .saturating_sub(1)
            .min(v.len() - 1)],
    )
}
fn latencies(v: &[f64]) -> serde_json::Value {
    json!({"samples":v.len(),"p50_ms":percentile(v,0.5),"p95_ms":percentile(v,0.95),"p99_ms":percentile(v,0.99)})
}
fn numbers(args: &Args, key: &str, default: &str) -> Result<Vec<usize>> {
    args.get(key)
        .unwrap_or(default)
        .split(',')
        .map(|n| n.parse::<usize>().map_err(Into::into))
        .collect()
}
pub fn command(raw: &[String]) -> Result<()> {
    if raw.iter().any(|a| a == "--help") {
        println!(
            "cargo xtask wordle-bench [--dictionary FILE --unigrams FILE --answers FILE --reference FILE --profile broad|common --samples 20 --attempts 6 --split evaluation|tuning --timeout-ms 2000 --target-ms 100 --endgames 12 --batches 256 --cache-bytes 8388608 --max-states 50000 --full --output FILE]\nComma-separated endgames/batches/cache-bytes sweep adaptive budgets; baselines run once. --reference measures coverage against an independently supplied answer list. --full scans the complete guess space, subject to deadline; timeout-ms 0 removes that deadline. Run release builds for meaningful timing."
        );
        return Ok(());
    }
    let args = Args::parse(
        raw,
        &[
            "dictionary",
            "unigrams",
            "answers",
            "reference",
            "profile",
            "samples",
            "attempts",
            "split",
            "timeout-ms",
            "target-ms",
            "endgames",
            "batches",
            "cache-bytes",
            "max-states",
            "output",
        ],
        &["full"],
    )?;
    let samples = usize::try_from(args.number("samples", 20)?)?;
    let attempts = usize::try_from(args.number("attempts", 6)?)?;
    if samples == 0 || !(1..=12).contains(&attempts) {
        return Err("Use positive samples and 1–12 attempts".into());
    }
    let split = args.get("split").unwrap_or("evaluation");
    if !["evaluation", "tuning"].contains(&split) {
        return Err("Split must be evaluation or tuning".into());
    }
    let control = Control::default();
    let load = Instant::now();
    let guesses = Snapshot::load(
        Path::new(
            args.get("dictionary")
                .unwrap_or(".anagram_data/dictionary/normal_user_v2.txt"),
        ),
        "wordle_guesses",
        &control,
    )?;
    let answers = args
        .get("answers")
        .map(|p| Snapshot::load(Path::new(p), "wordle_answers", &control))
        .transpose()?;
    let frequency_path = args
        .get("unigrams")
        .unwrap_or(".anagram_data/ngrams/count_1w.txt");
    let frequency = if Path::new(frequency_path).is_file() {
        Some(Snapshot::load(
            Path::new(frequency_path),
            "wordle_frequencies",
            &control,
        )?)
    } else {
        None
    };
    let unigrams = frequency
        .as_ref()
        .map(|f| Unigrams::load(Cursor::new(f.bytes())))
        .transpose()?;
    let kind = if answers.is_some() {
        ProfileKind::Custom
    } else {
        match args.get("profile").unwrap_or("broad") {
            "broad" => ProfileKind::Broad,
            "common" => ProfileKind::Common,
            _ => return Err("Unknown profile".into()),
        }
    };
    let mut sources = vec![guesses.identity().clone()];
    if let Some(a) = &answers {
        sources.push(a.identity().clone());
    }
    if let Some(f) = &frequency {
        sources.push(f.identity().clone());
    }
    let vocabulary = Vocabulary::load(
        kind,
        guesses.bytes(),
        answers.as_ref().map(Snapshot::bytes),
        unigrams.as_ref(),
        sources,
        &control,
    )?;
    let load_ms = load.elapsed().as_secs_f64() * 1000.0;
    let eligible: HashSet<_> = vocabulary
        .answers
        .iter()
        .map(|&i| vocabulary.words[i].word.clone())
        .collect();
    let reference = if let Some(path) = args.get("reference") {
        parse_words(&std::fs::read(path)?, &control)?
    } else {
        eligible.iter().cloned().collect()
    };
    let mut reports = Vec::new();
    let mut workloads = Vec::new();
    let endgames = numbers(&args, "endgames", "12")?;
    let batches = numbers(&args, "batches", "256")?;
    let caches = numbers(&args, "cache-bytes", "8388608")?;
    if endgames.is_empty() || batches.is_empty() || caches.is_empty() || batches.contains(&0) {
        return Err("Provide nonempty sweep settings and positive batches".into());
    }
    for length in 4..=8 {
        // Fixed disjoint split independent of frequency and strategy; seeded ordering is reproducible.
        let mut selected: Vec<_> = reference
            .iter()
            .filter(|w| w.len() == length)
            .filter(|w| {
                let hash = Sha256::digest(format!("wordle-workload-v1:{w}"));
                (hash[0] % 2 == 0) == (split == "evaluation")
            })
            .cloned()
            .collect();
        selected
            .sort_by_cached_key(|w| hex::encode(Sha256::digest(format!("wordle-order-v1:{w}"))));
        selected.truncate(samples);
        let workload_fingerprint = hex::encode(Sha256::digest(selected.join("\n")));
        workloads
            .push(json!({"length":length,"answers":selected,"fingerprint":workload_fingerprint}));
        for strategy in ["frequency", "entropy", "adaptive"] {
            for policy in [GuessPolicy::AnswersOnly, GuessPolicy::Unrestricted] {
                for (ei, &endgame) in endgames.iter().enumerate() {
                    for (bi, &batch) in batches.iter().enumerate() {
                        for (ci, &cache_bytes) in caches.iter().enumerate() {
                            if strategy != "adaptive" && (ei > 0 || bi > 0 || ci > 0) {
                                continue;
                            }
                            let budget = SearchBudget {
                                timeout_ms: args.number("timeout-ms", 2000)?,
                                target_ms: args.number("target-ms", 100)?,
                                endgame_candidates: endgame,
                                initial_batch: batch,
                                cache_bytes,
                                max_states: usize::try_from(args.number("max-states", 50_000)?)?,
                                full_search: args.has("full"),
                            };
                            let (
                                mut warm,
                                mut openings,
                                mut successes,
                                mut missing,
                                mut exact,
                                mut calls,
                            ) = (Vec::new(), Vec::new(), Vec::new(), 0, 0, 0);
                            let (mut endgame_attempts, mut endgame_completed) = (0, 0);
                            let mut games = Vec::new();
                            for answer in &selected {
                                if !eligible.contains(answer) {
                                    missing += 1;
                                    games.push(json!({"answer":answer,"outcome":"missing_answer","guesses":[]}));
                                    continue;
                                }
                                let mut history = Vec::new();
                                let mut solved = false;
                                let mut guesses = Vec::new();
                                let mut steps = Vec::new();
                                for turn in 0..attempts {
                                    let request = AnalysisRequest {
                                        length,
                                        attempts_remaining: attempts - turn,
                                        history: history.clone(),
                                        budget: budget.clone(),
                                        guess_policy: policy,
                                        strategy: if strategy == "entropy" {
                                            Strategy::Entropy
                                        } else {
                                            Strategy::Adaptive
                                        },
                                        ..Default::default()
                                    };
                                    let start = Instant::now();
                                    let (mut result, ids) =
                                        filter(&vocabulary, &request, &control)?;
                                    let endgame = strategy == "adaptive"
                                        && ids.len() > 1
                                        && ids.len() <= endgame
                                        && request.attempts_remaining > 2;
                                    if endgame {
                                        endgame_attempts += 1;
                                    }
                                    let guess = if strategy == "frequency" {
                                        ids.first().map(|&i| vocabulary.words[i].word.clone())
                                    } else {
                                        recommend(
                                            &vocabulary,
                                            &request,
                                            &ids,
                                            &control,
                                            &mut result,
                                        )?;
                                        result.strategic.as_ref().map(|r| r.word.clone())
                                    };
                                    let ms = start.elapsed().as_secs_f64() * 1000.0;
                                    if turn == 0 {
                                        openings.push(ms);
                                    } else {
                                        warm.push(ms);
                                    }
                                    calls += 1;
                                    if result.optimal_for_profile {
                                        exact += 1;
                                        if endgame {
                                            endgame_completed += 1;
                                        }
                                    }
                                    steps.push(json!({"candidate_count":ids.len(),"attempts_remaining":request.attempts_remaining,"latency_ms":ms,"method":result.method,"optimal_for_profile":result.optimal_for_profile,"evaluated_guesses":result.evaluated_guesses,"expanded_states":result.expanded_states}));
                                    let Some(guess) = guess else {
                                        break;
                                    };
                                    guesses.push(guess.clone());
                                    if &guess == answer {
                                        solved = true;
                                        successes.push(turn + 1);
                                        break;
                                    }
                                    history.push(HistoryRow {
                                        feedback: feedback(answer, &guess)?,
                                        word: guess,
                                    });
                                }
                                games.push(json!({"answer":answer,"outcome":if solved{"solved"}else{"attempt_limit"},"guesses":guesses,"steps":steps}));
                            }
                            let covered = selected.len() - missing;
                            let failures = covered - successes.len();
                            let cost = successes.iter().sum::<usize>()
                                + (failures + missing) * (attempts + 1);
                            reports.push(json!({"length":length,"strategy":strategy,"guess_policy":policy,"budget":budget,"workload_fingerprint":workload_fingerprint,"games":games,"answers":selected.len(),"covered_answers":covered,"missing_answers":missing,"vocabulary_coverage":if selected.is_empty(){None}else{Some(covered as f64/selected.len() as f64)},"solve_rate":if selected.is_empty(){None}else{Some(successes.len() as f64/selected.len() as f64)},"covered_solve_rate":if covered==0{None}else{Some(successes.len() as f64/covered as f64)},"mean_successful_guesses":if successes.is_empty(){None}else{Some(successes.iter().sum::<usize>() as f64/successes.len() as f64)},"maximum_successful_guesses":successes.iter().max(),"strategy_failures":failures,"penalized_expected_cost":if selected.is_empty(){None}else{Some(cost as f64/selected.len() as f64)},"recommendations":calls,"profile_optimal_rate":if calls==0{None}else{Some(exact as f64/calls as f64)},"endgame_attempts":endgame_attempts,"endgame_completions":endgame_completed,"exact_completion_rate":if endgame_attempts==0{None}else{Some(endgame_completed as f64/endgame_attempts as f64)},"opening_latency":latencies(&openings),"warm_latency":latencies(&warm)}));
                        }
                    }
                }
            }
        }
    }
    let report = json!({"schema_version":1,"engine":"wordle-v1","engine_id":ENGINE_ID,"benchmark_tool_id":hex::encode(Sha256::digest(include_bytes!("wordle.rs"))),"profile":vocabulary.profile,"split":split,"attempt_limit":attempts,"vocabulary_load_ms":load_ms,"workloads":workloads,"results":reports,"notes":["Uniform answer model; frequency only orders/ties.","Coverage is relative to the supplied reference, or the selected answer pool when omitted.","Percentiles from small samples are diagnostic; use release builds and adequate samples for performance conclusions.","Frequency baseline is inherently answer-only under either guess-policy setting."]});
    write_json(args.get("output"), &report)
}
