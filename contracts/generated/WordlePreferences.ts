// Generated from Rust by cargo xtask contracts. Do not edit.

export type Mode = "play" | "instant" | "practice";

export type ProfileKind = "broad" | "common" | "custom";

export type WordlePreferences = {
  "answers_path"?: string;
  "attempts"?: number;
  "guesses_path"?: string;
  "instant_profile"?: ProfileKind;
  "length"?: number;
  "mode"?: Mode;
  "play_profile"?: ProfileKind;
  "practice_profile"?: ProfileKind;
};
