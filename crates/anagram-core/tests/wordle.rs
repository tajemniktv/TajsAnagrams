use anagram_core::{control::Control, wordle::*};
fn vocab(guesses: &str, answers: Option<&str>) -> Vocabulary {
    Vocabulary::load(
        if answers.is_some() {
            ProfileKind::Custom
        } else {
            ProfileKind::Broad
        },
        guesses.as_bytes(),
        answers.map(str::as_bytes),
        None,
        vec![],
        &Control::default(),
    )
    .unwrap()
}
fn run(v: &Vocabulary, r: &AnalysisRequest) -> Analysis {
    analyze(v, r, &Control::default(), |_| {}).unwrap()
}
#[test]
fn objective_describes_the_actual_ranking_path() {
    let v = vocab("aaaa\naaab\naaba\nabaa\nbaaa", None);
    let mut r = AnalysisRequest {
        length: 4,
        attempts_remaining: 2,
        ..Default::default()
    };
    assert_eq!(run(&v, &r).objective, "win_probability_then_penalized_cost");
    r.strategy = Strategy::Entropy;
    assert_eq!(
        run(&v, &r).objective,
        "information_gain_then_expected_remaining"
    );
    r.attempts_remaining = 0;
    assert_eq!(run(&v, &r).objective, "none");
    assert_eq!(
        filter(&v, &r, &Control::default()).unwrap().0.objective,
        "candidate_filtering"
    );
}

// Independent positional consumption oracle; no pattern encoding or count-table reuse.
fn oracle(answer: &str, guess: &str) -> Vec<Mark> {
    let mut available: Vec<_> = answer.chars().map(Some).collect();
    let g: Vec<_> = guess.chars().collect();
    let mut out = vec![Mark::Gray; g.len()];
    for i in 0..g.len() {
        if available[i] == Some(g[i]) {
            out[i] = Mark::Green;
            available[i] = None;
        }
    }
    for i in 0..g.len() {
        if out[i] == Mark::Green {
            continue;
        }
        if let Some(p) = available.iter().position(|&a| a == Some(g[i])) {
            out[i] = Mark::Yellow;
            available[p] = None;
        }
    }
    out
}
fn binary_words(n: usize) -> Vec<String> {
    (0..(1 << n))
        .map(|mask| {
            (0..n)
                .map(|i| if mask & (1 << i) == 0 { 'a' } else { 'b' })
                .collect()
        })
        .collect()
}
#[test]
fn duplicate_oracle_and_replay_for_every_supported_length() {
    for length in 4..=8 {
        let words = binary_words(length);
        let v = vocab(&words.join("\n"), None);
        for answer in &words {
            for guess in &words {
                assert_eq!(feedback(answer, guess).unwrap(), oracle(answer, guess));
            }
        }
        for answer in &words {
            let history = vec![HistoryRow {
                word: words[0].clone(),
                feedback: oracle(answer, &words[0]),
            }];
            let (result, _) = filter(
                &v,
                &AnalysisRequest {
                    length,
                    history,
                    ..Default::default()
                },
                &Control::default(),
            )
            .unwrap();
            assert!(result.candidate_count > 0);
        }
    }
    assert_eq!(
        feedback("apple", "alley").unwrap(),
        vec![
            Mark::Green,
            Mark::Yellow,
            Mark::Gray,
            Mark::Yellow,
            Mark::Gray
        ]
    );
}
#[test]
fn partial_feedback_matches_only_known_tiles_against_the_independent_oracle() {
    for length in 4..=8 {
        let words = binary_words(length);
        let v = vocab(&words.join("\n"), None);
        let all = (1usize << length) - 1;
        let mut masks = vec![0, all, 0b10101010 & all];
        for i in 0..length {
            masks.extend([1 << i, all ^ (1 << i)]);
        }
        masks.sort_unstable();
        masks.dedup();
        for guess in [&words[0], &words[words.len() / 3], &words[words.len() - 1]] {
            for answer in [&words[0], &words[words.len() / 2], &words[words.len() - 1]] {
                let observed = oracle(answer, guess);
                for &mask in &masks {
                    let partial: Vec<_> = observed
                        .iter()
                        .enumerate()
                        .map(|(i, &mark)| (mask & (1 << i) != 0).then_some(mark))
                        .collect();
                    let request = AnalysisRequest {
                        length,
                        partial_history: vec![PartialHistoryRow {
                            word: guess.clone(),
                            feedback: partial.clone(),
                        }],
                        ..Default::default()
                    };
                    let (_, ids) = filter(&v, &request, &Control::default()).unwrap();
                    let actual: std::collections::BTreeSet<_> =
                        ids.iter().map(|&id| v.words[id].word.clone()).collect();
                    let expected: std::collections::BTreeSet<_> = words
                        .iter()
                        .filter(|candidate| {
                            oracle(candidate, guess)
                                .iter()
                                .zip(&partial)
                                .all(|(&mark, known)| known.is_none_or(|known| known == mark))
                        })
                        .cloned()
                        .collect();
                    assert_eq!(
                        actual, expected,
                        "length {length}, guess {guess}, mask {mask}"
                    );
                }
            }
        }
    }
}

