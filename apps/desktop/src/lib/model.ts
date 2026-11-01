import corpus from '../../../../tests/fixtures/demo.json';
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
  supported_action: 'none';
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
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const unit =
    bytes >= 1024 ** 3 ? 3 : bytes >= 1024 ** 2 ? 2 : bytes >= 1024 ? 1 : 0;
  return `${new Intl.NumberFormat('pt-BR', { maximumFractionDigits: 1 }).format(bytes / 1024 ** unit)} ${['B', 'KiB', 'MiB', 'GiB'][unit]}`;
}
export function selectDefaults(items: Candidate[]): string[] {
  return items
    .filter((c) => c.risk === 'low' && c.selected_by_default)
    .map((c) => c.id);
}
export function toggleSelection(ids: string[], item: Candidate): string[] {
  if (item.risk === 'protected') return ids;
  return ids.includes(item.id)
    ? ids.filter((id) => id !== item.id)
    : [...ids, item.id];
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
