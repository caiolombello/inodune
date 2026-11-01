import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { readFileSync } from 'node:fs';
import type { Candidate } from '../src/lib/model';

const corpus = JSON.parse(
  readFileSync(
    new URL('../../../tests/fixtures/demo.json', import.meta.url),
    'utf8',
  ),
) as { candidates: Candidate[] };

test('local cleanup reports every fixture file and never attributes volume delta', async ({
  page,
}) => {
  const cache = {
    ...corpus.candidates[0],
    id: 'fixture-cache',
    title: 'Cache de fixture',
    display_path: '/fixture/project/__pycache__',
    project: 'Projeto de fixture',
    scope_id: 'fixture-scope',
    risk: 'low' as const,
    supported_action: 'cache_contents' as const,
    logical_bytes: 19,
    allocated_bytes_estimate: 16,
  };
  const review = {
    ...corpus.candidates[1],
    id: 'fixture-review',
    title: 'Build para revisar',
    display_path: '/fixture/project/target',
    project: 'Projeto de fixture',
    scope_id: 'fixture-scope',
    risk: 'review' as const,
    supported_action: 'cache_contents' as const,
  };
  const report = {
    schema_version: 1,
    plan_id: 'fixture-plan',
    action: 'trash',
    items: [
      {
        candidate_id: 'fixture-cache',
        complete: false,
        files: [
          {
            display_path: '/fixture/project/__pycache__/module.pyc',
            status: 'trashed',
            logical_bytes: 10,
            message: 'Moved into fixture trash.',
            preserved_at: null,
          },
          {
            display_path: '/fixture/project/__pycache__/changed.pyc',
            status: 'quarantined',
            logical_bytes: 9,
            message: 'Identity changed during fixture execution.',
            preserved_at: '/fixture/quarantine/changed.pyc',
          },
        ],
      },
    ],
    volumes: [
      {
        device: 9,
        available_before: 1000,
        available_after: 1010,
        observed_delta: 10,
      },
    ],
    complete: false,
    cancelled: false,
    logical_bytes_processed: 10,
    attributed_freed_bytes: null,
    measurement_note:
      'Fixture observation only. Volume changes are not attributed to cleanup.',
    warnings: ['Fixture warning: quarantine requires manual review.'],
    history_path: '/fixture/history/report.json',
  };
  await page.addInitScript(
    ({ items, cleanupReport }) => {
      let pollCount = 0;
      let finishCleanup: (() => void) | undefined;
      const finalPoll = new Promise<void>((resolve) => {
        finishCleanup = resolve;
      });
      Object.assign(window, {
        isTauri: true,
        __TAURI_INTERNALS__: {
          invoke: async (cmd: string, args: Record<string, unknown>) => {
            const target = window as unknown as {
              __nativeCalls?: { cmd: string; args: Record<string, unknown> }[];
            };
            target.__nativeCalls ??= [];
            target.__nativeCalls.push({ cmd, args });
            if (cmd === 'list_scopes')
              return [
                {
                  id: 'fixture-scope',
                  label: 'Pasta de fixture',
                  display_path: '/fixture',
                  kind: 'projects',
                },
              ];
            if (cmd === 'begin_scan') return 'fixture-scan-job';
            if (cmd === 'poll_scan')
              return {
                job_id: 'fixture-scan-job',
                status: 'done',
                progress: {
                  visited: items.length,
                  candidates: items.length,
                  stage: 'Fixture',
                },
                result: {
                  scan_id: 'fixture-scan',
                  candidates: items,
                  issues: [],
                  stats: { visited: items.length, skipped: 0, elapsed_ms: 0 },
                  complete: true,
                  cancelled: false,
                  volumes: [
                    {
                      scope_path: '/fixture',
                      device: 9,
                      total_bytes: 2000,
                      available_bytes: 1000,
                    },
                  ],
                },
                error: null,
              };
            if (cmd === 'prepare_cleanup')
              return {
                id: 'fixture-plan',
                hash: 'fixture-hash',
                snapshot_id: 'fixture-snapshot',
                action: args.action,
                expires_in_seconds: 120,
                items: [
                  {
                    candidate_id: 'fixture-cache',
                    display_path: '/fixture/project/__pycache__',
                    logical_bytes: 19,
                    allocated_bytes_estimate: 16,
                  },
                ],
                logical_bytes_estimate: 19,
                allocated_bytes_estimate: 16,
              };
            if (cmd === 'begin_cleanup') return 'fixture-cleanup-job';
            if (cmd === 'poll_cleanup') {
              pollCount += 1;
              if (pollCount === 1)
                return {
                  status: 'running',
                  progress: {
                    completed_files: 1,
                    total_files: 2,
                    completed_items: 0,
                    total_items: 1,
                  },
                  report: null,
                  error: null,
                };
              await finalPoll;
              return {
                status: 'done',
                progress: {
                  completed_files: 2,
                  total_files: 2,
                  completed_items: 1,
                  total_items: 1,
                },
                report: cleanupReport,
                error: null,
              };
            }
            if (cmd === 'cancel_cleanup') return null;
            throw new Error('Unexpected fixture command: ' + cmd);
          },
        },
        __finishCleanup: () => finishCleanup?.(),
      });
    },
    { items: [cache, review], cleanupReport: report },
  );

  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  await expect(
    page.getByRole('button', { name: 'Analisar pastas', exact: true }),
  ).toBeEnabled();
  await page
    .getByRole('button', { name: 'Analisar pastas', exact: true })
    .click();
  await page.getByRole('button', { name: 'Revisar candidatos' }).click();
  await expect(
    page.getByRole('checkbox', { name: 'Selecionar Cache de fixture' }),
  ).toBeChecked();
  await expect(
    page.getByRole('checkbox', { name: 'Selecionar Build para revisar' }),
  ).toBeDisabled();
  await page.getByRole('button', { name: 'Revisar impacto' }).click();
  await page.getByRole('button', { name: 'Preparar limpeza segura' }).click();
  await expect(page.getByRole('dialog')).toContainText('fixture-snapshot');
  await expect(page.getByRole('dialog')).toContainText('__pycache__');
  await page.getByRole('checkbox').check();
  await page
    .getByRole('button', { name: 'Prosseguir para confirmação nativa' })
    .click();
  await expect(page.getByText('1 de 2 arquivos')).toBeVisible();
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .last()
    .click();
  await expect(page.getByLabel('Fonte de dados')).toBeDisabled();
  await expect(
    page.getByRole('button', { name: 'Analisar pastas aprovadas' }),
  ).toBeDisabled();
  await expect(
    page.locator('.scope-option input[type="checkbox"]'),
  ).toBeDisabled();
  await page.getByRole('button', { name: 'Histórico', exact: true }).click();
  await page.evaluate(() => {
    (window as unknown as { __finishCleanup?: () => void }).__finishCleanup?.();
  });
  await expect(page.getByText('module.pyc')).toBeVisible();
  await expect(
    page.getByText('/fixture/project/__pycache__/changed.pyc'),
  ).toBeVisible();
  await expect(page.getByText('Enviado à Lixeira')).toBeVisible();
  await expect(
    page.getByText('Preservado · verificar recuperação'),
  ).toBeVisible();
  await expect(page.getByText('/fixture/quarantine/changed.pyc')).toBeVisible();
  await expect(page.getByText('Não atribuídos')).toBeVisible();
  await expect(
    page.getByText('Variações observadas não são atribuídas à limpeza.'),
  ).toBeVisible();
  await expect(
    page.getByText('Fixture warning: quarantine requires manual review.'),
  ).toBeVisible();
  await expect(page.getByText('/fixture/history/report.json')).toBeVisible();
  const path = resolve(
    process.cwd(),
    '../../docs/evidence/f2/screenshots/chromium-cleanup-report.png',
  );
  await mkdir(resolve(path, '..'), { recursive: true });
  await page.screenshot({ path, fullPage: true, animations: 'disabled' });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect
    .poll(() =>
      page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
    )
    .toBe(true);
  await page.screenshot({
    path: resolve(path, '..', 'chromium-cleanup-report-narrow.png'),
    fullPage: true,
    animations: 'disabled',
  });
  const calls = await page.evaluate(
    () =>
      (
        window as unknown as {
          __nativeCalls: { cmd: string; args: Record<string, unknown> }[];
        }
      ).__nativeCalls,
  );
  const prepare = calls.find((call) => call.cmd === 'prepare_cleanup');
  expect(prepare?.args).toMatchObject({
    scanId: 'fixture-scan',
    candidateIds: ['fixture-cache'],
    action: 'trash',
  });
  expect(calls.find((call) => call.cmd === 'begin_cleanup')?.args).toEqual({
    planId: 'fixture-plan',
    hash: 'fixture-hash',
  });
});