#[test]
fn partial_duplicate_gray_does_not_exclude_an_unknown_positive_duplicate() {
    let v = vocab("apple\nalley\nallay\nample", None);
    let request = AnalysisRequest {
        partial_history: vec![PartialHistoryRow {
            word: "alley".into(),
            feedback: vec![None, None, Some(Mark::Gray), None, None],
        }],
        ..Default::default()
    };
    let (a, _) = filter(&v, &request, &Control::default()).unwrap();
    assert!(a.candidates.contains(&"apple".into()));
    assert!(!a.candidates.contains(&"allay".into()));
    let mut invalid = request.clone();
    invalid.partial_history[0].feedback.pop();
    assert!(filter(&v, &invalid, &Control::default()).is_err());
    invalid = request.clone();
    invalid.partial_history = (0..13)
        .map(|_| request.partial_history[0].clone())
        .collect();
    assert!(filter(&v, &invalid, &Control::default()).is_err());
    let mut conflicting = request;
    conflicting.constraints.fixed = vec![None, None, Some('l')];
    assert_eq!(
        filter(&v, &conflicting, &Control::default())
            .unwrap()
            .0
            .diagnostic
            .as_deref(),
        Some("contradictory_clues")
    );
}
#[test]
fn impossible_yellows_are_contradictory_even_with_partial_feedback() {
    let v = vocab("jelly\njolly\nabout\napple\nareas", None);
    for word in ["jjjjj", "aaaaa"] {
        for position in 0..5 {
            let mut marks = vec![None; 5];
            marks[position] = Some(Mark::Yellow);
            let request = AnalysisRequest {
                partial_history: vec![PartialHistoryRow {
                    word: word.into(),
                    feedback: marks,
                }],
                ..Default::default()
            };
            let (a, _) = filter(&v, &request, &Control::default()).unwrap();
            assert_eq!(a.diagnostic.as_deref(), Some("contradictory_clues"));
            assert!(
                a.notices
                    .iter()
                    .any(|notice| notice.contains("no possible unmatched position"))
            );
        }
    }
    // This yellow A can fit the final position and must not be called a conflict.
    let mut request = AnalysisRequest {
        partial_history: vec![PartialHistoryRow {
            word: "aaaab".into(),
            feedback: vec![Some(Mark::Yellow), None, None, None, None],
        }],
        ..Default::default()
    };
    assert_ne!(
        filter(&v, &request, &Control::default())
            .unwrap()
            .0
            .diagnostic
            .as_deref(),
        Some("contradictory_clues")
    );
    request.constraints.fixed = vec![None, None, None, None, Some('b')];
    assert_eq!(
        filter(&v, &request, &Control::default())
            .unwrap()
            .0
            .diagnostic
            .as_deref(),
        Some("contradictory_clues")
    );
}
#[test]
fn importer_validates_original_bytes_and_profile_identity() {
    let c = Control::default();
    assert!(
        parse_words(b"apple\ncan\xfft", &c)
            .unwrap_err()
            .contains("Line 2")
    );
    for word in ["can't", "co-op", "école", "ＡＢＣＤＥ"] {
        assert!(parse_words(word.as_bytes(), &c).is_err());
    }
    assert_eq!(
        parse_words("\u{feff} APPLE \n\nApple\n".as_bytes(), &c)
            .unwrap()
            .len(),
        1
    );
    let a = vocab("apple\nalley", Some("apple"));
    let b = vocab("apple\nalley", Some("alley"));
    assert_ne!(a.profile.fingerprint, b.profile.fingerprint);
    assert!(vocab("alley", Some("apple")).contains("apple"));
    assert!(Vocabulary::load(ProfileKind::Common, b"apple", None, None, vec![], &c).is_err());
}

