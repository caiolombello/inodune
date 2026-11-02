import { expect, test } from '@playwright/test';

// 0.1.11 QA: the grouped "Dependências Node" row must expand and list each
// node_modules folder. Fixture IPC only.
test('grouped node_modules row expands to list each folder', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const node = (id: string, project: string) => ({
      id,
      title: 'Dependências Node',
      display_path: `/home/user/./${project}/node_modules`,
      detector_id: 'node.modules',
      detector_version: '1',
      scope_id: '/home/user',
      category: 'Desenvolvimento',
      project,
      risk: 'review',
      logical_bytes: 1_048_576,
      allocated_bytes_estimate: 1_048_576,
      identity: { dev: 1, ino: id.length, mtime_unix: 1 },
      coverage_key: `1:${id}`,
      selected_by_default: false,
      reason: 'node_modules recriável',
      impact: 'Recriável',
      rebuild_hint: 'npm install',
      preserved: 'Código preservado',
      evidence: [],
      activity: 'mtime',
      supported_action: 'reviewed_cache_contents',
    });
    Object.assign(window, {
      isTauri: true,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          if (cmd === 'list_scopes') return [];
          if (cmd === 'begin_automatic_scan') return 'job';
          if (cmd === 'poll_scan')
            return {
              job_id: 'job',
              status: 'done',
              progress: { visited: 2, candidates: 2, stage: 'Concluído' },
              error: null,
              result: {
                scan_id: 'scan',
                candidates: [node('a', 'atelie'), node('b', 'farol')],
                issues: [],
                stats: {
                  visited: 2,
                  skipped: 0,
                  elapsed_ms: 1,
                  skip_reasons: {},
                },
                complete: true,
                cancelled: false,
                volumes: [],
              },
            };
          return null;
        },
      },
    });
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador', exact: true })
    .first()
    .click();
  await page.locator('.review-drawer > summary').click();
  const toggle = page.getByRole('button', {
    name: 'Mostrar pastas de Dependências Node',
  });
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await toggle.click();
  const list = page.getByRole('list', { name: 'Pastas de Dependências Node' });
  await expect(list.getByText('/home/user/atelie/node_modules')).toBeVisible();
  await expect(list.getByText('/home/user/farol/node_modules')).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Ocultar pastas de Dependências Node' }),
  ).toHaveAttribute('aria-expanded', 'true');
  await page
    .getByRole('button', { name: 'Ocultar pastas de Dependências Node' })
    .click();
  await expect(list).toHaveCount(0);
});
