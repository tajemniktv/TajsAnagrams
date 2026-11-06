// Generated from Rust by cargo xtask contracts. Do not edit.

export type Constraints = {
  "counts"?: Array<LetterBound>;
  "fixed"?: Array<string | null>;
  "forbidden_positions"?: Array<string>;
};

export type GuessPolicy = "unrestricted" | "answers_only";

export type HistoryRow = {
  "feedback": Array<Mark>;
  "word": string;
};

export type LetterBound = {
  "letter": string;
  "max"?: number | null;
  "min": number;
};

export type Mark = "gray" | "yellow" | "green";

export type PartialHistoryRow = {
  "feedback": Array<(Mark) | (null)>;
  "word": string;
};

export type SearchBudget = {
  "cache_bytes"?: number;
  "endgame_candidates"?: number;
  "exact_endgame"?: boolean;
  "full_search"?: boolean;
  "initial_batch"?: number;
  "max_states"?: number;
  "target_ms"?: number;
  "timeout_ms"?: number;
};

export type Strategy = "adaptive" | "entropy";

export type WordleRequest = {
  "attempts_remaining"?: number;
  "budget"?: SearchBudget;
  "constraints"?: Constraints;
  "guess_policy"?: GuessPolicy;
  "history"?: Array<HistoryRow>;
  "length"?: number;
  "limit"?: number;
  "offset"?: number;
  "partial_history"?: Array<PartialHistoryRow>;
  "strategy"?: Strategy;
};
