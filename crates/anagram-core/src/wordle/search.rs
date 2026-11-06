use super::*;
use crate::control::Control;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    time::{Duration, Instant},
};

pub fn filter(
    v: &Vocabulary,
    request: &AnalysisRequest,
    control: &Control,
) -> Result<(Analysis, Vec<usize>), String> {
    let start = Instant::now();
    if !(4..=8).contains(&request.length)
        || request.attempts_remaining > 12
        || request.history.len() > 12
        || request.limit > 100
    {
        return Err("Use 4–8 letters, 0–12 remaining attempts, at most 12 history rows and 100 results per page".into());
    }
    if request.budget.initial_batch == 0 || request.budget.max_states == 0 {
        return Err("Search batch and state budgets must be positive".into());
    }
    let n = request.length;
    let mut fixed = vec![None; n];
    let mut forbidden = vec![[false; 26]; n];
    let (mut minimum, mut maximum) = ([0usize; 26], [n; 26]);
    let mut contradiction = false;
    if request.constraints.fixed.len() > n || request.constraints.forbidden_positions.len() > n {
        return Err("Position constraints exceed the word length".into());
    }
    for (i, ch) in request.constraints.fixed.iter().enumerate() {
        if let Some(ch) = ch {
            if !ch.is_ascii_alphabetic() {
                return Err("Fixed positions require English letters".into());
            }
            fixed[i] = Some(ch.to_ascii_lowercase() as u8);
        }
    }
    for (i, letters) in request.constraints.forbidden_positions.iter().enumerate() {
        for b in letters.bytes() {
            if !b.is_ascii_alphabetic() {
                return Err("Forbidden positions require English letters".into());
            }
            forbidden[i][(b.to_ascii_lowercase() - b'a') as usize] = true;
        }
    }
    for bound in &request.constraints.counts {
        if bound.letter.len() != 1
            || !bound.letter.as_bytes()[0].is_ascii_alphabetic()
            || bound.min > n
            || bound.max.is_some_and(|m| m > n)
        {
            return Err(
                "Letter counts require one English letter and bounds within the word length".into(),
            );
        }
        let i = (bound.letter.as_bytes()[0].to_ascii_lowercase() - b'a') as usize;
        minimum[i] = minimum[i].max(bound.min);
        maximum[i] = maximum[i].min(bound.max.unwrap_or(n));
    }
    let mut rows = Vec::new();
    let mut notices = Vec::new();
    for row in &request.history {
        let word = validate_word(&row.word)?;
        if word.len() != n || row.feedback.len() != n {
            return Err(
                "Every history row and feedback must match the selected word length".into(),
            );
        }
        if !v.contains(&word) {
            notices.push(format!(
                "{} is not in the local allowed-guess list; its feedback is still applied.",
                word.to_uppercase()
            ));
        }
        let (mut matched, mut gray) = ([0usize; 26], [false; 26]);
        for (i, (&b, &mark)) in word.as_bytes().iter().zip(&row.feedback).enumerate() {
            let letter = (b - b'a') as usize;
            if mark == Mark::Green {
                if fixed[i].is_some_and(|old| old != b) {
                    contradiction = true;
                }
                fixed[i] = Some(b);
            } else {
                forbidden[i][letter] = true;
            }
            if mark == Mark::Gray {
                gray[letter] = true;
            } else {
                matched[letter] += 1;
            }
        }
        for i in 0..26 {
            minimum[i] = minimum[i].max(matched[i]);
            if gray[i] {
                maximum[i] = maximum[i].min(matched[i]);
            }
        }
        rows.push((word, row_pattern(&row.feedback)));
    }
    let mut fixed_counts = [0usize; 26];
    for (i, ch) in fixed.iter().enumerate() {
        if let Some(ch) = ch {
            let l = (ch - b'a') as usize;
            fixed_counts[l] += 1;
            contradiction |= forbidden[i][l];
        }
    }
    for i in 0..26 {
        minimum[i] = minimum[i].max(fixed_counts[i]);
        contradiction |= minimum[i] > maximum[i];
    }
    contradiction |= minimum.iter().sum::<usize>() > n || maximum.iter().sum::<usize>() < n;
    let mut ids = Vec::new();
    if !contradiction {
        for &id in &v.answers {
            control.check().map_err(str::to_owned)?;
            let word = v.words[id].word.as_bytes();
            if word.len() != n
                || word.iter().enumerate().any(|(i, &b)| {
                    fixed[i].is_some_and(|f| f != b) || forbidden[i][(b - b'a') as usize]
                })
            {
                continue;
            }
            let mut counts = [0usize; 26];
            for &b in word {
                counts[(b - b'a') as usize] += 1;
            }
            if (0..26).any(|i| counts[i] < minimum[i] || counts[i] > maximum[i]) {
                continue;
            }
            if rows
                .iter()
                .all(|(guess, code)| pattern(word, guess.as_bytes()) == *code)
            {
                ids.push(id);
            }
        }
    }
    control
        .sort_by(&mut ids, |&a, &b| tie(v, a, b))
        .map_err(str::to_owned)?;
    let analysis = Analysis {
        engine_id: ENGINE_ID.into(),
        length: request.length,
        attempts_remaining: request.attempts_remaining,
        guess_policy: request.guess_policy,
        objective: "win_probability_then_penalized_cost".into(),
        profile: v.profile.clone(),
        candidate_count: ids.len(),
        candidates: ids
            .iter()
            .skip(request.offset)
            .take(request.limit)
            .map(|&i| v.words[i].word.clone())
            .collect(),
        offset: request.offset,
        strategic: None,
        answer: None,
        method: "filtering_complete".into(),
        optimal_for_profile: false,
        guess_space_complete: false,
        evaluated_guesses: 0,
        expanded_states: 0,
        diagnostic: if contradiction {
            Some("contradictory_clues".into())
        } else if ids.is_empty() {
            Some("no_profile_matches".into())
        } else {
            None
        },
        notices,
        filtering_ms: start.elapsed().as_secs_f64() * 1000.0,
        recommendation_ms: 0.0,
    };
    Ok((analysis, ids))
}

