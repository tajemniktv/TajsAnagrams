//! Wordle rules and task-specific vocabulary. Anagram admission is intentionally separate.
mod search;
mod vocabulary;
use schemars::JsonSchema;
pub use search::{analyze, filter, recommend};
use serde::{Deserialize, Serialize};
pub const ENGINE_ID: &str = env!("ANAGRAM_CORE_BUILD_ID");
pub use vocabulary::{Vocabulary, parse_words, validate_word};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    Gray,
    Yellow,
    Green,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoryRow {
    pub word: String,
    pub feedback: Vec<Mark>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartialHistoryRow {
    pub word: String,
    /// None means unknown, never gray. Only known feedback constrains candidates.
    pub feedback: Vec<Option<Mark>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    #[default]
    Broad,
    Common,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct VocabularyProfile {
    pub schema_version: u32,
    pub kind: ProfileKind,
    pub name: String,
    pub fingerprint: String,
    pub sources: Vec<crate::contracts::DataIdentity>,
    pub probability_model: String,
    pub allowed_guesses: usize,
    pub eligible_answers: usize,
    pub frequency_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LetterBound {
    pub letter: String,
    pub min: usize,
    pub max: Option<usize>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Constraints {
    pub fixed: Vec<Option<char>>,
    pub forbidden_positions: Vec<String>,
    pub counts: Vec<LetterBound>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GuessPolicy {
    #[default]
    Unrestricted,
    AnswersOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    #[default]
    Adaptive,
    Entropy,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Play,
    Instant,
    Practice,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub length: usize,
    pub attempts: usize,
    pub mode: Mode,
    pub play_profile: ProfileKind,
    pub instant_profile: ProfileKind,
    pub practice_profile: ProfileKind,
    pub answers_path: String,
    pub guesses_path: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            length: 5,
            attempts: 6,
            mode: Mode::Play,
            play_profile: ProfileKind::Broad,
            instant_profile: ProfileKind::Broad,
            practice_profile: ProfileKind::Common,
            answers_path: String::new(),
            guesses_path: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Analyze,
    Filter,
    NewPractice,
    Guess,
    Reveal,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionCommand {
    /// Explicitly recheck all supplied clues against a newly selected vocabulary.
    #[serde(default)]
    pub restart: bool,
    /// Durable non-Practice sessions must verify the exact previously used corpus.
    #[serde(default)]
    pub expected_fingerprint: Option<String>,
    pub mode: Mode,
    pub action: SessionAction,
    pub revision: u64,
    pub profile: ProfileKind,
    pub attempts: usize,
    pub answers_path: String,
    pub guesses_path: String,
    pub word: String,
    pub request: AnalysisRequest,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
pub struct SessionSnapshot {
    pub id: u64,
    pub revision: u64,
    pub mode: Mode,
    pub active: bool,
    pub analysis: Option<Analysis>,
    pub history: Vec<HistoryRow>,
    pub revealed_answer: Option<String>,
    pub finished: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct SearchBudget {
    /// Disable only multi-step endgame search, retaining its heuristic fallback.
    /// Used for matched ablations; the interactive default remains enabled.
    pub exact_endgame: bool,
    /// Local recommendation deadline, checked between scoring passes. Preparation
    /// and a complete guess/answer pass may overrun it; caller deadlines still apply.
    pub timeout_ms: u64,
    /// Initial seed/widening increment, never a permanent guess shortlist.
    pub initial_batch: usize,
    pub endgame_candidates: usize,
    pub max_states: usize,
    /// Bypass soft phase targets, retaining caller/hard deadlines and state limits.
    pub full_search: bool,
    /// Optional feedback-matrix allocation only; not a total heap/memoization cap.
    pub cache_bytes: usize,
    /// Ordinary ranking target. Deeper adaptive endgames share twice this budget:
    /// exact work yields after the first target so fallback ranking can resume.
    pub target_ms: u64,
}
impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            exact_endgame: true,
            timeout_ms: 2000,
            initial_batch: 256,
            endgame_candidates: 12,
            max_states: 50_000,
            full_search: false,
            cache_bytes: 8 * 1024 * 1024,
            target_ms: 100,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct AnalysisRequest {
    pub length: usize,
    pub attempts_remaining: usize,
    pub history: Vec<HistoryRow>,
    pub partial_history: Vec<PartialHistoryRow>,
    pub constraints: Constraints,
    pub budget: SearchBudget,
    pub guess_policy: GuessPolicy,
    pub strategy: Strategy,
    pub offset: usize,
    pub limit: usize,
}
impl Default for AnalysisRequest {
    fn default() -> Self {
        Self {
            length: 5,
            attempts_remaining: 6,
            history: vec![],
            partial_history: vec![],
            constraints: Constraints::default(),
            budget: SearchBudget::default(),
            guess_policy: GuessPolicy::Unrestricted,
            strategy: Strategy::Adaptive,
            offset: 0,
            limit: 100,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Recommendation {
    pub word: String,
    pub information_bits: f64,
    /// A solved branch has zero candidates left to guess.
    pub expected_remaining: f64,
    pub immediate_win_probability: f64,
    pub solve_probability: Option<f64>,
    pub penalized_expected_cost: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Analysis {
    pub engine_id: String,
    pub length: usize,
    pub attempts_remaining: usize,
    pub guess_policy: GuessPolicy,
    pub objective: String,
    pub profile: VocabularyProfile,
    pub candidate_count: usize,
    pub candidates: Vec<String>,
    pub offset: usize,
    pub strategic: Option<Recommendation>,
    pub answer: Option<Recommendation>,
    pub method: String,
    pub optimal_for_profile: bool,
    pub guess_space_complete: bool,
    pub evaluated_guesses: usize,
    pub expanded_states: usize,
    pub diagnostic: Option<String>,
    pub notices: Vec<String>,
    pub filtering_ms: f64,
    pub recommendation_ms: f64,
}

/// Exact duplicate accounting: greens consume answer occurrences before yellows.
pub fn feedback(answer: &str, guess: &str) -> Result<Vec<Mark>, String> {
    let a = validate_word(answer)?;
    let g = validate_word(guess)?;
    if a.len() != g.len() {
        return Err("Answer and guess lengths differ".into());
    }
    let code = pattern(a.as_bytes(), g.as_bytes());
    let mut value = code;
    Ok((0..a.len())
        .map(|_| {
            let mark = match value % 3 {
                2 => Mark::Green,
                1 => Mark::Yellow,
                _ => Mark::Gray,
            };
            value /= 3;
            mark
        })
        .collect())
}

// 3^8 fits in u16; intake validates the supported 4–8 letter domain.
pub(crate) fn pattern(answer: &[u8], guess: &[u8]) -> u16 {
    let mut remaining = [0u8; 26];
    for (&a, &g) in answer.iter().zip(guess) {
        if a != g {
            remaining[(a - b'a') as usize] += 1;
        }
    }
    let (mut code, mut place) = (0, 1);
    for (&a, &g) in answer.iter().zip(guess) {
        let mark = if a == g {
            2
        } else if remaining[(g - b'a') as usize] > 0 {
            remaining[(g - b'a') as usize] -= 1;
            1
        } else {
            0
        };
        code += mark * place;
        place *= 3;
    }
    code
}
pub(crate) fn row_pattern(marks: &[Mark]) -> u16 {
    marks.iter().rev().fold(0, |n, m| {
        n * 3
            + match m {
                Mark::Gray => 0,
                Mark::Yellow => 1,
                Mark::Green => 2,
            }
    })
}
