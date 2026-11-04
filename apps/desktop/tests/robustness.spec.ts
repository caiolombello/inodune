import { expect, test, type Page } from '@playwright/test';

// Contract drift from the backend must never blank the UI or enable cleanup.
// Synthetic IPC only; no filesystem access.
type Variant =
  | 'duplicate'
  | 'fractional'
  | 'unsafe'
  | 'missing'
  | 'overflow'
  | 'nullCoverage'
  | 'protectedUnknown';

async function mock(page: Page, variant: Variant) {
  await page.addInitScript((variant: Variant) => {
    const item = (id: string, bytes: number, coverage: string | null) => ({
      id,
      title: 'Cache npm',
      display_path: `/home/user/.npm/${id}`,
      detector_id: 'npm.cache',
      detector_version: '1',
      scope_id: '/home/user',
      category: 'Desenvolvimento',
      project: 'Pasta pessoal',
      risk: 'low',
      logical_bytes: bytes,
      allocated_bytes_estimate: bytes,
      identity: { dev: 1, ino: 1, mtime_unix: 1 },
      coverage_key: coverage,
      selected_by_default: true,
      reason: 'Cache',
      impact: 'Recriável',
      rebuild_hint: 'A ferramenta recria.',
      preserved: 'Código preservado',
      evidence: [],
      activity: 'mtime',
      supported_action: 'cache_contents',
    });
    const missing: Record<string, unknown> = item('a', 10, 'k1');
    delete missing.reason;
    const candidates =
      variant === 'duplicate'
        ? [item('a', 10, 'k1'), item('a', 20, 'k2')]
        : variant === 'fractional'
          ? [item('a', 1.5, 'k1')]
          : variant === 'unsafe'
            ? [item('a', Number.MAX_SAFE_INTEGER + 2, 'k1')]
            : variant === 'missing'
              ? [missing]
              : variant === 'overflow'
                ? [
                    item('a', Number.MAX_SAFE_INTEGER, 'k1'),
                    item('b', 10, 'k2'),
                  ]
                : variant === 'protectedUnknown'
                  ? [
                      {
                        ...item('w', 1_900_000_000, null),
                        title: 'Worktree / repositório Git',
                        category: 'Git Worktrees',
                        risk: 'protected',
                        supported_action: 'none',
                        selected_by_default: false,
                      },
                    ]
                  : [item('a', 10, null)];
    const calls: string[] = [];
    Object.assign(window, {
      isTauri: true,
      __calls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          calls.push(cmd);
          if (cmd === 'list_scopes') return [];
          if (cmd === 'begin_automatic_scan') return 'job';
          if (cmd === 'poll_scan')
            return {
              job_id: 'job',
              status: 'done',
              progress: { visited: 1, candidates: 1, stage: 'Concluído' },
              error: null,
              result: {
                scan_id: 'scan',
                candidates,
                issues: [],
                stats: { visited: 1, skipped: 0, elapsed_ms: 1 },
                complete: true,
                cancelled: false,
                volumes: [],
              },
            };
          return null;
        },
      },
    });
  }, variant);
}

for (const variant of [
  'duplicate',
  'fractional',
  'unsafe',
  'missing',
] as const) {
  test(`malformed scan result (${variant}) is refused, not applied`, async ({
    page,
  }) => {
    const errors: string[] = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    await mock(page, variant);
    await page.goto('/');
    await page
      .getByRole('button', { name: 'Analisar computador' })
      .first()
      .click();
    await expect(page.getByRole('alert')).toContainText('formato inesperado');
    await expect(page.getByRole('alert')).toContainText('nada foi alterado');
    await page
      .getByRole('button', { name: 'Desenvolvimento', exact: true })
      .click();
    await expect(page.getByRole('heading', { level: 1 })).not.toHaveText(
      /Visão geral|perspectiva/,
    );
    expect(errors).toEqual([]);
  });
}

test('an unestimable selection blocks cleanup with an explicit notice', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await mock(page, 'nullCoverage');
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  await expect(
    page.getByText('Não foi possível estimar a seleção', { exact: false }),
  ).toBeVisible();
  await expect(page.getByText('estimativa indisponível').first()).toBeVisible();
  const clean = page.getByRole('button', { name: /^Limpar/ }).first();
  if (await clean.isEnabled()) await clean.click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  const calls = await page.evaluate(
    () => (window as unknown as { __calls: string[] }).__calls,
  );
  expect(calls).not.toContain('prepare_cleanup');
  expect(calls).not.toContain('prepare_safe_cleanup');
  expect(errors).toEqual([]);
});

test('a sum beyond the safe range is accepted but cannot be cleaned', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await mock(page, 'overflow');
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  await expect(
    page.getByText('Não foi possível estimar a seleção', { exact: false }),
  ).toBeVisible();
  const calls = await page.evaluate(
    () => (window as unknown as { __calls: string[] }).__calls,
  );
  expect(calls).not.toContain('prepare_cleanup');
  expect(errors).toEqual([]);
});

test('protected items without a summable size are "not measured", never 0 B', async ({
  page,
}) => {
  await mock(page, 'protectedUnknown');
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador' })
    .first()
    .click();
  const git = page.locator('.category-card', { hasText: 'Git Worktrees' });
  await expect(git).toContainText('Não medido');
  await expect(git).toContainText('tamanho não somado');
  await expect(git).not.toContainText('0 B');
  await expect(git).not.toContainText('Nada encontrado');
});
