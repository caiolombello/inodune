import { expect, test } from '@playwright/test';

test('inventory is opt-in, read-only and exposes partial sources and signal confidence', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const calls: string[] = [];
    Object.assign(window, {
      isTauri: true,
      __appsCalls: calls,
      __TAURI_INTERNALS__: {
        invoke: async (cmd: string) => {
          calls.push(cmd);
          if (cmd === 'list_scopes') return [];
          if (cmd === 'list_installed_apps')
            return {
              sources: [
                {
                  source: 'rpm',
                  status: 'ok',
                  warnings: [],
                  items: [
                    {
                      source: 'rpm',
                      id: 'synthetic-editor',
                      name: 'Synthetic Editor',
                      version: '1',
                      size_bytes: 1024,
                      install_date: null,
                      protected: true,
                      notes: ['Fixture'],
                      signals: [],
                    },
                  ],
                },
                {
                  source: 'flatpak',
                  status: 'partial',
                  warnings: ['Fonte de sistema indisponível'],
                  items: [
                    {
                      source: 'flatpak_user',
                      id: 'org.example.App',
                      name: 'Synthetic Flatpak',
                      version: null,
                      size_bytes: null,
                      install_date: null,
                      protected: false,
                      notes: [],
                      signals: [
                        {
                          kind: 'flatpak_data_directory_mtime',
                          timestamp: 1700000000,
                          confidence: 'low',
                        },
                      ],
                    },
                  ],
                },
                {
                  source: 'snap',
                  status: 'unavailable',
                  warnings: [],
                  items: [],
                },
              ],
            };
          throw new Error(`Unexpected command: ${cmd}`);
        },
      },
    });
  });
  await page.goto('/');
  await page
    .getByRole('button', { name: 'Programas & Jogos', exact: true })
    .click();
  const inventory = page.getByRole('region', {
    name: 'Inventário de programas e jogos',
  });
  await expect(inventory.getByText('Synthetic Editor')).toHaveCount(0);
  await page
    .getByRole('button', { name: 'Listar programas e jogos', exact: true })
    .click();
  await expect(inventory.getByText('Synthetic Editor')).toBeVisible();
  await expect(inventory.getByText('Protegido', { exact: true })).toBeVisible();
  await expect(inventory.getByText(/confiança baixa/)).toBeVisible();
  await expect(inventory.getByText('Último uso desconhecido.')).toBeVisible();
  await expect(
    inventory.getByText('Fonte de sistema indisponível'),
  ).toBeVisible();
  await expect(inventory.getByRole('checkbox')).toHaveCount(0);
  await expect(
    inventory.getByRole('button', { name: /remover|desinstalar|limpar/i }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () => (window as unknown as { __appsCalls: string[] }).__appsCalls,
    ),
  ).toEqual(['list_scopes', 'list_installed_apps']);
  await page.screenshot({
    path: '../../.ui-qa/apps-readonly-wide.png',
    fullPage: true,
  });
  await page.setViewportSize({ width: 720, height: 900 });
  await expect(inventory.getByText('Synthetic Editor')).toBeVisible();
  await page.screenshot({
    path: '../../.ui-qa/apps-readonly-narrow.png',
    fullPage: true,
  });
});

test('web preview does not query real software', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Explorar sem analisar' }).click();
  await page
    .getByRole('button', { name: 'Programas & Jogos', exact: true })
    .click();
  await expect(
    page.getByRole('button', { name: 'Listar programas e jogos', exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText(/Nenhum inventário real é carregado/),
  ).toBeVisible();
});
