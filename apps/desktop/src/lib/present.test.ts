import { describe, expect, it } from 'vitest';
import { language } from './i18n.svelte';
import {
  actionLabel,
  countLabel,
  displayPath,
  groupIssues,
  groupSkipReasons,
  systemMessage,
  tidyReason,
  uninstallError,
  warningText,
} from './present';

describe('present', () => {
  it('removes redundant dot segments from displayed paths', () => {
    expect(displayPath('/home/u/./.cache/pip')).toBe('/home/u/.cache/pip');
    expect(displayPath('/home/u/./a/./b/.')).toBe('/home/u/a/b');
    expect(displayPath('/home/u/.local/x')).toBe('/home/u/.local/x');
    expect(displayPath('/./')).toBe('/');
  });

  it('uses correct singular and plural', () => {
    language.value = 'pt-BR';
    expect(countLabel(1, 'arquivo', 'arquivos')).toBe('1 arquivo');
    expect(countLabel(2, 'arquivo', 'arquivos')).toBe('2 arquivos');
    expect(countLabel(1200, 'item', 'itens')).toBe('1.200 itens');
  });

  it('never shows internal skip keys and merges equivalent ones', () => {
    const groups = groupSkipReasons({
      structural_denied: 3,
      system_protected: 12,
      hardlink: 2,
      volume: 1,
      mount_protected: 2,
      unknown_new_code: 1,
    });
    const text = groups.map((g) => g.label).join(' ');
    expect(text).not.toMatch(/structural_denied|system_protected|hardlink:|_/);
    expect(
      groups.find((g) => g.label.startsWith('Outros volumes'))?.count,
    ).toBe(3);
    expect(groups[0]).toEqual({
      label: 'Áreas do sistema protegidas',
      count: 12,
    });
  });

  it('translates and groups OS errors', () => {
    expect(
      systemMessage(
        'Entrada não examinada: Invalid cross-device link (os error 18)',
      ),
    ).toBe(
      'Entrada em outro sistema de arquivos (ponto de montagem); não examinada',
    );
    expect(systemMessage('Not a directory (os error 20)')).toBe(
      'Um arquivo está onde se esperava uma pasta',
    );
    const issues = Array.from({ length: 23 }, (_, i) => ({
      display_path: `/home/u/./p${i}`,
      message: 'Not a directory (os error 20)',
    }));
    const [group] = groupIssues(issues);
    expect(group.count).toBe(23);
    expect(group.paths).toHaveLength(5);
    expect(group.paths[0]).toBe('/home/u/p0');
  });

  it('removes duplicated coverage wording and keyed warnings', () => {
    expect(
      tidyReason('X. Cobertura incompleta (cobertura incompleta): protegido.'),
    ).toBe('X. Cobertura incompleta: protegido.');
    expect(warningText('structural_denied: 3')).toBe(
      'Pastas do sistema sem permissão de leitura: 3',
    );
    expect(warningText('Cobertura incompleta (cobertura incompleta)')).toBe(
      'Cobertura incompleta',
    );
  });

  it('gives uninstall errors a specific title', () => {
    expect(
      uninstallError(
        'Autorização negada ou cancelada na janela de senha do sistema (polkit). Nenhum pacote foi removido.',
      ).title,
    ).toBe('Autorização não concedida');
    expect(
      uninstallError(
        'Outros pacotes dependem desta seleção, e o Inodune não remove dependentes automaticamente.',
      ).title,
    ).toBe('Outros programas dependem desta seleção');
    expect(uninstallError('PackageKit indisponível. Verifique…').title).toBe(
      'PackageKit indisponível',
    );
    expect(uninstallError('Pacote bloqueado: coreutils').title).toBe(
      'Pacote protegido',
    );
  });

  it('derives the action wording only from the plan action', () => {
    language.value = 'pt-BR';
    expect(actionLabel('trash')).toBe('Lixeira');
    expect(actionLabel('permanent')).toBe('exclusão permanente');
    // Missing data never defaults to "permanent".
    expect(actionLabel(undefined)).not.toContain('permanente');
    expect(actionLabel(null)).not.toContain('Lixeira');
  });

  it('treats a cancellation that removed nothing as information', () => {
    language.value = 'pt-BR';
    const dialog = uninstallError(
      'Desinstalação cancelada. Nada foi removido.',
    );
    expect(dialog.cancelled).toBe(true);
    expect(dialog.title).toBe('Desinstalação cancelada');
    const polkit = uninstallError(
      'Autorização negada ou cancelada na janela de senha do sistema (polkit). Nenhum pacote foi removido.',
    );
    expect(polkit.cancelled).toBe(true);
    // Cancelled mid-transaction without that guarantee stays an error.
    expect(
      uninstallError(
        'A transação foi cancelada antes de terminar. Confira a lista de programas antes de tentar de novo.',
      ).cancelled,
    ).toBe(false);
    expect(uninstallError('Pacote bloqueado: coreutils').cancelled).toBe(false);
  });
});
