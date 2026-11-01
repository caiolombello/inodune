/** Rótulo de política; nenhuma regra abaixo afirma que o item está em uso. */
export function protectionReason(reason: string): string {
  const text = reason.toLocaleLowerCase('pt-BR');
  if (text.includes('sem lock')) return 'sem lock';
  if (text.includes('acima do limite')) return 'acima do limite';
  if (text.includes('symlink') || text.includes('links')) return 'symlink';
  if (text.includes('credencia')) return 'credenciais';
  if (text.includes('agente') || text.includes('sessão'))
    return 'sessão de agente';
  if (text.includes('git')) return 'repo git';
  if (text.includes('referenciada por lock')) return 'referenciado em lock';
  return reason.length > 64 ? reason.slice(0, 61) + '…' : reason;
}
