import demoCorpus from 'virtual:inodune-demo';
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
// Dev/test builds resolve the demo corpus (validated there); release builds get an empty one.
const corpus = demoCorpus as Inventory;
export const demo = corpus;
/** Demonstração só existe em builds de desenvolvimento/teste. */
export const demoAvailable = demo.candidates.length > 0;
export const riskLabel: Record<Risk, string> = {
  low: 'Baixo risco',
  review: 'Revisar',
  protected: 'Protegido',
};
export const outcomeLabel: Record<Outcome, string> = {
  pending: 'Aguardando',
  processing: 'Processando exemplo',
  simulated: 'Concluído no exemplo',
  failed: 'Falhou no exemplo',
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
    language.value === 'en-US' ? 'occupied (not recoverable)' : 'ocupados';
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
export function isCarefulItem(item: Candidate): boolean {
  if (item.detector_id === 'node.modules') return true;
  const reason = item.reason.toLocaleLowerCase('pt-BR');
  return (
    item.detector_id === 'terraform.provider-package' &&
    (reason.includes('referenciad') || reason.includes('terraform init'))
  );
}
/** Pré-seleção da Visão geral: somente risco baixo, nunca review/protegido. */
export function selectOverviewDefaults(
  items: Candidate[],
  local = false,
): string[] {
  return items
    .filter((item) => {
      if (item.risk !== 'low' || isCarefulItem(item)) return false;
      if (isLocallyEligible(item)) return true;
      // Local plans only accept supported actions; demo keeps the fixture flag.
      return !local && item.selected_by_default;
    })
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
/**
 * Sum for display: only sizes with a known coverage key, each coverage once
 * (nested or repeated paths are not added twice). Unknown sizes are counted,
 * never treated as zero.
 */
export function measuredSum(items: Candidate[]): {
  bytes: number;
  unknown: number;
} {
  const seen = new Set<string>();
  let bytes = 0;
  let unknown = 0;
  for (const item of items) {
    if (item.coverage_key === null) {
      unknown += 1;
      continue;
    }
    if (seen.has(item.coverage_key)) continue;
    seen.add(item.coverage_key);
    bytes += item.logical_bytes;
  }
  return { bytes, unknown };
}
export type Budget = ReturnType<typeof simulateBudget>;
export type BudgetResult =
  ({ ok: true } & Budget) | { ok: false; error: string };
/**
 * Same checks as simulateBudget, as data: a bad selection or inventory becomes an
 * explicit error state for the UI (no estimate shown) instead of a render crash.
 */
export function trySimulateBudget(
  items: Candidate[],
  ids: string[],
  policy: Policy,
): BudgetResult {
  try {
    return { ok: true, ...simulateBudget(items, ids, policy) };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

const RISKS: readonly string[] = ['low', 'review', 'protected'];
const ACTIONS: readonly string[] = [
  'none',
  'cache_contents',
  'reviewed_cache_contents',
];
const isText = (value: unknown) => typeof value === 'string';
const isBytes = (value: unknown) =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
// Volume sizes are only displayed: a huge volume is shown approximately
// instead of refusing the whole analysis.
const isMeasure = (value: unknown) =>
  typeof value === 'number' && Number.isFinite(value) && value >= 0;
const TEXT_FIELDS = [
  'title',
  'display_path',
  'detector_id',
  'detector_version',
  'scope_id',
  'category',
  'project',
  'reason',
  'impact',
  'rebuild_hint',
  'preserved',
  'activity',
] as const;
/**
 * IPC contract guard for a finished scan: every field the interface reads is
 * checked before the result is applied. Returns a Portuguese reason when the
 * result cannot be shown safely (it is then never applied), or null.
 * Each size must be a safe integer; totals are not limited here (the
 * selection estimate checks its own sum).
 */
export function validateScanResult(result: unknown): string | null {
  if (!result || typeof result !== 'object')
    return 'resultado ausente ou inválido';
  const r = result as Record<string, unknown>;
  if (!isText(r.scan_id)) return 'resultado sem identificador';
  if (typeof r.complete !== 'boolean' || typeof r.cancelled !== 'boolean')
    return 'estado da análise ausente';
  if (!Array.isArray(r.issues)) return 'lista de avisos ausente';
  for (const issue of r.issues as Record<string, unknown>[])
    if (
      !issue ||
      !isText(issue.display_path) ||
      !isText(issue.code) ||
      !isText(issue.message)
    )
      return 'aviso da análise inválido';
  if (!Array.isArray(r.volumes)) return 'lista de volumes ausente';
  for (const volume of r.volumes as Record<string, unknown>[])
    if (
      !volume ||
      !isText(volume.scope_path) ||
      typeof volume.device !== 'number' ||
      !isMeasure(volume.total_bytes) ||
      !isMeasure(volume.available_bytes)
    )
      return 'volume inválido';
  const stats = r.stats as Record<string, unknown> | undefined;
  if (!stats || !isMeasure(stats.visited) || !isMeasure(stats.skipped))
    return 'estatísticas da análise ausentes';
  // skip_reasons has a serde default; when present it is a map of counts.
  const reasons = stats.skip_reasons;
  if (
    reasons !== undefined &&
    (!reasons ||
      typeof reasons !== 'object' ||
      Array.isArray(reasons) ||
      !Object.values(reasons).every(isMeasure))
  )
    return 'motivos de exclusão inválidos';
  const candidates = r.candidates;
  if (!Array.isArray(candidates)) return 'lista de itens ausente';
  const ids = new Set<string>();
  for (const item of candidates as Record<string, unknown>[]) {
    if (!item || !isText(item.id) || !item.id) return 'item sem identificador';
    if (ids.has(item.id as string)) return 'identificadores duplicados';
    ids.add(item.id as string);
    if (!isBytes(item.logical_bytes)) return 'tamanho de item inválido';
    if (
      item.allocated_bytes_estimate !== null &&
      !isBytes(item.allocated_bytes_estimate)
    )
      return 'tamanho de item inválido';
    if (!isText(item.risk) || !RISKS.includes(item.risk as string))
      return 'nível de risco desconhecido';
    if (
      !isText(item.supported_action) ||
      !ACTIONS.includes(item.supported_action as string)
    )
      return 'ação de limpeza desconhecida';
    for (const field of TEXT_FIELDS)
      if (!isText(item[field])) return 'item sem nome, caminho ou explicação';
    if (item.coverage_key !== null && !isText(item.coverage_key))
      return 'cobertura de item inválida';
    if (typeof item.selected_by_default !== 'boolean')
      return 'item sem seleção padrão definida';
    if (!Array.isArray(item.evidence) || !item.evidence.every(isText))
      return 'evidências de item inválidas';
  }
  return null;
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

/**
 * State after a finished scan: the new result replaces the previous one
 * entirely (no merge with old rows); selection is recomputed from it.
 */
export function applyScanResult(
  result: { candidates: Candidate[]; complete: boolean; cancelled: boolean },
  automatic: boolean,
): { candidates: Candidate[]; selected: string[] } {
  const candidates = [...result.candidates];
  const selected =
    !result.cancelled && (result.complete || automatic)
      ? selectOverviewDefaults(candidates, true)
      : [];
  return { candidates, selected };
}