fn tie(v: &Vocabulary, a: usize, b: usize) -> std::cmp::Ordering {
    v.words[b]
        .frequency
        .total_cmp(&v.words[a].frequency)
        .then_with(|| v.words[a].word.cmp(&v.words[b].word))
}

pub fn analyze(
    v: &Vocabulary,
    request: &AnalysisRequest,
    control: &Control,
    mut publish: impl FnMut(&Analysis),
) -> Result<Analysis, String> {
    let (mut result, ids) = filter(v, request, control)?;
    publish(&result);
    recommend(v, request, &ids, control, &mut result)?;
    Ok(result)
}

fn metrics(v: &Vocabulary, ids: &[usize], g: usize) -> Recommendation {
    let mut buckets = [0usize; 6561];
    for &a in ids {
        buckets[pattern(v.words[a].word.as_bytes(), v.words[g].word.as_bytes()) as usize] += 1;
    }
    let n = ids.len() as f64;
    let green = 3usize.pow(v.words[g].word.len() as u32) - 1;
    let information_bits = buckets
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum();
    let expected_remaining = buckets
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != green)
        .map(|(_, &c)| (c * c) as f64 / n)
        .sum();
    let immediate_win_probability = if ids.contains(&g) { 1.0 / n } else { 0.0 };
    Recommendation {
        word: v.words[g].word.clone(),
        information_bits,
        expected_remaining,
        immediate_win_probability,
        solve_probability: None,
        penalized_expected_cost: None,
    }
}

