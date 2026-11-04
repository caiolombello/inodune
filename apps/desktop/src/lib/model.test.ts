import { describe, expect, it } from 'vitest';
import {
  applyScanResult,
  demo,
  selectDefaults,
  selectOverviewDefaults,
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
  validateScanResult,
  trySimulateBudget,
  measuredSum,
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
    expect(toggleSelection([low.id], reviewed)).toEqual([low.id, reviewed.id]);
    expect(toggleSelection([reviewed.id], low)).toEqual([reviewed.id, low.id]);
    expect(toggleSelection([reviewed.id], reviewed)).toEqual([]);
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
      '2 itens protegidos · 12 KiB ocupados',
    );
    expect(protectedOnlyLabel(1, '0 B')).toBe(
      '1 item protegido · 0 B ocupados',
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

it('overview preselects only low risk eligible items', () => {
  const low = {
    ...demo.candidates[0],
    risk: 'low' as const,
    supported_action: 'cache_contents' as const,
    selected_by_default: true,
  };
  const review = {
    ...low,
    id: 'review',
    risk: 'review' as const,
    supported_action: 'reviewed_cache_contents' as const,
  };
  expect(selectOverviewDefaults([low, review])).toEqual([low.id]);
});

it('local overview preselection only takes eligible low-risk items', () => {
  const base = demo.candidates[0];
  const eligible = {
    ...base,
    id: 'a',
    risk: 'low' as const,
    supported_action: 'cache_contents' as const,
  };
  const unsupported = {
    ...base,
    id: 'b',
    risk: 'low' as const,
    selected_by_default: true,
    supported_action: 'none' as const,
  };
  const reviewed = {
    ...base,
    id: 'c',
    risk: 'review' as const,
    supported_action: 'reviewed_cache_contents' as const,
  };
  const locked = {
    ...base,
    id: 'd',
    risk: 'protected' as const,
    supported_action: 'none' as const,
  };
  const careful = { ...eligible, id: 'e', detector_id: 'node.modules' };
  expect(
    selectOverviewDefaults(
      [eligible, unsupported, reviewed, locked, careful],
      true,
    ),
  ).toEqual(['a']);
});

it('a new scan result replaces the previous rows instead of merging', () => {
  const base = demo.candidates[0];
  const stale = {
    ...base,
    id: 'old-pip',
    risk: 'protected' as const,
    reason: 'Cobertura incompleta: protegido.',
  };
  const fresh = {
    ...base,
    id: 'new-pip',
    risk: 'low' as const,
    supported_action: 'cache_contents' as const,
  };
  const first = applyScanResult(
    { candidates: [stale], complete: true, cancelled: false },
    true,
  );
  expect(first.candidates.map((c) => c.id)).toEqual(['old-pip']);
  const second = applyScanResult(
    { candidates: [fresh], complete: false, cancelled: false },
    true,
  );
  expect(second.candidates.map((c) => c.id)).toEqual(['new-pip']);
  expect(second.selected).toEqual(['new-pip']);
  expect(
    applyScanResult(
      { candidates: [fresh], complete: true, cancelled: true },
      true,
    ).selected,
  ).toEqual([]);
});

describe('IPC contract guard', () => {
  const base = demo.candidates[0];
  const result = (candidates: unknown[]) => ({
    scan_id: 'scan',
    candidates,
    issues: [],
    stats: { visited: 1, skipped: 0, elapsed_ms: 1 },
    complete: true,
    cancelled: false,
    volumes: [],
  });
  it('accepts a well-formed scan result', () => {
    expect(validateScanResult(result(demo.candidates))).toBeNull();
  });
  it('rejects duplicate IDs, fractional or unsafe sizes and unknown risks', () => {
    expect(validateScanResult(result([base, base]))).toMatch(/duplicad/);
    expect(
      validateScanResult(result([{ ...base, logical_bytes: 1.5 }])),
    ).toMatch(/tamanho/);
    expect(
      validateScanResult(result([{ ...base, logical_bytes: -1 }])),
    ).toMatch(/tamanho/);
    expect(
      validateScanResult(
        result([{ ...base, logical_bytes: Number.MAX_SAFE_INTEGER + 2 }]),
      ),
    ).toMatch(/tamanho/);
    expect(validateScanResult(result([{ ...base, risk: 'unknown' }]))).toMatch(
      /risco/,
    );
    expect(validateScanResult({ candidates: 'x' })).not.toBeNull();
    expect(validateScanResult(null)).not.toBeNull();
  });
  it('turns budget failures into an explicit error state instead of throwing', () => {
    const ok = trySimulateBudget(demo.candidates, ['thumbs'], 'trash');
    expect(ok.ok).toBe(true);
    const failed = trySimulateBudget(demo.candidates, ['ai'], 'trash');
    expect(failed.ok).toBe(false);
    if (!failed.ok) expect(failed.error).toMatch(/protegido/);
  });
});

describe('measured totals', () => {
  const [a, b] = demo.candidates;
  it('adds known sizes once and counts unknown coverage apart', () => {
    expect(measuredSum([a, b])).toEqual({
      bytes: a.logical_bytes + b.logical_bytes,
      unknown: 0,
    });
    expect(measuredSum([a, { ...b, coverage_key: a.coverage_key }]).bytes).toBe(
      a.logical_bytes,
    );
    expect(measuredSum([a, { ...b, coverage_key: null }])).toEqual({
      bytes: a.logical_bytes,
      unknown: 1,
    });
    expect(measuredSum([])).toEqual({ bytes: 0, unknown: 0 });
  });
});

describe('IPC contract guard covers every consumed field', () => {
  const base = demo.candidates[0];
  const ok = {
    scan_id: 'scan',
    candidates: [base],
    issues: [],
    stats: { visited: 1, skipped: 0, elapsed_ms: 1 },
    complete: true,
    cancelled: false,
    volumes: [],
  };
  it('accepts the complete native shape', () => {
    expect(validateScanResult(ok)).toBeNull();
  });
  it('rejects candidates missing text the UI reads', () => {
    for (const field of [
      'reason',
      'impact',
      'rebuild_hint',
      'preserved',
      'activity',
      'project',
      'category',
      'detector_id',
    ]) {
      const broken = { ...base } as Record<string, unknown>;
      delete broken[field];
      expect(
        validateScanResult({ ...ok, candidates: [broken] }),
        field,
      ).not.toBeNull();
    }
    expect(
      validateScanResult({ ...ok, candidates: [{ ...base, evidence: null }] }),
    ).not.toBeNull();
    expect(
      validateScanResult({ ...ok, candidates: [{ ...base, coverage_key: 7 }] }),
    ).not.toBeNull();
  });
  it('rejects a result without issues, volumes or flags', () => {
    expect(validateScanResult({ ...ok, issues: undefined })).not.toBeNull();
    expect(validateScanResult({ ...ok, volumes: null })).not.toBeNull();
    expect(validateScanResult({ ...ok, complete: 'yes' })).not.toBeNull();
    expect(validateScanResult({ ...ok, scan_id: 3 })).not.toBeNull();
    expect(
      validateScanResult({
        ...ok,
        volumes: [{ scope_path: '/', device: 1, total_bytes: 1.5 }],
      }),
    ).not.toBeNull();
  });
  it('does not refuse a large inventory only because of its total', () => {
    const big = Number.MAX_SAFE_INTEGER - 1;
    expect(
      validateScanResult({
        ...ok,
        candidates: [
          { ...base, id: 'a', logical_bytes: big },
          { ...base, id: 'b', logical_bytes: big },
        ],
      }),
    ).toBeNull();
  });
});

describe('IPC contract guard: statistics and volumes', () => {
  const base = demo.candidates[0];
  const ok = {
    scan_id: 'scan',
    candidates: [base],
    issues: [],
    stats: { visited: 1, skipped: 0, elapsed_ms: 1, skip_reasons: { a: 2 } },
    complete: true,
    cancelled: false,
    volumes: [],
  };
  it('rejects malformed skip reasons but accepts them absent', () => {
    expect(validateScanResult(ok)).toBeNull();
    const { skip_reasons: _omit, ...without } = ok.stats;
    void _omit;
    expect(validateScanResult({ ...ok, stats: without })).toBeNull();
    for (const bad of ['x', [1], { a: 'many' }, { a: -1 }, null])
      expect(
        validateScanResult({
          ...ok,
          stats: { ...ok.stats, skip_reasons: bad },
        }),
        JSON.stringify(bad),
      ).not.toBeNull();
  });
  it('shows a huge volume instead of refusing the whole analysis', () => {
    const huge = 2 ** 60;
    expect(
      validateScanResult({
        ...ok,
        volumes: [
          { scope_path: '/', device: 1, total_bytes: huge, available_bytes: 1 },
        ],
      }),
    ).toBeNull();
    expect(
      validateScanResult({
        ...ok,
        volumes: [
          { scope_path: '/', device: 1, total_bytes: -1, available_bytes: 1 },
        ],
      }),
    ).not.toBeNull();
  });
});
