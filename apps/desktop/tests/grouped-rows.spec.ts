import { expect, test, type Page } from '@playwright/test';

// 0.1.12 QA (G-1): with real data the three "Dependências Node" folders
// (different roots and projects) were three separate rows in the lists.
// The fixture copies the format of `inodune-cli scan --json` with three
// roots: real display paths, one scope per root, real sizes. Fixture IPC only.
const node = (
  id: string,
  scope: string,
  project: string,
  bytes: number,
  ino: number,
) => ({
  id,
  title: 'Dependências Node',
  display_path: `${scope}/${project}/node_modules`,
  detector_id: 'node.modules',
  detector_version: 'f3-v1',
  scope_id: scope,
  category: 'Desenvolvimento',
  project,
  risk: 'review',
  logical_bytes: bytes,
  allocated_bytes_estimate: bytes,
  identity: { dev: 54, ino, mtime_unix: 1_793_600_000 },
  coverage_key: `54:${ino}`,
  selected_by_default: false,
  reason:
    'node_modules recriável pelo gerenciador do projeto. Fora da limpeza de 1 clique.',
  impact: 'O próximo install baixa os pacotes de novo.',
  rebuild_hint: 'npm install',
  preserved: 'Manifest, lockfile e código do projeto permanecem.',
  evidence: ['package.json'],
  activity: 'Idade não comprova desuso.',
  supported_action: 'reviewed_cache_contents',
});
const candidates = [
  node('scan-1-0', '/home/user/projects', 'nativeapp', 56_728_371, 900896),
  node('scan-2-1', '/home/user/work', 'webapp', 3_040_870, 900901),
  node('scan-3-2', '/home/user/.cache', 'proj', 1_000_038, 900907),
];

async function boot(page: Page) {
  await page.addInitScript((items) => {
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
              progress: {
                visited: 3,
                candidates: items.length,
                stage: 'Concluído',
              },
              error: null,
              result: {
                scan_id: 'scan',
                mode: 'local',
                candidates: items,
                issues: [],
                stats: {
                  visited: 3,
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
  }, candidates);
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Analisar computador', exact: true })
    .first()
    .click();
}

async function checkGroup(page: Page) {
  const toggle = page.getByRole('button', {
    name: 'Mostrar pastas de Dependências Node',
  });
  await expect(toggle).toHaveCount(1);
  await expect(toggle).toHaveAttribute('aria-expanded', 'false');
  await toggle.click();
  const list = page.getByRole('list', { name: 'Pastas de Dependências Node' });
  for (const item of candidates)
    await expect(list.getByText(item.display_path)).toBeVisible();
  // Each folder has its own selection and its own "Por que…".
  const webapp = list.getByRole('checkbox', {
    name: 'Selecionar /home/user/work/webapp/node_modules',
  });
  await webapp.check();
  await expect(webapp).toBeChecked();
  await expect(
    list.getByRole('checkbox', {
      name: 'Selecionar /home/user/projects/nativeapp/node_modules',
    }),
  ).not.toBeChecked();
  const group = page.getByRole('checkbox', {
    name: 'Selecionar Dependências Node',
  });
  expect(await group.evaluate((n: HTMLInputElement) => n.indeterminate)).toBe(
    true,
  );
  await webapp.uncheck();
  await list
    .getByRole('button', {
      name: 'Por que apareceu: /home/user/.cache/proj/node_modules',
    })
    .click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toContainText('Por que este item apareceu?');
  await expect(dialog).toContainText('/home/user/.cache/proj/node_modules');
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await page
    .getByRole('button', { name: 'Ocultar pastas de Dependências Node' })
    .click();
  await expect(list).toHaveCount(0);
}

test('overview drawer groups node_modules from different roots', async ({
  page,
}) => {
  await boot(page);
  await page.locator('.review-drawer > summary').click();
  await checkGroup(page);
});

test('development list shows one expandable node_modules group', async ({
  page,
}) => {
  await boot(page);
  await page
    .getByRole('button', { name: 'Desenvolvimento', exact: true })
    .first()
    .click();
  await expect(page.locator('.item-row')).toHaveCount(1);
  await expect(page.locator('.item-row')).toContainText('3 pastas');
  await checkGroup(page);
});

test('overview drawer keeps expanded groups and selection after closing', async ({
  page,
}) => {
  await boot(page);
  const summary = page.locator('.review-drawer > summary');
  await summary.click();
  await page
    .getByRole('button', { name: 'Mostrar pastas de Dependências Node' })
    .click();
  const list = page.getByRole('list', { name: 'Pastas de Dependências Node' });
  await expect(list).toBeVisible();
  await summary.click();
  await expect(list).toHaveCount(0);
  await summary.click();
  // The rows are mounted only while open, but the expanded state survives.
  await expect(list).toBeVisible();
});
