import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import type { Candidate } from '../src/lib/model';
const corpus = JSON.parse(
  readFileSync(
    new URL('../../../tests/fixtures/demo.json', import.meta.url),
    'utf8',
  ),
) as { candidates: Candidate[] };
import { writeFile, mkdir } from 'node:fs/promises';
for (const count of [100, 1000, 10000]) {
  test(`renderer keeps ${count} synthetic IPC candidates bounded and keyboard reachable`, async ({
    page,
  }, info) => {
    const candidates = Array.from({ length: count }, (_, index) => ({
      ...corpus.candidates[2],
      id: 'fixture-' + index,
      title: 'Build de fixture ' + index,
      display_path: '/fixture/project-' + index + '/target',
      project: 'Projeto de benchmark',
      scope_id: 'fixture-scope',
      coverage_key: 'fixture-' + index,
      logical_bytes: index + 1,
    }));
    await page.addInitScript(
      ({ items }) => {
        Object.assign(window, {
          isTauri: true,
          __TAURI_INTERNALS__: {
            invoke: async (cmd: string, args: Record<string, unknown>) => {
              if (cmd === 'list_scopes')
                return [
                  {
                    id: 'fixture-scope',
                    label: 'Fixture IPC',
                    display_path: '/fixture',
                    kind: 'projects',
                  },
                ];
              if (cmd === 'begin_scan') {
                if (JSON.stringify(args.scopeIds) !== '["fixture-scope"]')
                  throw Error('Wrong root IDs');
                return 'fixture-job';
              }
              if (cmd === 'poll_scan')
                return {
                  job_id: 'fixture-job',
                  status: 'done',
                  progress: {
                    visited: items.length,
                    candidates: items.length,
                    stage: 'Fixture',
                  },
                  error: null,
                  result: {
                    scan_id: 'fixture-scan',
                    candidates: items,
                    issues: [],
                    stats: { visited: items.length, skipped: 0, elapsed_ms: 0 },
                    complete: true,
                    cancelled: false,
                    volumes: [],
                  },
                };
              throw Error('Unexpected command ' + cmd);
            },
          },
        });
      },
      { items: candidates },
    );
    await page.goto('/');
    await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
    await expect(
      page.getByRole('button', { name: 'Analisar pastas', exact: true }),
    ).toBeEnabled();
    const start = await page.evaluate(() => performance.now());
    await page
      .getByRole('button', { name: 'Analisar pastas', exact: true })
      .click();
    await page.getByRole('button', { name: 'Revisar candidatos' }).click();
    await expect(
      page.getByRole('heading', { name: 'Revisão de itens', exact: true }),
    ).toBeVisible();
    const visibleRows = await page.locator('.item-row').count();
    const elapsed = (await page.evaluate(() => performance.now())) - start;
    expect(visibleRows).toBeLessThanOrEqual(count === 100 ? 100 : 60);
    if (count > 100) {
      const region = page.getByRole('region', {
        name: 'Lista virtualizada de itens; paginação disponível acima',
      });
      await region.focus();
      await page.keyboard.press('PageDown');
      await expect
        .poll(() => region.evaluate((el) => el.scrollTop))
        .toBeGreaterThan(0);
      await page.getByRole('button', { name: 'Usar paginação' }).click();
      await expect(page.locator('.item-row')).toHaveCount(100);
      await page.getByRole('button', { name: 'Próxima', exact: true }).click();
      await expect(
        page.getByRole('status').filter({ hasText: 'Página 2' }),
      ).toBeVisible();
    }
    await page
      .getByLabel('Buscar item ou caminho')
      .fill('/fixture/project-' + (count - 1) + '/target');
    await expect(page.locator('.item-row')).toHaveCount(1);
    await expect(
      page.getByRole('button', {
        name: 'Build de fixture ' + (count - 1),
        exact: true,
      }),
    ).toBeVisible();
    await mkdir('../../docs/evidence/f3', { recursive: true });
    await writeFile(
      `../../docs/evidence/f3/${info.project.name}-renderer-${count}.json`,
      JSON.stringify(
        {
          count,
          visibleRows,
          scanToReviewMilliseconds: elapsed,
          kind: 'synthetic IPC renderer baseline; not native scanner or FPS benchmark',
        },
        null,
        2,
      ) + '\n',
    );
  });
}
