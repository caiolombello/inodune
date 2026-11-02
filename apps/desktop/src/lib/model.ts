import corpus from '../../../../tests/fixtures/demo.json';
import { language } from './i18n.svelte';
export type Risk = 'low' | 'review' | 'protected';
export type Policy = 'trash' | 'permanent';
export type Scenario = 'normal' | 'partial' | 'empty' | 'error';
export type ScanState =
  'idle' | 'scanning' | 'paused' | 'complete' | 'cancelled' | 'error';
export type Outcome =
  | 'pending'
  | 'processing'
  | 'simulated'
  | 'failed'
  | 'skipped_changed'
  | 'cancelled';
export interface Candidate {
  id: string;
  title: string;
  display_path: string;
  detector_id: string;
  detector_version: string;
  scope_id: string;
  category: string;
  project: string;
  risk: Risk;
  logical_bytes: number;
  allocated_bytes_estimate: number | null;
  identity: { dev: number; ino: number; mtime_unix: number } | null;
  coverage_key: string | null;
  selected_by_default: boolean;
  reason: string;
  impact: string;
  rebuild_hint: string;
  preserved: string;
  evidence: string[];
  activity: string;
  supported_action: 'none' | 'cache_contents' | 'reviewed_cache_contents';
}
export interface Inventory {
  schema_version: number;
  mode: 'simulation';
  scopes: { id: string; label: string; display_path: string }[];
  volume: { label: string; total_bytes: number; used_bytes: number };
  candidates: Candidate[];
}
// One checked fixture for both Rust and TS. Not an external-input parser or real detector.
if (
  corpus.mode !== 'simulation' ||
  corpus.candidates.some(
    (c) =>
      !['low', 'review', 'protected'].includes(c.risk) ||
      c.supported_action !== 'none' ||
      !c.display_path.startsWith('/demo/'),
  )
)
  throw new Error('Invalid demo corpus');
export const demo = corpus as Inventory;
export const riskLabel: Record<Risk, string> = {
  low: 'Baixo risco',
  review: 'Revisar',
  protected: 'Protegido',
};
export const outcomeLabel: Record<Outcome, string> = {
  pending: 'Aguardando',
  processing: 'Processando exemplo',
  simulated: 'Simulado · sem alteração',
  failed: 'Falhou · permissão simulada',
  skipped_changed: 'Pulado · item alterado',
  cancelled: 'Cancelado · não processado',
};
export const CLEANUP_BATCH_LIMIT = 1000;

export function cleanupBatches(
  ids: readonly string[],
  limit = CLEANUP_BATCH_LIMIT,
): string[][] {
  if (!Number.isSafeInteger(limit) || limit < 1)
    throw new Error('Limite de lote inválido');
  const unique: string[] = [];
  const seen = new Set<string>();
  for (const id of ids) {
    if (seen.has(id)) continue;
    seen.add(id);
    unique.push(id);
  }
  const batches: string[][] = [];
  for (let index = 0; index < unique.length; index += limit)
    batches.push(unique.slice(index, index + limit));
  return batches;
}

export function cleanupBatchWarning(
  total: number,
  limit = CLEANUP_BATCH_LIMIT,
): string | null {
  if (total <= limit) return null;
  const batches = Math.ceil(total / limit);
  const n = (value: number) =>
    new Intl.NumberFormat(language.value).format(value);
  if (language.value === 'en-US')
    return `The selection has ${n(total)} items. Each plan accepts at most ${n(limit)}. Preparation will be split into ${n(batches)} batches; this batch covers the first ${n(limit)}.`;
  return `A seleção tem ${n(total)} itens. Cada plano aceita no máximo ${n(limit)}. A preparação será dividida em ${n(batches)} lotes; este lote cobre os primeiros ${n(limit)}.`;
}

export function protectedOnlyLabel(
  count: number,
  occupiedLabel: string,
): string {
  const noun =
    language.value === 'en-US'
      ? count === 1
        ? 'protected item'
        : 'protected items'
      : count === 1
        ? 'item protegido'
        : 'itens protegidos';
  const occupied =
    language.value === 'en-US'
      ? 'occupied (not recoverable)'
      : 'ocupados (não recuperável)';
  return `${new Intl.NumberFormat(language.value).format(count)} ${noun} · ${occupiedLabel} ${occupied}`;
}

export function compareInstalledSize(
  a: number | null,
  b: number | null,
): number {
  if (a === b) return 0;
  if (a === null) return 1;
  if (b === null) return -1;
  return a > b ? -1 : 1;
}
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const unit =
    bytes >= 1024 ** 3 ? 3 : bytes >= 1024 ** 2 ? 2 : bytes >= 1024 ? 1 : 0;
  return `${new Intl.NumberFormat(language.value, { maximumFractionDigits: 1 }).format(bytes / 1024 ** unit)} ${['B', 'KiB', 'MiB', 'GiB'][unit]}`;
}
export function formatSignedBytes(bytes: number): string {
  const sign = bytes > 0 ? '+' : bytes < 0 ? '−' : '';
  return sign + formatBytes(Math.abs(bytes));
}
export function selectDefaults(items: Candidate[]): string[] {
  return items
    .filter((c) => c.risk === 'low' && c.selected_by_default)
    .map((c) => c.id);
}
export function isLocallyEligible(item: Candidate): boolean {
  return (
    (item.risk === 'low' && item.supported_action === 'cache_contents') ||
    (item.risk === 'review' &&
      item.supported_action === 'reviewed_cache_contents')
  );
}
export function selectLocalDefaults(items: Candidate[]): string[] {
  return items
    .filter(
      (item) =>
        item.selected_by_default &&
        item.risk === 'low' &&
        isLocallyEligible(item),
    )
    .map((item) => item.id);
}
export function toggleSelection(ids: string[], item: Candidate): string[] {
  if (item.risk === 'protected') return ids;
  if (ids.includes(item.id)) return ids.filter((id) => id !== item.id);
  return [...ids, item.id];
}
export function simulateBudget(
  items: Candidate[],
  ids: string[],
  policy: Policy,
) {
  if (new Set(items.map((c) => c.id)).size !== items.length)
    throw new Error('IDs duplicados');
  const unique = [...new Set(ids)];
  const picked = unique.map((id) => {
    const item = items.find((c) => c.id === id);
    if (!item) throw new Error('Item desconhecido');
    if (item.risk === 'protected') throw new Error('Item protegido');
    return item;
  });
  const coverage = picked.map((c) => c.coverage_key);
  if (coverage.includes(null) || new Set(coverage).size !== coverage.length)
    throw new Error('Cobertura incerta ou sobreposta');
  const logicalBytes = picked.reduce((n, c) => n + c.logical_bytes, 0);
  if (!Number.isSafeInteger(logicalBytes))
    throw new Error('Estimativa fora do limite');
  return {
    picked,
    logicalBytes,
    physicalPotential: policy === 'trash' ? 0 : null,
    measuredFreedBytes: null,
    reviewCount: picked.filter((c) => c.risk === 'review').length,
  };
}
export function executeDemo(
  id: string,
  scenario: Scenario,
  cancelled: boolean,
): Outcome {
  if (cancelled) return 'cancelled';
  if (scenario === 'partial' && id === 'py') return 'failed';
  if (scenario === 'partial' && id === 'rust') return 'skipped_changed';
  return 'simulated';
}
