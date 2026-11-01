import { describe, expect, it } from 'vitest';
import { protectionReason } from './protection';

describe('motivo curto de proteção', () => {
  it.each([
    ['sem lock relevante dentro da raiz Git/análise', 'sem lock'],
    ['Provider acima do limite de 2 GiB por arquivo', 'acima do limite'],
    ['symlink ou arquivo inacessível', 'symlink'],
    ['Repositório Git preservado', 'repo git'],
    ['Estado, locks, credenciais ou instruções não são cache.', 'credenciais'],
    [
      'Localização de ferramenta de agentes: todo conteúdo é protegido.',
      'sessão de agente',
    ],
    [
      'Versão referenciada por lock relevante; uso não detectado',
      'referenciado em lock',
    ],
  ])('resume %s sem afirmar uso', (reason, expected) => {
    expect(protectionReason(reason)).toBe(expected);
  });
  it('mantém um motivo desconhecido visível', () => {
    expect(protectionReason('Política específica da fonte')).toBe(
      'Política específica da fonte',
    );
  });
});
