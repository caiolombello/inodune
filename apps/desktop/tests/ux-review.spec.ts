import { expect, test, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// 0.1.9 UX review: QA items G1, G2, M4 and (d)–(h). Fixture IPC only.
async function mock(page: Page) {
  await page.addInitScript(() => {
    const candidate = (
      id: string,
      title: string,
      category: string,
      bytes: number,
      risk: string,
      action: string,
      reason: string,
      path = `/home/user/./.cache/${id}`,
    ) => ({
      id,
      title,
      display_path: path,
      detector_id: id.startsWith('pyc') ? 'python.pycache' : id,
      detector_version: '1',
      scope_id: '/home/user',
      category,
      project: 'Pasta pessoal',
      risk,
      logical_bytes: bytes,
      allocated_bytes_estimate: bytes,
      identity: { dev: 42, ino: bytes, mtime_unix: 1 },
      coverage_key: `42:${id}`,
      selected_by_default: risk === 'low',
      reason,
      impact: 'Recriável',
      rebuild_hint: 'A ferramenta recria.',
      preserved: 'Código preservado',
      evidence: [],
      activity: 'mtime',
      supported_action: action,
    });
    const candidates = [
      ...Array.from({ length: 12 }, (_, i) =>
        candidate(
          `pyc-${i}`,
          'Bytecode Python',
          'Desenvolvimento',
          2_000_000,
          'low',
          'cache_contents',
          'Bytecode',
          `/home/user/projetos/./pkg${i}/__pycache__`,
        ),
      ),
      candidate(
        'pip',
        'Cache pip',
        'Desenvolvimento',
        2_000_000,
        'review',
        'reviewed_cache_contents',
        'Cache pip',
      ),
      candidate(
        'cred',
        'Cache com credencial',
        'Sistema',
        0,
        'protected',
        'none',
        'Cache de ferramenta. Cobertura incompleta (cobertura incompleta): protegido.',
      ),
    ];
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
              progress: { visited: 10, candidates: 14, stage: 'Concluído' },
              error: null,
              result: {
                scan_id: 'scan',
                candidates,
                issues: Array.from({ length: 21 }, (_, i) => ({
                  display_path: `/home/user/./x${i}/file`,
                  code: 'read_error',
                  message: 'Not a directory (os error 20)',
                })),
                stats: {
                  visited: 10,
                  skipped: 36,
                  elapsed_ms: 1,
                  skip_reasons: {
                    structural_denied: 3,
                    system_protected: 12,
                    read_error: 21,
                  },
                },
                complete: false,
                cancelled: false,
                volumes: [
                  {
                    scope_path: '/home/user',
                    device: 42,
                    total_bytes: 10 ** 12,
                    available_bytes: 4 * 10 ** 11,
                  },
                ],
              },
            };
          if (cmd === 'list_installed_apps')
            return {
              sources: [
                {
                  source: 'apt',
                  status: 'ok',
                  warnings: [],
                  items: [
                    {
                      source: 'apt',
                      id: 'aisleriot',
                      name: 'aisleriot',
                      version: '1:3.22.35-5',
                      size_bytes: 9_437_184,
                      install_date: null,
                      protected: false,
                      notes: [],
                      signals: [],
                    },
                    {
                      source: 'apt',
                      id: 'coreutils',
                      name: 'coreutils',
                      version: '9.5-1',
                      size_bytes: 18_000_000,
                      install_date: null,
                      protected: true,
                      notes: [
                        'Pacote essencial do sistema; desinstalação bloqueada.',
                      ],
                      signals: [],
                    },
                    ...Array.from({ length: 30 }, (_, i) => ({
                      source: 'apt',
                      id: `vendor-tool-${i}`,
                      name: `vendor-tool-${i}`,
                      version: '1',
                      size_bytes: 1000 + i,
                      install_date: null,
                      protected: false,
                      notes: [],
                      signals: [],
                    })),
                  ],
                },
              ],
            };
          if (cmd === 'uninstall_prepare')
            return {
              id: 'uplan',
              expires_in_seconds: 120,
              steam_handoff: false,
              items: [
                {
                  id: 'aisleriot;1:3.22.35-5;amd64;manual:ubuntu-resolute-universe',
                  version: '1:3.22.35-5',
                },
                {
                  id: 'aisleriot-data;1:3.22.35-5;all;auto:ubuntu-resolute-universe',
                  version: '1:3.22.35-5',
                },
              ],
              details: [
                {
                  id: 'aisleriot;1:3.22.35-5;amd64;manual:ubuntu-resolute-universe',
                  name: 'aisleriot',
                  version: '1:3.22.35-5',
                  arch: 'amd64',
                  size_bytes: 9_437_184,
                  requested: true,
                },
                {
                  id: 'aisleriot-data;1:3.22.35-5;all;auto:ubuntu-resolute-universe',
                  name: 'aisleriot-data',
                  version: '1:3.22.35-5',
                  arch: 'all',
                  size_bytes: 21_200_000,
                  requested: false,
                },
              ],
            };
          if (cmd === 'uninstall_execute')
            throw 'Autorização negada ou cancelada na janela de senha do sistema (polkit). Nenhum pacote foi removido.';
          return null;
        },
      },
    });
  });
  await page.goto('/');
}

