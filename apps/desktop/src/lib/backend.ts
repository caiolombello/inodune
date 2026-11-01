import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Candidate } from './model';
export const isDesktop = isTauri();
export type ScopeKind = 'projects' | 'thumbnails' | 'protected';
export interface NativeScope {
  id: string;
  label: string;
  display_path: string;
  kind: ScopeKind;
  exclusions?: string[];
}
export interface NativeIssue {
  display_path: string;
  code: string;
  message: string;
}
export interface NativeVolume {
  scope_path: string;
  device: number;
  total_bytes: number;
  available_bytes: number;
}
export interface NativeResult {
  scan_id: string;
  candidates: Candidate[];
  issues: NativeIssue[];
  stats: { visited: number; skipped: number; elapsed_ms: number };
  complete: boolean;
  cancelled: boolean;
  volumes: NativeVolume[];
}
export interface ScanPoll {
  job_id: string;
  status: 'running' | 'done' | 'error';
  progress: ScanActivity;
  result: NativeResult | null;
  error: string | null;
}
export type ScanFinding = Pick<
  Candidate,
  'id' | 'title' | 'category' | 'risk' | 'logical_bytes'
>;
export interface ScanActivity {
  visited: number;
  candidates: number;
  stage: string;
  recent_findings?: ScanFinding[];
}
export interface CleanupPlanItem {
  candidate_id: string;
  display_path: string;
  logical_bytes: number;
  allocated_bytes_estimate: number | null;
  risk?: Candidate['risk'];
  impact?: string;
  rebuild_hint?: string;
}
export interface CleanupPlan {
  id: string;
  hash: string;
  snapshot_id: string;
  action: 'trash' | 'permanent';
  expires_in_seconds: number;
  items: CleanupPlanItem[];
  logical_bytes_estimate: number;
  allocated_bytes_estimate: number | null;
  review_required?: boolean;
  inventory_complete?: boolean;
}
export type CleanupFileStatus =
  | 'trashed'
  | 'removed'
  | 'skipped_changed'
  | 'failed'
  | 'cancelled'
  | 'quarantined';
export interface CleanupFileResult {
  display_path: string;
  status: CleanupFileStatus;
  logical_bytes: number;
  message: string | null;
  preserved_at: string | null;
  durability_confirmed?: boolean;
}
export interface CleanupExecutionReport {
  schema_version: number;
  plan_id: string;
  action: 'trash' | 'permanent';
  items: {
    candidate_id: string;
    files: CleanupFileResult[];
    complete: boolean;
  }[];
  volumes: {
    device: number;
    available_before: number;
    available_after: number | null;
    observed_delta: number | null;
  }[];
  complete: boolean;
  cancelled: boolean;
  logical_bytes_processed: number;
  attributed_freed_bytes: number | null;
  measurement_note: string;
  warnings: string[];
  history_path: string | null;
}
export interface CleanupPoll {
  status: 'running' | 'done' | 'error';
  progress: {
    completed_files: number;
    total_files: number;
    completed_items: number;
    total_items: number;
  };
  report: CleanupExecutionReport | null;
  error: string | null;
}
export const native = {
  chooseScope: (kind: ScopeKind, locale: 'pt-BR' | 'en-US' = 'pt-BR') =>
    invoke<NativeScope | null>('choose_scope', { kind, locale }),
  listScopes: () => invoke<NativeScope[]>('list_scopes'),
  forgetScope: (scopeId: string) => invoke<void>('forget_scope', { scopeId }),
  beginAutomaticScan: () => invoke<string>('begin_automatic_scan'),
  beginScan: (scopeIds: string[]) => invoke<string>('begin_scan', { scopeIds }),
  pollScan: (jobId: string) => invoke<ScanPoll>('poll_scan', { jobId }),
  controlScan: (jobId: string, action: 'pause' | 'resume' | 'cancel') =>
    invoke<void>('control_scan', { jobId, action }),
  prepareCleanup: (
    scanId: string,
    candidateIds: string[],
    action: 'trash' | 'permanent',
    locale?: 'pt-BR' | 'en-US',
  ) =>
    invoke<CleanupPlan>('prepare_cleanup', {
      scanId,
      candidateIds,
      action,
      locale,
    }),
  beginCleanup: (planId: string, hash: string) =>
    invoke<string>('begin_cleanup', { planId, hash }),
  pollCleanup: (jobId: string) =>
    invoke<CleanupPoll>('poll_cleanup', { jobId }),
  cancelCleanup: (jobId: string) => invoke<void>('cancel_cleanup', { jobId }),
};
