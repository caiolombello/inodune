import { describe, expect, it } from 'vitest';
import { formatDate, formatNumber, language, t } from './i18n.svelte';
import { formatBytes } from './model';

describe('i18n', () => {
  it('keeps Portuguese as the legacy default and falls back to source text', () => {
    language.value = 'pt-BR';
    expect(t('Configurações')).toBe('Configurações');
    expect(t('/home/user/project')).toBe('/home/user/project');
    expect(t('unknown backend error')).toBe('unknown backend error');
  });

  it('translates exact display strings and formats values for the selected locale', () => {
    language.value = 'en-US';
    expect(t('Configurações')).toBe('Settings');
    expect(t('ENOTFOUND: /tmp/cache')).toBe('ENOTFOUND: /tmp/cache');
    expect(formatNumber(1234.5)).toBe('1,234.5');
    expect(formatBytes(1536)).toBe('1.5 KiB');
    expect(
      formatDate(new Date('2026-01-02T12:00:00Z'), { timeZone: 'UTC' }),
    ).toContain('2026');
    language.value = 'pt-BR';
  });
});