#[derive(Clone, Copy, Debug)]
struct Value {
    wins: usize,
    cost: usize,
}
impl Value {
    fn better(self, other: Self) -> bool {
        self.wins > other.wins || (self.wins == other.wins && self.cost < other.cost)
    }
}
type Action = (usize, Value);
// Equivalent actions on the root remain equivalent on every descendant subset.
// Keep the winning answer identity in the key: a singleton observation is not a win.
fn reduce_guesses(
    v: &Vocabulary,
    guesses: &[usize],
    root: &[usize],
    control: &Control,
    cache_bytes: usize,
) -> Result<(Vec<usize>, Option<Vec<u16>>), String> {
    let keep_matrix = guesses.len().saturating_mul(root.len()) <= cache_bytes / 2;
    let mut seen = HashSet::new();
    let mut reduced = Vec::new();
    let mut matrix = Vec::new();
    for &g in guesses {
        control.check().map_err(str::to_owned)?;
        let mut groups = BTreeMap::<u16, Vec<usize>>::new();
        let mut win = None;
        let mut row = Vec::with_capacity(root.len());
        for &a in root {
            let code = pattern(v.words[a].word.as_bytes(), v.words[g].word.as_bytes());
            row.push(code);
            if a == g {
                win = Some(a);
            } else {
                groups.entry(code).or_default().push(a);
            }
        }
        let mut groups: Vec<_> = groups.into_values().collect();
        groups.sort();
        if win.is_none() && groups.len() == 1 {
            continue;
        }
        if seen.insert((win, groups)) {
            reduced.push(g);
            if keep_matrix {
                matrix.extend(row);
            }
        }
    }
    Ok((reduced, if keep_matrix { Some(matrix) } else { None }))
}
struct Exact<'a> {
    v: &'a Vocabulary,
    guesses: Vec<usize>,
    control: &'a Control,
    root_positions: HashMap<usize, usize>,
    matrix: Option<Vec<u16>>,
    width: usize,
    memo: HashMap<(Vec<usize>, usize), Value>,
    states: usize,
    max_states: usize,
    answers_only: bool,
}
impl Exact<'_> {
    fn partitions(&self, ids: &[usize], gindex: usize) -> (Option<usize>, Vec<Vec<usize>>) {
        let g = self.guesses[gindex];
        let mut map = BTreeMap::<u16, Vec<usize>>::new();
        let mut win = None;
        for &a in ids {
            if a == g {
                win = Some(a);
                continue;
            }
            let p = self.matrix.as_ref().map_or_else(
                || {
                    pattern(
                        self.v.words[a].word.as_bytes(),
                        self.v.words[g].word.as_bytes(),
                    )
                },
                |m| m[gindex * self.width + self.root_positions[&a]],
            );
            map.entry(p).or_default().push(a);
        }
        let mut parts: Vec<_> = map.into_values().collect();
        parts.sort();
        (win, parts)
    }
    fn actions(&mut self, ids: &[usize], turns: usize) -> Result<Vec<Action>, String> {
        self.control.check().map_err(str::to_owned)?;
        self.states += 1;
        if self.states > self.max_states {
            return Err("state_budget".into());
        }
        let mut seen = HashSet::new();
        let mut actions = Vec::new();
        for gi in 0..self.guesses.len() {
            self.control.check().map_err(str::to_owned)?;
            let g = self.guesses[gi];
            if self.answers_only && !ids.contains(&g) {
                continue;
            }
            if turns == 1 {
                if ids.contains(&g) {
                    actions.push((
                        g,
                        Value {
                            wins: 1,
                            cost: 2 * ids.len() - 1,
                        },
                    ));
                }
                continue;
            }
            let (win, parts) = self.partitions(ids, gi);
            if win.is_none() && parts.len() == 1 {
                continue;
            } // A non-winning, non-informative guess is dominated.
            if !seen.insert((win, parts.clone())) {
                continue;
            }
            let mut value = Value {
                wins: usize::from(win.is_some()),
                cost: ids.len(),
            };
            for part in parts {
                let sub = self.value(&part, turns - 1)?;
                value.wins += sub.wins;
                value.cost += sub.cost;
            }
            actions.push((g, value));
            // At most one answer wins now; every other answer needs another guess.
            // Sorted tie order makes this bound sufficient for both recommendation slots.
            if value.wins == ids.len() && value.cost == 2 * ids.len() - 1 {
                break;
            }
        }
        Ok(actions)
    }
    fn value(&mut self, ids: &[usize], turns: usize) -> Result<Value, String> {
        if turns == 0 {
            return Ok(Value {
                wins: 0,
                cost: ids.len(),
            });
        }
        if ids.len() == 1 {
            return Ok(Value { wins: 1, cost: 1 });
        }
        if turns == 1 {
            return Ok(Value {
                wins: 1,
                cost: 2 * ids.len() - 1,
            });
        }
        let key = (ids.to_vec(), turns);
        if let Some(&value) = self.memo.get(&key) {
            return Ok(value);
        }
        let actions = self.actions(ids, turns)?;
        let mut best = Value {
            wins: 0,
            cost: usize::MAX,
        };
        for (_, value) in actions {
            if value.better(best) {
                best = value;
            }
        }
        self.memo.insert(key, best); // Only fully solved states reach this point.
        Ok(best)
    }
}

