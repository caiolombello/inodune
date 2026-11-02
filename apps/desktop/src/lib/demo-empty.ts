// Build de release: nenhum dado de demonstração no bundle.
import type { Inventory } from './model';
const empty: Inventory = {
  schema_version: 1,
  mode: 'simulation',
  scopes: [],
  volume: { label: '', total_bytes: 0, used_bytes: 0 },
  candidates: [],
};
export const catalog: Record<string, string> = {};
export default empty;
