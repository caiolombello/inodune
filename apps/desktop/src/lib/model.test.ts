import { describe, expect, it } from 'vitest';
import {
  demo,
  selectDefaults,
  simulateBudget,
  toggleSelection,
  executeDemo,
} from './model';
describe('conservative simulation policy', () => {
  it('suggests only low risk and rejects protected IDs even from a forged selection', () => {
    expect(selectDefaults(demo.candidates)).toEqual(['thumbs', 'py']);
    expect(
      toggleSelection(
        [],
        demo.candidates.find((c) => c.id === 'ai')!,
      ),
    ).toEqual([]);
    expect(() => simulateBudget(demo.candidates, ['ai'], 'permanent')).toThrow(
      'protegido',
    );
    expect(() =>
      simulateBudget(demo.candidates, ['unknown'], 'permanent'),
    ).toThrow();
  });
  it('deduplicates estimates and never turns trash into measured free space', () => {
    const budget = simulateBudget(
      demo.candidates,
      ['thumbs', 'thumbs'],
      'trash',
    );
    expect(budget.logicalBytes).toBe(demo.candidates[0].logical_bytes);
    expect(budget.physicalPotential).toBe(0);
    expect(budget.measuredFreedBytes).toBeNull();
    expect(
      simulateBudget(demo.candidates, ['rust'], 'permanent').physicalPotential,
    ).toBeNull();
  });
  it('refuses overlapping or unknown coverage and duplicate inventory IDs', () => {
    const [a, b] = demo.candidates;
    expect(() =>
      simulateBudget(
        [a, { ...b, coverage_key: a.coverage_key }],
        [a.id, b.id],
        'trash',
      ),
    ).toThrow();
    expect(() => simulateBudget([a, a], [a.id], 'trash')).toThrow();
    expect(() =>
      simulateBudget([{ ...a, coverage_key: null }], [a.id], 'trash'),
    ).toThrow();
  });
  it('retains per-item failure/changed/cancelled and never reports a real deletion', () => {
    expect(executeDemo('py', 'partial', false)).toBe('failed');
    expect(executeDemo('rust', 'partial', false)).toBe('skipped_changed');
    expect(executeDemo('thumbs', 'normal', true)).toBe('cancelled');
    expect(executeDemo('thumbs', 'normal', false)).toBe('simulated');
  });
});
