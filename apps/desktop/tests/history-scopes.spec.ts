import { expect, test } from '@playwright/test';

test('history presents persisted reports and exports only after explicit selection', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: { cmd: string; args: Record<string, unknown> }[] = [];
    const scopes: {
      id: string;
      label: string;
      display_path: string;
      kind: string;
      exclusions: string[];
    }[] = [
      {
        id: 'scope-fixture',
        label: 'Fixture projects',
        display_path: '/tmp/inodune-test/projects',
        kind: 'projects',
        exclusions: [],
      },
    ];
    const entries = [
      {
        report_id: 'report-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        created_unix: 1_790_000_000,
        plan_id: 'plan-fixture-a',
        action: 'trash',
        complete: true,
        cancelled: false,
        logical_bytes_processed: 19,
        files_count: 1,
        error: null,
      },
      {
        report_id: 'report-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        created_unix: 1_790_000_001,
        plan_id: '',
        action: null,
        complete: false,
        cancelled: false,
        logical_bytes_processed: 0,
        files_count: 0,
        error: 'Unsupported report schema',
      },
    ];
    const report = {
      schema_version: 1,
      plan_id: 'plan-fixture-a',
      action: 'trash',
      items: [
        {
          candidate_id: 'candidate-fixture',
          complete: true,
          files: [
            {
              display_path:
                '/tmp/inodune-test/projects/app/__pycache__/module.pyc',
              status: 'trashed',
              logical_bytes: 19,
              message: 'Fixture moved to trash.',
              preserved_at: null,
              durability_confirmed: true,
            },
          ],
        },
      ],
      volumes: [],
      complete: true,
      cancelled: false,
      logical_bytes_processed: 19,
      attributed_freed_bytes: null,
      measurement_note: 'Filesystem observation only.',
      warnings: [],
      history_path: null,
    };
    Object.assign(window, {
      isTauri: true,
      __nativeCalls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          calls.push({ cmd, args });
          if (cmd === 'list_scopes') return scopes;
          if (cmd === 'list_history') return entries;
          if (cmd === 'get_history_report') {
            if (args.reportId === entries[1].report_id)
              throw new Error('Unsupported report schema');
            return report;
          }
          if (cmd === 'export_history_report') {
            return (
              (window as unknown as { __nextExport?: string | null })
                .__nextExport ?? null
            );
          }
          if (cmd === 'set_scope_exclusions') {
            scopes[0].exclusions = args.relativePaths as string[];
            return scopes[0];
          }
          if (cmd === 'forget_scope') return null;
          if (cmd === 'begin_scan') return 'fixture-scan-job';
          if (cmd === 'poll_scan')
            return {
              job_id: 'fixture-scan-job',
              status: 'done',
              progress: { visited: 1, candidates: 0, stage: 'Fixture' },
              result: {
                scan_id: 'scan-after-exclusion',
                candidates: [],
                issues: [],
                stats: { visited: 1, skipped: 1, elapsed_ms: 1 },
                complete: true,
                cancelled: false,
                volumes: [],
              },
              error: null,
            };
          throw new Error(`Unexpected fixture command: ${cmd}`);
        },
      },
      __nextExport: null,
    });
  });

  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  await page.getByRole('button', { name: 'Histórico', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Histórico local' }),
  ).toBeVisible();
  await expect(page.locator('.entries button')).toHaveCount(2);

  await page.getByRole('button', { name: /Lixeira · Concluído/ }).click();
  await expect(page.getByText('plan-fixture-a')).toBeVisible();
  await expect(
    page.getByText('/tmp/inodune-test/projects/app/__pycache__/module.pyc'),
  ).toBeVisible();
  const calls = () =>
    page.evaluate(
      () =>
        (
          window as unknown as {
            __nativeCalls: { cmd: string; args: Record<string, unknown> }[];
          }
        ).__nativeCalls,
    );
  await page.getByRole('button', { name: 'Exportar JSON local' }).click();
  await expect(page.getByText('Relatório exportado')).toHaveCount(0);
  await page.evaluate(() => {
    (window as unknown as { __nextExport: string }).__nextExport =
      '/tmp/inodune-export-fixture/report.json';
  });
  await page.getByRole('button', { name: 'Exportar JSON local' }).click();
  await expect(
    page
      .getByRole('status')
      .filter({ hasText: '/tmp/inodune-export-fixture/report.json' }),
  ).toBeVisible();
  expect(
    (await calls()).filter((call) => call.cmd === 'export_history_report'),
  ).toHaveLength(2);

  await page.getByRole('button', { name: /Relatório com erro/ }).click();
  await expect(page.getByRole('alert')).toContainText(
    'Unsupported report schema',
  );
  await expect(
    page.getByText('Relatório indisponível', { exact: true }),
  ).toBeVisible();
  expect(
    (await calls())
      .filter((call) => call.cmd === 'get_history_report')
      .map((call) => call.args.reportId),
  ).toEqual([
    'report-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    'report-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
  ]);
});

test('scope exclusions send only opaque scope ID and literal relative prefixes, then revoke approval', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: { cmd: string; args: Record<string, unknown> }[] = [];
    const scopes: {
      id: string;
      label: string;
      display_path: string;
      kind: string;
      exclusions: string[];
    }[] = [
      {
        id: 'opaque-scope-7f12',
        label: 'Fixture projects',
        display_path: '/tmp/private-fixture-root',
        kind: 'projects',
        exclusions: [],
      },
    ];
    Object.assign(window, {
      isTauri: true,
      __nativeCalls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          calls.push({ cmd, args });
          if (cmd === 'list_scopes') return scopes;
          if (cmd === 'set_scope_exclusions') {
            scopes[0].exclusions = args.relativePaths as string[];
            return scopes[0];
          }
          if (cmd === 'forget_scope') return null;
          throw new Error(`Unexpected fixture command: ${cmd}`);
        },
      },
    });
  });

  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .last()
    .click();
  await expect(
    page.getByRole('heading', { name: 'Controles dos escopos' }),
  ).toBeVisible();
  await page.getByLabel('Caminho relativo a excluir').fill('vendor/generated');
  await page.getByRole('button', { name: 'Adicionar exclusão' }).click();
  await expect(page.getByText('Exclusões atualizadas.')).toBeVisible();
  await expect(page.getByText('vendor/generated')).toBeVisible();

  const calls = () =>
    page.evaluate(
      () =>
        (
          window as unknown as {
            __nativeCalls: { cmd: string; args: Record<string, unknown> }[];
          }
        ).__nativeCalls,
    );
  const setCall = (await calls()).find(
    (call) => call.cmd === 'set_scope_exclusions',
  );
  expect(setCall?.args).toEqual({
    scopeId: 'opaque-scope-7f12',
    relativePaths: ['vendor/generated'],
  });
  expect(JSON.stringify(setCall?.args)).not.toContain(
    '/tmp/private-fixture-root',
  );
  expect(JSON.stringify(setCall?.args)).not.toContain('vendor/generated/..');

  await page.getByRole('button', { name: 'Revogar aprovação' }).click();
  await expect(page.getByText('Fixture projects')).toHaveCount(0);
  const revoke = (await calls()).find((call) => call.cmd === 'forget_scope');
  expect(revoke?.args).toEqual({ scopeId: 'opaque-scope-7f12' });
  expect(JSON.stringify(revoke?.args)).not.toContain(
    '/tmp/private-fixture-root',
  );
});