pub fn recommend(
    v: &Vocabulary,
    r: &AnalysisRequest,
    ids: &[usize],
    caller: &Control,
    result: &mut Analysis,
) -> Result<(), String> {
    let start = Instant::now();
    if ids.is_empty() || r.attempts_remaining == 0 {
        result.method = "no_next_guess".into();
        return Ok(());
    }
    let control = if r.budget.timeout_ms == 0 {
        caller.clone()
    } else {
        caller.with_earlier_deadline(Instant::now() + Duration::from_millis(r.budget.timeout_ms))
    };
    let mut guesses: Vec<_> = if r.guess_policy == GuessPolicy::AnswersOnly {
        ids.to_vec()
    } else {
        v.words
            .iter()
            .enumerate()
            .filter(|(_, w)| w.word.len() == r.length)
            .map(|(i, _)| i)
            .collect()
    };
    guesses.sort_by(|&a, &b| tie(v, a, b));
    if ids.len() == 1 || r.attempts_remaining == 1 {
        let g = ids[0];
        let mut rec = metrics(v, ids, g);
        rec.solve_probability = Some(1.0 / ids.len() as f64);
        rec.penalized_expected_cost = Some((2 * ids.len() - 1) as f64 / ids.len() as f64);
        result.strategic = Some(rec.clone());
        result.answer = Some(rec);
        result.method = "optimal_for_profile".into();
        result.optimal_for_profile = true;
        result.guess_space_complete = true;
        result.evaluated_guesses = ids.len();
        result.recommendation_ms = start.elapsed().as_secs_f64() * 1000.0;
        return Ok(());
    }
    // Coverage ordering includes positional and distinct-letter coverage, including probe words.
    let mut positional = vec![[0usize; 26]; r.length];
    let mut occurrence = [0usize; 26];
    for &a in ids {
        let mut seen = [false; 26];
        for (p, &b) in v.words[a].word.as_bytes().iter().enumerate() {
            let l = (b - b'a') as usize;
            positional[p][l] += 1;
            if !seen[l] {
                occurrence[l] += 1;
                seen[l] = true;
            }
        }
    }
    let coverage = |g: usize| {
        let mut seen = [false; 26];
        let mut score = 0;
        for (p, &b) in v.words[g].word.as_bytes().iter().enumerate() {
            let l = (b - b'a') as usize;
            score += positional[p][l];
            if !seen[l] {
                score += occurrence[l];
                seen[l] = true;
            }
        }
        score
    };
    guesses.sort_by(|&a, &b| coverage(b).cmp(&coverage(a)).then_with(|| tie(v, a, b)));
    // Ensure an answer recommendation exists even if the ranking deadline is very small.
    if let Some(pos) = guesses.iter().position(|&g| g == ids[0]) {
        guesses.swap(0, pos);
    }
    let answer_set: HashSet<_> = ids.iter().copied().collect();
    let mut evaluated = 0;
    let mut best: Option<(usize, Recommendation)> = None;
    let mut answer: Option<(usize, Recommendation)> = None;
    // Reserve half the interactive budget for exact search when eligible.
    let exact_eligible =
        r.strategy == Strategy::Adaptive && ids.len() <= r.budget.endgame_candidates;
    let heuristic_end = if exact_eligible && r.budget.timeout_ms > 0 {
        Some(start + Duration::from_millis(r.budget.timeout_ms / 2))
    } else {
        None
    };
    for &g in &guesses {
        if evaluated > 0
            && (control.check().is_err() || heuristic_end.is_some_and(|t| Instant::now() >= t))
        {
            break;
        }
        caller.check().map_err(str::to_owned)?;
        let mut rec = metrics(v, ids, g);
        if r.attempts_remaining == 2 && r.strategy == Strategy::Adaptive {
            let mut buckets = HashSet::new();
            for &a in ids {
                if a != g {
                    buckets.insert(pattern(
                        v.words[a].word.as_bytes(),
                        v.words[g].word.as_bytes(),
                    ));
                }
            }
            let wins = buckets.len() + usize::from(answer_set.contains(&g));
            rec.solve_probability = Some(wins as f64 / ids.len() as f64);
            rec.penalized_expected_cost = Some(
                (3 * ids.len() - wins - usize::from(answer_set.contains(&g))) as f64
                    / ids.len() as f64,
            );
        }
        let better = |old: &Option<(usize, Recommendation)>| {
            old.as_ref().is_none_or(|(old_id, old)| {
                let order = if r.attempts_remaining == 2 && r.strategy == Strategy::Adaptive {
                    rec.solve_probability
                        .unwrap()
                        .total_cmp(&old.solve_probability.unwrap())
                        .then_with(|| {
                            old.penalized_expected_cost
                                .unwrap()
                                .total_cmp(&rec.penalized_expected_cost.unwrap())
                        })
                } else {
                    rec.information_bits
                        .total_cmp(&old.information_bits)
                        .then_with(|| old.expected_remaining.total_cmp(&rec.expected_remaining))
                };
                order.is_gt() || (order.is_eq() && tie(v, g, *old_id).is_lt())
            })
        };
        if answer_set.contains(&g) && better(&answer) {
            answer = Some((g, rec.clone()));
        }
        if better(&best) {
            best = Some((g, rec));
        }
        evaluated += 1;
        // A batch is a widening increment, never a permanent shortlist limit.
        if exact_eligible && evaluated >= r.budget.initial_batch {
            break;
        }
        if !r.budget.full_search
            && evaluated % r.budget.initial_batch == 0
            && start.elapsed().as_millis() >= u128::from(r.budget.target_ms)
        {
            break;
        }
    }
    result.strategic = best.map(|(_, rec)| rec);
    result.answer = answer.map(|(_, rec)| rec);
    result.evaluated_guesses = evaluated;
    result.guess_space_complete = evaluated == guesses.len();
    result.method = if r.attempts_remaining == 2 && r.strategy == Strategy::Adaptive {
        "two_turn_search"
    } else {
        "entropy_heuristic"
    }
    .into();
    result.optimal_for_profile = r.attempts_remaining == 2
        && r.strategy == Strategy::Adaptive
        && result.guess_space_complete;
    if exact_eligible
        && r.budget.exact_endgame
        && r.attempts_remaining > 2
        && control.check().is_ok()
    {
        // Interactive endgames get twice the soft latency target. The hard ceiling
        // still applies; offline full searches can spend the entire supplied budget.
        let exact_control = if r.budget.full_search || r.budget.timeout_ms == 0 {
            control.clone()
        } else {
            control.with_earlier_deadline(
                start + Duration::from_millis(r.budget.target_ms.saturating_mul(2).max(1)),
            )
        };
        guesses.sort_by(|&a, &b| tie(v, a, b));
        let mut root = ids.to_vec();
        root.sort_unstable();
        let positions = root.iter().enumerate().map(|(i, &a)| (a, i)).collect();
        let full_guess_count = guesses.len();
        let Ok((guesses, matrix)) =
            reduce_guesses(v, &guesses, &root, &exact_control, r.budget.cache_bytes)
        else {
            caller.check().map_err(str::to_owned)?;
            result.method = "budget_limited_heuristic".into();
            result.recommendation_ms = start.elapsed().as_secs_f64() * 1000.0;
            return Ok(());
        };
        let mut exact = Exact {
            v,
            guesses,
            control: &exact_control,
            root_positions: positions,
            matrix,
            width: root.len(),
            memo: HashMap::new(),
            states: 0,
            max_states: r.budget.max_states,
            answers_only: r.guess_policy == GuessPolicy::AnswersOnly,
        };
        if let Ok(actions) = exact.actions(&root, r.attempts_remaining) {
            let mut strategic: Option<Action> = None;
            let mut candidate: Option<Action> = None;
            for action in actions {
                if strategic.is_none_or(|old| action.1.better(old.1)) {
                    strategic = Some(action);
                }
                if answer_set.contains(&action.0)
                    && candidate.is_none_or(|old| action.1.better(old.1))
                {
                    candidate = Some(action);
                }
            }
            let make = |(g, value): Action| {
                let mut rec = metrics(v, ids, g);
                rec.solve_probability = Some(value.wins as f64 / ids.len() as f64);
                rec.penalized_expected_cost = Some(value.cost as f64 / ids.len() as f64);
                rec
            };
            result.strategic = strategic.map(make);
            result.answer = candidate.map(make);
            result.optimal_for_profile = true;
            result.guess_space_complete = true;
            result.method = "optimal_for_profile".into();
            result.evaluated_guesses = full_guess_count;
        } else {
            result.method = "budget_limited_heuristic".into();
        }
        result.expanded_states = exact.states;
    }
    caller.check().map_err(str::to_owned)?;
    result.recommendation_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(())
}
