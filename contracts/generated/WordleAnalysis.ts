// Generated from Rust by cargo xtask contracts. Do not edit.

export type DataIdentity = {
  "bytes": number;
  "present": boolean;
  "representation": DataRepresentation;
  "role": string;
  "sha256": string;
};

export type DataRepresentation = "file_bytes" | "phrase_rows_v1";

export type GuessPolicy = "unrestricted" | "answers_only";

export type ProfileKind = "broad" | "common" | "custom";

export type Recommendation = {
  "expected_remaining": number;
  "immediate_win_probability": number;
  "information_bits": number;
  "penalized_expected_cost"?: number | null;
  "solve_probability"?: number | null;
  "word": string;
  [key: string]: unknown;
};

export type VocabularyProfile = {
  "allowed_guesses": number;
  "eligible_answers": number;
  "fingerprint": string;
  "frequency_available": boolean;
  "kind": ProfileKind;
  "name": string;
  "probability_model": string;
  "schema_version": number;
  "sources": Array<DataIdentity>;
  [key: string]: unknown;
};

export type WordleAnalysis = {
  "answer"?: (Recommendation) | (null);
  "attempts_remaining": number;
  "candidate_count": number;
  "candidates": Array<string>;
  "diagnostic"?: string | null;
  "engine_id": string;
  "evaluated_guesses": number;
  "expanded_states": number;
  "filtering_ms": number;
  "guess_policy": GuessPolicy;
  "guess_space_complete": boolean;
  "length": number;
  "method": string;
  "notices": Array<string>;
  "objective": string;
  "offset": number;
  "optimal_for_profile": boolean;
  "profile": VocabularyProfile;
  "recommendation_ms": number;
  "strategic"?: (Recommendation) | (null);
  [key: string]: unknown;
};
