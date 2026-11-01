import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Candidate } from './model';
export const isDesktop = isTauri();
export type ScopeKind = 'projects' | 'thumbnails' | 'protected';
export interface NativeScope {
  id: string;
  label: string;
  display_path: string;
  kind: ScopeKind;
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
  progress: { visited: number; candidates: number; stage: string };
  result: NativeResult | null;
  error: string | null;
}
export const native = {
  chooseScope: (kind: ScopeKind) =>
    invoke<NativeScope | null>('choose_scope', { kind }),
  listScopes: () => invoke<NativeScope[]>('list_scopes'),
  forgetScope: (scopeId: string) => invoke<void>('forget_scope', { scopeId }),
  beginScan: (scopeIds: string[]) => invoke<string>('begin_scan', { scopeIds }),
  pollScan: (jobId: string) => invoke<ScanPoll>('poll_scan', { jobId }),
  controlScan: (jobId: string, action: 'pause' | 'resume' | 'cancel') =>
    invoke<void>('control_scan', { jobId, action }),
};