test('overview pre-selects low risk, explains coverage in Portuguese and hides internals', async ({
  page,
}) => {
  await mock(page);
  await page
    .getByRole('button', { name: 'Analisar computador', exact: true })
    .first()
    .click();
  // G1: 12 low-risk items pre-selected; the review item is not.
  const clean = page.getByRole('button', { name: /^Limpar 12 itens · / });
  await expect(clean).toBeEnabled();
  await expect(clean).toContainText('22,9 MiB');
  // (d): no internal keys, no raw OS errors, no "/./".
  const main = page.locator('main');
  await expect(
    page.getByText('Algumas áreas ficaram fora da análise'),
  ).toBeVisible();
  await page.getByText(/Ver caminhos afetados/).click();
  const text = await main.innerText();
  expect(text).not.toMatch(
    /structural_denied|system_protected|read_error|os error|\/\.\//,
  );
  await expect(
    page.getByText('Um arquivo está onde se esperava uma pasta · 21'),
  ).toBeVisible();
  // (g): grouped rows, plural and no duplicated coverage wording.
  await page.locator('.review-drawer > summary').click();
  const review = page.locator('.review-drawer');
  await expect(
    review.getByRole('checkbox', { name: /Bytecode Python/ }),
  ).toBeChecked();
  await expect(review.getByText('12 pastas')).toBeVisible();
  await expect(
    review.getByRole('checkbox', { name: /Cache pip/ }),
  ).not.toBeChecked();
  expect(await review.innerText()).not.toContain(
    'cobertura incompleta (cobertura incompleta)',
  );
  await expect(
    review.getByText(/Cobertura incompleta: protegido/),
  ).toBeVisible();
  // (g): protected labels are in Portuguese; English names only in en-US.
  await page
    .locator('aside')
    .getByRole('button', { name: /^Itens protegidos/ })
    .click();
  await expect(page.getByText('REGISTRO DE PROTEÇÕES')).toBeVisible();
  // (d): coverage banner stays on the overview, not on every page.
  await expect(
    page.getByText('Algumas áreas ficaram fora da análise'),
  ).toHaveCount(0);
  await page
    .getByRole('button', { name: 'Configurações', exact: true })
    .last()
    .click();
  await expect(
    page.getByText('Algumas áreas ficaram fora da análise'),
  ).toHaveCount(0);
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test('apps: search, filters, counter and a readable uninstall dialog', async ({
  page,
}) => {
  await mock(page);
  await page
    .getByRole('button', { name: 'Programas & Jogos', exact: true })
    .click();
  await page
    .getByRole('button', { name: 'Listar programas e jogos', exact: true })
    .click();
  // (h)
  await expect(page.getByText('32 de 32 programas')).toBeVisible();
  await page.getByLabel('Buscar programa').fill('aisle');
  await expect(page.getByText('1 de 32 programas')).toBeVisible();
  await page.getByLabel('Buscar programa').fill('');
  await page.getByLabel('Filtrar por proteção').selectOption('protected');
  await expect(page.getByText('1 de 32 programas')).toBeVisible();
  // (e): protection reason visible without opening notes.
  await expect(
    page.getByText('Pacote essencial do sistema; desinstalação bloqueada.'),
  ).toBeVisible();
  await page.getByLabel('Filtrar por proteção').selectOption('all');
  await page.getByLabel('Ordenar programas').selectOption('name');
  // M4: open the dialog from a row near the end of the list.
  await page.getByLabel('Buscar programa').fill('aisleriot');
  await page.getByRole('button', { name: 'Desinstalar aisleriot' }).click();
  const dialog = page.getByRole('dialog', { name: 'Confirmar desinstalação' });
  await expect(dialog).toBeInViewport();
  expect(
    await page.evaluate(() =>
      Boolean(document.activeElement?.closest('dialog[open]')),
    ),
  ).toBe(true);
  // G2: friendly name, version, dependency, count and size.
  await expect(dialog).toContainText('1 programa + 1 dependência');
  await expect(dialog).toContainText('2 pacotes · 29,2 MiB instalados');
  await expect(dialog).toContainText('aisleriot-data');
  await expect(dialog.getByText('1:3.22.35-5').first()).toBeVisible();
  await expect(
    dialog.getByText(
      'aisleriot;1:3.22.35-5;amd64;manual:ubuntu-resolute-universe',
    ),
  ).toBeHidden();
  // (f): specific, alert-styled error.
  await dialog.getByRole('button', { name: 'Desinstalar 2 pacotes' }).click();
  const alert = page.getByRole('alert');
  await expect(alert).toContainText('Autorização não concedida');
  await expect(alert).toContainText('polkit');
  await expect(page.locator('.notice.error')).toBeVisible();
});

test('English locale translates the new labels', async ({ page }) => {
  await mock(page);
  await page.evaluate(() => localStorage.setItem('inodune-locale', 'en-US'));
  await page.reload();
  await page
    .getByRole('button', { name: 'Analyze computer', exact: true })
    .first()
    .click();
  await expect(page.getByText('Some areas were not analyzed')).toBeVisible();
  await expect(
    page.getByRole('button', { name: /^Clean 12 items · / }),
  ).toBeVisible();
});
