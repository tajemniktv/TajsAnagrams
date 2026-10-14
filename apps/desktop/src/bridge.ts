import { invoke } from '@tauri-apps/api/core';
import type { SolveRequest } from '../../../contracts/generated/SolveRequest';
import type { SolveResult as RankedResult } from '../../../contracts/generated/SolveResult';
import type { Generated } from '../../../contracts/generated/Generated';
export type GeneratedResult = Generated & {status: JobStatus};
export type SolveResult = RankedResult | GeneratedResult;
export type { RankedResult };
import type { JobStatus } from '../../../contracts/generated/JobStatus';
import type { DeploymentLimits } from '../../../contracts/generated/DeploymentLimits';
export type { SolveRequest, JobStatus };
export type CorpusKey = 'dictionary' | 'unigrams' | 'bigrams' | 'wordnet' | 'phrase';
export interface Settings { schema_version: number; developer_mode: boolean; theme: 'dark' | 'light' | 'system'; corpora: Record<CorpusKey, string>; model: string; request: SolveRequest; runtime: {cache_path: string; cache_entries: number; cache_bytes: number; custom_limits: boolean; limits: DeploymentLimits} }
export interface Bootstrap { settings: Settings; notice: string | null; limits: DeploymentLimits; corpus_error: string | null; data_directory: string; engine_version: string; active_jobs: number; queue_capacity: number }
export interface Job { id: number; active: boolean; status: JobStatus | null; error: string | null }
export interface TrainingStatus { active: boolean; error: string | null; report: { model_path: string; baseline: { groups: number; recall1: number; mrr: number }; held_out: { groups: number; recall1: number; mrr: number } } | null }
export type Verdict = 'useful' | 'not_useful' | 'unsure';
export interface ReviewRow { id: string; phrase: string; rank: number; word_count: number; score: number; judgeable: boolean }
export interface ReviewCapture { id: string; case_id: string; source_family: string | null; input: string; rows: ReviewRow[]; request: SolveRequest; result: RankedResult | null; error: string | null; status?: JobStatus; trial?: number; cache_state?: string; dictionary_variant?: string; context_id: string; source_context?: {original_context?: string; constraints?: unknown}; task_mapping?: {task?: string; unsupported_structural_constraints?: unknown}; origin: string }
export interface ReviewJudgment { id: string; verdict: Verdict | null; reasons: string[]; note: string; task_fit: Verdict | null; looked_up: boolean }
export interface ReviewPreference { id?: string; capture_id: string; left: string; right: string; choice: 'left' | 'right' | 'tie' | 'both_bad' | 'unsure' }
export interface ReviewState { schema_version: number; captures: ReviewCapture[]; judgments: ReviewJudgment[]; preferences: ReviewPreference[]; storage?: string }
// All desktop transport lives here. Browser development never fabricates a solver.
export const bridge = {
  reviewState: () => invoke<ReviewState>('review_state'),
  reviewCapture: (id: number) => invoke<number>('review_capture', { id }),
  reviewImport: (path: string, workloads: string | null) => invoke<number>('review_import', { path, workloads }),
  reviewJudge: (judgment: ReviewJudgment) => invoke<void>('review_judge', { judgment }),
  reviewPrefer: (preference: ReviewPreference) => invoke<void>('review_prefer', { preference }),
  reviewExport: () => invoke<string | null>('review_export'),
  manage: (action: unknown) => invoke<void>('manage', { action }),
  diagnostics: () => invoke<Record<string, unknown>>('diagnostics'),
  readReport: (path: string) => invoke<unknown>('read_report', { path }),
  maintenanceStatus: () => invoke<{active: boolean; error: string | null; report: Record<string, unknown> | null}>('training_status'),
  bootstrap: () => invoke<Bootstrap>('bootstrap'),
  save: (settings: Settings) => invoke<void>('save_settings', { settings }),
  start: (request: SolveRequest, rebuild = false, extended = false, generationOnly = false) => invoke<number>('start_job', { request, rebuild, extended, generationOnly }),
  clearCache: () => invoke<void>('clear_cache'),
  status: () => invoke<Job>('job_status'),
  result: (id: number) => invoke<SolveResult | null>('job_result', { id }),
  cancel: (id: number) => invoke<void>('cancel_job', { id }),
  choose: (kind: CorpusKey | 'model' | 'training' | 'source' | 'titles' | 'cache' | 'review' | 'review-workloads') => invoke<string | null>('choose_path', { kind }),
  train: (dataset: string, epochs: number, folds: number, learningRate = 0.08, l2 = 0.002) => invoke<void>('train_model', { dataset, epochs, folds, learningRate, l2 }),
  trainingStatus: () => invoke<TrainingStatus>('training_status'),
  cancelTraining: () => invoke<void>('cancel_training'),
  copy: (id: number) => invoke<void>('copy_results', { id }),
  export: (id: number, format: 'txt' | 'json') => invoke<string | null>('export_results', { id, format }),
};
export const message = (error: unknown) => error instanceof Error ? error.message : String(error);