#[test]
fn common_profile_changes_answer_eligibility_without_changing_allowed_guesses() {
    let frequencies = anagram_core::lexicon::Unigrams::load(std::io::Cursor::new(
        b"apple\t1000000\nother\t999000000\nalley\t1\n",
    ))
    .unwrap();
    let v = Vocabulary::load(
        ProfileKind::Common,
        b"apple\nother\nalley",
        None,
        Some(&frequencies),
        vec![],
        &Control::default(),
    )
    .unwrap();
    assert!(v.contains("alley"));
    let result = run(&v, &AnalysisRequest::default());
    assert_eq!(result.candidate_count, 2);
    assert!(!result.candidates.contains(&"alley".to_owned()));
    assert_eq!(result.profile.probability_model, "uniform");
}
#[test]
fn contradictions_differ_from_missing_answers_and_edits_replay() {
    let v = vocab("apple\nalley\nother", None);
    let mut r = AnalysisRequest::default();
    r.constraints.fixed = vec![Some('z')];
    assert_eq!(
        run(&v, &r).diagnostic.as_deref(),
        Some("no_profile_matches")
    );
    r.constraints.counts = vec![LetterBound {
        letter: "z".into(),
        min: 0,
        max: Some(0),
    }];
    assert_eq!(
        run(&v, &r).diagnostic.as_deref(),
        Some("contradictory_clues")
    );
    r.constraints = Default::default();
    r.history = vec![HistoryRow {
        word: "alley".into(),
        feedback: feedback("apple", "alley").unwrap(),
    }];
    assert_eq!(run(&v, &r).candidates, vec!["apple"]);
    r.history.clear();
    assert_eq!(run(&v, &r).candidate_count, 3);
    r.constraints.counts = vec![LetterBound {
        letter: "p".into(),
        min: 2,
        max: Some(2),
    }];
    assert_eq!(run(&v, &r).candidates, vec!["apple"]);
}
#[test]
fn probes_improve_two_turns_but_never_the_final_turn() {
    let answers = "bound\nfound\nhound\nmound\npound\nround\nsound\nwound";
    let v = vocab(&format!("{answers}\nswamp"), Some(answers));
    let r = AnalysisRequest {
        attempts_remaining: 2,
        budget: SearchBudget {
            timeout_ms: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    let a = run(&v, &r);
    assert_eq!(a.strategic.as_ref().unwrap().word, "swamp");
    assert_eq!(a.strategic.unwrap().solve_probability, Some(5.0 / 8.0));
    assert!(a.optimal_for_profile);
    let restricted = run(
        &v,
        &AnalysisRequest {
            guess_policy: GuessPolicy::AnswersOnly,
            ..r.clone()
        },
    );
    assert_eq!(
        restricted.strategic.unwrap().solve_probability,
        Some(2.0 / 8.0)
    );
    let last = run(
        &v,
        &AnalysisRequest {
            attempts_remaining: 1,
            ..r
        },
    );
    assert_ne!(last.strategic.unwrap().word, "swamp");
}
// Independent brute force: enumerate full policies without memoization or action reduction.
fn brute(answers: &[String], guesses: &[String], turns: usize, only: bool) -> (usize, usize) {
    if turns == 0 {
        return (0, answers.len());
    }
    let mut best = (0, usize::MAX);
    for g in guesses {
        if only && !answers.contains(g) {
            continue;
        }
        let mut groups = std::collections::BTreeMap::<String, Vec<String>>::new();
        let mut wins = 0;
        for a in answers {
            if a == g {
                wins += 1;
            } else {
                groups
                    .entry(format!("{:?}", oracle(a, g)))
                    .or_default()
                    .push(a.clone());
            }
        }
        let mut cost = answers.len();
        for group in groups.values() {
            let (w, c) = brute(group, guesses, turns - 1, only);
            wins += w;
            cost += c;
        }
        if wins > best.0 || (wins == best.0 && cost < best.1) {
            best = (wins, cost);
        }
    }
    best
}
#[test]
fn exact_search_matches_full_policy_oracle_with_and_without_cache() {
    let guesses = vec![
        "aaaaa", "aaaab", "aaaba", "aabaa", "abaaa", "baaaa", "bbbbb",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for mask in [7, 15, 31] {
        let answers: Vec<_> = guesses[..5]
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, w)| w.clone())
            .collect();
        let v = vocab(&guesses.join("\n"), Some(&answers.join("\n")));
        for only in [false, true] {
            for turns in 1..=3 {
                for cache_bytes in [0, 1024 * 1024] {
                    let expected = brute(&answers, &guesses, turns, only);
                    let result = run(
                        &v,
                        &AnalysisRequest {
                            attempts_remaining: turns,
                            guess_policy: if only {
                                GuessPolicy::AnswersOnly
                            } else {
                                GuessPolicy::Unrestricted
                            },
                            budget: SearchBudget {
                                timeout_ms: 0,
                                cache_bytes,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    );
                    assert!(result.optimal_for_profile);
                    let rec = result.strategic.unwrap();
                    assert_eq!(
                        rec.solve_probability,
                        Some(expected.0 as f64 / answers.len() as f64)
                    );
                    assert_eq!(
                        rec.penalized_expected_cost,
                        Some(expected.1 as f64 / answers.len() as f64)
                    );
                }
            }
        }
    }
}
#[test]
fn budget_fallback_never_claims_exact_and_cancellation_is_an_error() {
    let v = vocab(
        "bound\nfound\nhound\nmound\npound\nround\nsound\nwound\nswamp",
        None,
    );
    let r = AnalysisRequest {
        budget: SearchBudget {
            max_states: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let a = run(&v, &r);
    assert!(!a.optimal_for_profile);
    assert!(a.strategic.is_some());
    let control = Control::default();
    control.cancel();
    assert!(analyze(&v, &r, &control, |_| {}).is_err());
}

#[test]
fn endgame_ablation_retains_adaptive_two_turn_rules_and_heuristic_fallback() {
    let answers = "bound\nfound\nhound\nmound\npound\nround\nsound\nwound";
    let v = vocab(&format!("{answers}\nswamp"), Some(answers));
    let request = AnalysisRequest {
        attempts_remaining: 3,
        budget: SearchBudget {
            exact_endgame: false,
            timeout_ms: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    let result = run(&v, &request);
    assert_eq!(result.expanded_states, 0);
    assert_eq!(result.method, "entropy_heuristic");
    assert!(!result.optimal_for_profile);
    let two = run(
        &v,
        &AnalysisRequest {
            attempts_remaining: 2,
            ..request
        },
    );
    assert_eq!(two.strategic.unwrap().solve_probability, Some(5.0 / 8.0));
    assert!(two.optimal_for_profile);
}
#[test]
fn external_pinned_feedback_fixtures() {
    for (i, line) in include_str!("../../../tests/wordle/fixtures/tests.txt")
        .lines()
        .enumerate()
    {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 3);
        let expected: Vec<_> = fields[2]
            .chars()
            .map(|c| match c {
                'c' => Mark::Green,
                'm' => Mark::Yellow,
                'w' => Mark::Gray,
                _ => panic!("fixture encoding"),
            })
            .collect();
        assert_eq!(
            feedback(fields[0], fields[1]).unwrap(),
            expected,
            "fixture {}",
            i + 1
        );
    }
}

#[test]
fn small_seeds_widen_in_two_turn_search_and_without_exact_search() {
    let answers = "bound\nfound\nhound\nmound\npound\nround\nsound\nwound";
    let v = vocab(&format!("{answers}\nswamp"), Some(answers));
    for initial_batch in [1, 2, 256] {
        for turns in [2, 3] {
            let result = run(
                &v,
                &AnalysisRequest {
                    attempts_remaining: turns,
                    budget: SearchBudget {
                        initial_batch,
                        exact_endgame: false,
                        full_search: true,
                        target_ms: 0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            assert_eq!(result.evaluated_guesses, v.words.len());
            assert!(result.guess_space_complete);
            assert_eq!(result.strategic.unwrap().word, "swamp");
            assert_eq!(result.optimal_for_profile, turns == 2);
        }
    }
}

#[test]
fn unfinished_exact_search_resumes_widening_in_coverage_order() {
    let answers = "bound\nfound\nhound\nmound\npound\nround\nsound\nwound";
    let v = vocab(&format!("{answers}\nswamp"), Some(answers));
    let result = run(
        &v,
        &AnalysisRequest {
            attempts_remaining: 3,
            budget: SearchBudget {
                initial_batch: 1,
                max_states: 1,
                timeout_ms: 0,
                target_ms: 10_000,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.method, "budget_limited_heuristic");
    assert!(result.expanded_states > 0);
    assert_eq!(result.evaluated_guesses, v.words.len());
    assert!(result.guess_space_complete);
    assert!(!result.optimal_for_profile);
    assert_eq!(result.strategic.unwrap().word, "swamp");
}

#[test]
fn full_search_crosses_256_guesses_but_soft_stop_remains_honest() {
    let answers = "bound\nfound\nhound\nmound\npound\nround\nsound\nwound";
    let mut words: Vec<String> = (0..1024)
        .map(|mut n| {
            (0..5)
                .map(|_| {
                    let b = b'a' + (n % 4) as u8;
                    n /= 4;
                    char::from(b)
                })
                .collect()
        })
        .collect();
    words.push(answers.into());
    words.push("swamp".into());
    let v = vocab(&words.join("\n"), Some(answers));
    let r = AnalysisRequest {
        attempts_remaining: 2,
        budget: SearchBudget {
            initial_batch: 256,
            target_ms: 0,
            ..Default::default()
        },
        ..Default::default()
    };
    let limited = run(&v, &r);
    assert_eq!(limited.evaluated_guesses, 1);
    assert!(limited.strategic.is_some());
    assert!(!limited.guess_space_complete);
    assert!(!limited.optimal_for_profile);
    let full = run(
        &v,
        &AnalysisRequest {
            budget: SearchBudget {
                full_search: true,
                ..r.budget.clone()
            },
            ..r
        },
    );
    assert!(full.evaluated_guesses > 256);
    assert_eq!(full.evaluated_guesses, v.words.len());
    assert!(full.guess_space_complete && full.optimal_for_profile);
    assert_eq!(full.strategic.unwrap().solve_probability, Some(5.0 / 8.0));
}

#[test]
fn direct_recommendation_rejects_zero_batches_and_expired_callers() {
    let v = vocab("apple\nalley", None);
    let request = AnalysisRequest::default();
    let (mut result, ids) = filter(&v, &request, &Control::default()).unwrap();
    let invalid = AnalysisRequest {
        budget: SearchBudget {
            initial_batch: 0,
            ..Default::default()
        },
        ..request.clone()
    };
    assert!(recommend(&v, &invalid, &ids, &Control::default(), &mut result).is_err());
    let expired =
        Control::with_deadline(std::time::Instant::now() - std::time::Duration::from_secs(1));
    assert!(recommend(&v, &request, &ids, &expired, &mut result).is_err());
    let cancelled_after_filter = Control::default();
    assert!(
        analyze(&v, &request, &cancelled_after_filter, |_| {
            cancelled_after_filter.cancel()
        })
        .is_err()
    );
}
