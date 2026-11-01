import { test, expect } from '@playwright/test';

test('English keyboard journey translates the interface and preserves visual preferences', async ({
  page,
}) => {
  await page.goto('/');
  const explore = page.getByRole('button', { name: 'Explorar sem analisar' });
  await explore.focus();
  await page.keyboard.press('Enter');

  const settings = page
    .getByRole('button', { name: 'Configurações', exact: true })
    .first();
  await settings.focus();
  await page.keyboard.press('Enter');
  const theme = page.getByLabel('Tema', { exact: true });
  await theme.focus();
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  const reduceMotion = page.getByLabel('Reduzir animações', { exact: false });
  await reduceMotion.focus();
  await page.keyboard.press('Space');

  const language = page.getByLabel('Idioma');
  await language.focus();
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');

  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(page.locator('html')).toHaveAttribute('data-motion', 'reduce');
  await page.getByRole('button', { name: 'Explore without analyzing' }).focus();
  await page.keyboard.press('Enter');

  const analyze = page.getByRole('button', {
    name: 'Analyze sample',
    exact: true,
  });
  await analyze.focus();
  await page.keyboard.press('Enter');
  await page
    .getByRole('button', { name: 'Review candidates' })
    .waitFor({ state: 'visible' });
  await expect(
    page.getByRole('button', { name: 'Review candidates' }),
  ).toBeEnabled();
  await page.getByRole('button', { name: 'Review candidates' }).focus();
  await page.keyboard.press('Enter');

  await page.getByRole('button', { name: 'Protected items' }).first().focus();
  await page.keyboard.press('Enter');
  const explain = page.getByRole('button', {
    name: 'Why it appeared: Agent sessions',
  });
  await explain.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog')).toContainText(
    'Conversations may help you resume work.',
  );
  await page.keyboard.press('Escape');

  await page.getByRole('button', { name: 'Projects', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', {
      level: 1,
      name: 'Every project has its own space.',
    }),
  ).toBeVisible();
  await page.getByRole('button', { name: 'Protected items' }).first().focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', { level: 1, name: 'Keep what matters.' }),
  ).toBeVisible();

  await page.getByRole('button', { name: 'Overview', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', {
      level: 1,
      name: 'Your space, in perspective.',
    }),
  ).toBeFocused();
  await expect(
    page.getByRole('button', { name: 'Review candidates' }),
  ).toBeEnabled();
  await page.getByRole('button', { name: 'Review candidates' }).focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', { level: 1, name: 'Review with confidence.' }),
  ).toBeFocused();
  await page.getByRole('button', { name: 'Simulate impact' }).focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', {
      level: 1,
      name: 'Understand before you decide.',
    }),
  ).toBeFocused();
  await page.getByRole('button', { name: 'Review confirmation' }).focus();
  await page.keyboard.press('Enter');
  const consent = page.getByRole('checkbox', {
    name: 'I reviewed the items and understand this is a simulation with no disk changes.',
  });
  await consent.focus();
  await page.keyboard.press('Space');
  await page.getByRole('button', { name: 'Confirm simulation' }).focus();
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', { name: 'Simulation completed.' }),
  ).toBeVisible();
  await expect(page.getByRole('heading', { level: 1 })).toBeFocused();
});
