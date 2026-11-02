import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import type { Candidate } from '../src/lib/model';

const corpus = JSON.parse(
  readFileSync(
    new URL('../../../tests/fixtures/demo.json', import.meta.url),
    'utf8',
  ),
) as { candidates: Candidate[] };

test('starts an automatic local scan without choosing folders and refreshes discovered scopes', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    let started = false;
    const discovered = [
      {
        id: 'fixture-home',
        label: 'Home fixture',
        display_path: '/tmp/inodune-fixture/home',
        kind: 'projects',
        exclusions: [],
      },
    ];
    Object.assign(window, {
      isTauri: true,
      __nativeCalls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          calls.push(cmd);
          if (cmd === 'list_scopes') return started ? discovered : [];
          if (cmd === 'begin_automatic_scan') {
            started = true;
            return 'fixture-automatic-job';
          }
          if (cmd === 'poll_scan')
            return {
              job_id: 'fixture-automatic-job',
              status: 'done',
              progress: { visited: 12, candidates: 0, stage: 'Fixture' },
              result: {
                scan_id: 'fixture-automatic-result',
                candidates: [],
                issues: [],
                stats: { visited: 12, skipped: 2, elapsed_ms: 3 },
                complete: true,
                cancelled: false,
                volumes: [],
              },
              error: null,
            };
          throw new Error(`Unexpected fixture command: ${cmd}`);
        },
      },
    });
  });

  await page.goto('/');
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  await expect(
    page.getByRole('combobox', { name: 'Modo de análise' }),
  ).toHaveValue('automatic');
  await page.getByRole('button', { name: 'Analisar computador' }).click();

  await expect(
    page.getByRole('heading', { name: 'Seu espaço, em perspectiva.' }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Analisar novamente' }),
  ).toBeVisible();
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  const calls = await page.evaluate(
    () => (window as unknown as { __nativeCalls: string[] }).__nativeCalls,
  );
  expect(calls.filter((call) => call === 'list_scopes')).toHaveLength(2);
  expect(calls).toContain('begin_automatic_scan');
  expect(calls).not.toContain('choose_scope');
  expect(calls).not.toContain('begin_scan');
});

test('keeps chosen-scope scanning available as an advanced mode', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    const scopes = [
      {
        id: 'fixture-projects',
        label: 'Project fixture',
        display_path: '/tmp/inodune-fixture/projects',
        kind: 'projects',
        exclusions: [],
      },
    ];
    Object.assign(window, {
      isTauri: true,
      __nativeCalls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          calls.push(cmd);
          if (cmd === 'list_scopes') return scopes;
          if (cmd === 'begin_scan') return 'fixture-manual-job';
          if (cmd === 'poll_scan')
            return {
              job_id: 'fixture-manual-job',
              status: 'done',
              progress: { visited: 4, candidates: 0, stage: 'Fixture' },
              result: {
                scan_id: 'fixture-manual-result',
                candidates: [],
                issues: [],
                stats: { visited: 4, skipped: 0, elapsed_ms: 2 },
                complete: true,
                cancelled: false,
                volumes: [],
              },
              error: null,
            };
          throw new Error(`Unexpected fixture command: ${cmd}`);
        },
      },
    });
  });

  await page.goto('/');
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  await page
    .getByRole('combobox', { name: 'Modo de análise' })
    .selectOption('manual');
  await expect(
    page.getByRole('button', { name: 'Analisar pastas aprovadas' }),
  ).toBeEnabled();
  await page.getByRole('button', { name: 'Analisar pastas aprovadas' }).click();
  const calls = await page.evaluate(
    () => (window as unknown as { __nativeCalls: string[] }).__nativeCalls,
  );
  expect(calls).toContain('begin_scan');
  expect(calls).not.toContain('begin_automatic_scan');
});

test('allows reviewing a selected complete item from a partial automatic scan', async ({
  page,
}) => {
  const candidate = {
    ...corpus.candidates[0],
    id: 'partial-fixture-cache',
    title: 'Cache parcial de fixture',
    display_path: '/tmp/inodune-fixture/project/__pycache__',
    project: 'Projeto parcial de fixture',
    scope_id: 'fixture-home',
    risk: 'low' as const,
    supported_action: 'cache_contents' as const,
    logical_bytes: 21,
  };
  await page.addInitScript(
    ({ candidate: item }) => {
      Object.assign(window, {
        isTauri: true,
        __TAURI_INTERNALS__: {
          invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
            if (cmd === 'list_scopes') return [];
            if (cmd === 'begin_automatic_scan') return 'fixture-partial-job';
            if (cmd === 'poll_scan')
              return {
                job_id: 'fixture-partial-job',
                status: 'done',
                progress: { visited: 8, candidates: 1, stage: 'Fixture' },
                result: {
                  scan_id: 'fixture-partial-scan',
                  candidates: [item],
                  issues: [
                    {
                      display_path: '/home/fixture/private',
                      code: 'permission_denied',
                      message: 'Permission denied',
                    },
                  ],
                  stats: { visited: 8, skipped: 1, elapsed_ms: 3 },
                  complete: false,
                  cancelled: false,
                  volumes: [],
                },
                error: null,
              };
            if (cmd === 'prepare_cleanup')
              return {
                id: 'fixture-partial-plan',
                hash: 'fixture-partial-hash',
                snapshot_id: 'fixture-partial-snapshot',
                action: args.action,
                expires_in_seconds: 120,
                items: [
                  {
                    candidate_id: 'partial-fixture-cache',
                    display_path: '/tmp/inodune-fixture/project/__pycache__',
                    logical_bytes: 21,
                    allocated_bytes_estimate: null,
                  },
                ],
                logical_bytes_estimate: 21,
                allocated_bytes_estimate: null,
                inventory_complete: false,
              };
            throw new Error(`Unexpected fixture command: ${cmd}`);
          },
        },
      });
    },
    { candidate },
  );

  await page.goto('/');
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .click();
  await page.getByRole('button', { name: 'Analisar computador' }).click();
  await expect(
    page.getByText('Algumas áreas ficaram fora da análise'),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Abrir a lista completa' }).click();
  const candidateBox = page.getByRole('checkbox', {
    name: 'Selecionar Cache parcial de fixture',
  });
  // G1: low-risk eligible items are pre-selected even when an automatic
  // scan skipped other areas.
  await expect(candidateBox).toBeChecked();
  await page.getByRole('button', { name: 'Revisar impacto' }).click();
  await page.getByRole('button', { name: 'Preparar limpeza segura' }).click();
  await expect(page.getByRole('dialog')).toContainText(
    'A análise tem áreas não examinadas. Este plano contém somente os itens selecionados com manifesto completo; nada fora dele será alterado.',
  );
});
