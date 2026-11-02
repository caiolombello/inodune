import { describe, expect, it } from 'vitest';
import {
  demo,
  selectDefaults,
  simulateBudget,
  toggleSelection,
  executeDemo,
  isLocallyEligible,
  selectLocalDefaults,
  formatSignedBytes,
  compareInstalledSize,
  cleanupBatches,
  cleanupBatchWarning,
  protectedOnlyLabel,
  CLEANUP_BATCH_LIMIT,
} from './model';
describe('conservative simulation policy', () => {
  it('formats signed observed volume deltas by magnitude', () => {
    expect(formatSignedBytes(1024)).toBe('+1 KiB');
    expect(formatSignedBytes(-1024)).toBe('−1 KiB');
    expect(formatSignedBytes(0)).toBe('0 B');
  });
  it('allows low cache contents and individually reviewed cache contents', () => {
    const lowCache = {
      ...demo.candidates[0],
      supported_action: 'cache_contents' as const,
    };
    const reviewCache = { ...lowCache, risk: 'review' as const };
    const unsupportedLow = { ...demo.candidates[0] };
    expect(isLocallyEligible(lowCache)).toBe(true);
    const reviewedContents = {
      ...reviewCache,
      id: 'reviewed-cache',
      selected_by_default: true,
      supported_action: 'reviewed_cache_contents' as const,
    };
    expect(isLocallyEligible(reviewCache)).toBe(false);
    expect(isLocallyEligible(reviewedContents)).toBe(true);
    expect(isLocallyEligible(unsupportedLow)).toBe(false);
    expect(
      selectLocalDefaults([lowCache, reviewedContents, unsupportedLow]),
    ).toEqual([lowCache.id]);
  });
  it('selects review items together with other deletable items', () => {
    const low = { ...demo.candidates[0], risk: 'low' as const };
    const reviewed = {
      ...low,
      id: 'reviewed',
      risk: 'review' as const,
      supported_action: 'reviewed_cache_contents' as const,
    };
    expect(toggleSelection([low.id], reviewed, [low, reviewed])).toEqual([
      low.id,
      reviewed.id,
    ]);
    expect(toggleSelection([reviewed.id], low, [low, reviewed])).toEqual([
      reviewed.id,
      low.id,
    ]);
    expect(toggleSelection([reviewed.id], reviewed, [low, reviewed])).toEqual(
      [],
    );
  });
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
  it('sorts unknown installed sizes after known sizes', () => {
    const sizes = [null, 10, null, 30, 10];
    expect(sizes.slice().sort(compareInstalledSize)).toEqual([
      30,
      10,
      10,
      null,
      null,
    ]);
    expect(compareInstalledSize(null, null)).toBe(0);
    expect(Number.isNaN(compareInstalledSize(null, 1))).toBe(false);
  });
  it('labels a protected-only card as occupied space that cannot be reclaimed', () => {
    expect(protectedOnlyLabel(2, '12 KiB')).toBe(
      '2 itens protegidos · 12 KiB ocupados (não recuperável)',
    );
    expect(protectedOnlyLabel(1, '0 B')).toBe(
      '1 item protegido · 0 B ocupados (não recuperável)',
    );
  });
  it('splits a selection above the plan limit into ordered batches and warns', () => {
    const ids = Array.from(
      { length: CLEANUP_BATCH_LIMIT + 2 },
      (_, index) => `item-${index}`,
    );
    ids.push('item-0');
    const batches = cleanupBatches(ids);
    expect(batches).toHaveLength(2);
    expect(batches[0]).toHaveLength(CLEANUP_BATCH_LIMIT);
    expect(batches[0][0]).toBe('item-0');
    expect(batches[1]).toEqual(['item-1000', 'item-1001']);
    expect(cleanupBatchWarning(CLEANUP_BATCH_LIMIT)).toBeNull();
    expect(cleanupBatchWarning(CLEANUP_BATCH_LIMIT + 2)).toContain(
      'dividida em 2 lotes',
    );
    expect(cleanupBatchWarning(CLEANUP_BATCH_LIMIT + 2)).toContain('1.002');
  });
  it('retains per-item failure/changed/cancelled and never reports a real deletion', () => {
    expect(executeDemo('py', 'partial', false)).toBe('failed');
    expect(executeDemo('rust', 'partial', false)).toBe('skipped_changed');
    expect(executeDemo('thumbs', 'normal', true)).toBe('cancelled');
    expect(executeDemo('thumbs', 'normal', false)).toBe('simulated');
  });
});
