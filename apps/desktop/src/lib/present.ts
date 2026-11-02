// Apresentação de textos vindos do backend: caminhos, plurais, motivos e erros.
// Nada aqui muda o que é selecionável ou removido.
import { formatNumber, t } from './i18n.svelte';

/** Remove segmentos `.` redundantes (`/home/u/./x` → `/home/u/x`). */
export function displayPath(path: string): string {
  if (!path) return path;
  let out = path.replace(/\/\.(?=\/|$)/g, '');
  out = out.replace(/\/{2,}/g, '/');
  return out === '' ? '/' : out;
}

/** `1 arquivo`, `2 arquivos`, com número localizado. */
export function countLabel(count: number, one: string, many: string): string {
  return `${formatNumber(count)} ${t(count === 1 ? one : many)}`;
}

const skipLabels: Record<string, string> = {
  structural_denied: 'Pastas do sistema sem permissão de leitura',
  structural_entry_error: 'Pastas do sistema indisponíveis',
  structural_stat_failed: 'Pastas do sistema indisponíveis',
  structural_unavailable: 'Pastas do sistema indisponíveis',
  system_protected: 'Áreas do sistema protegidas',
  hardlink: 'Arquivos com hardlinks, preservados',
  symlink: 'Links simbólicos, não seguidos',
  read_error: 'Entradas que não puderam ser lidas',
  inaccessible: 'Pastas sem permissão de acesso',
  policy_excluded: 'Exclusões definidas por você',
  credential_preserved: 'Credenciais preservadas',
  special: 'Arquivos especiais (sockets, dispositivos)',
  owner: 'Itens de outros usuários',
  owner_protected: 'Itens de outros usuários',
  volume: 'Outros volumes ou montagens protegidas',
  volume_protected: 'Outros volumes ou montagens protegidas',
  mount_protected: 'Outros volumes ou montagens protegidas',
  volume_stat_failed: 'Volumes sem medição',
  home_protected: 'Áreas protegidas da pasta pessoal',
  home_filesystem_protected: 'Áreas protegidas da pasta pessoal',
  mountinfo_invalid: 'Tabela de montagens ilegível',
  mount_limit: 'Limite de áreas analisadas atingido',
  root_limit: 'Limite de áreas analisadas atingido',
  nested_mount_exclusion_failed: 'Montagens internas não puderam ser isoladas',
};

/** Frase pt-BR para uma chave interna de exclusão (`system_protected`). */
export function skipReasonLabel(code: string): string {
  return t(skipLabels[code] ?? 'Outros motivos de segurança');
}

/** Agrupa contagens por frase, somando chaves que significam o mesmo. */
export function groupSkipReasons(
  reasons: Record<string, number> | undefined,
): { label: string; count: number }[] {
  const groups = new Map<string, number>();
  for (const [code, count] of Object.entries(reasons ?? {})) {
    if (!count) continue;
    const label = skipReasonLabel(code);
    groups.set(label, (groups.get(label) ?? 0) + count);
  }
  return [...groups.entries()]
    .map(([label, count]) => ({ label, count }))
    .sort((a, b) => b.count - a.count);
}

const errnoLabels: Record<number, string> = {
  1: 'Operação não permitida',
  2: 'O arquivo ou a pasta não existe mais',
  5: 'Erro de leitura no disco',
  13: 'Permissão negada',
  20: 'Um arquivo está onde se esperava uma pasta',
  21: 'Uma pasta está onde se esperava um arquivo',
  36: 'Nome de arquivo longo demais',
  40: 'Links simbólicos em excesso',
  116: 'Arquivo de rede desatualizado',
};

/** Traduz mensagens de sistema (`Not a directory (os error 20)`). */
export function systemMessage(message: string): string {
  const errno = /\(os error (\d+)\)/.exec(message);
  if (errno) {
    const label = errnoLabels[Number(errno[1])];
    if (label) return t(label);
  }
  return t(message);
}

/** Agrupa avisos iguais e mantém poucos caminhos de exemplo. */
export function groupIssues(
  issues: { display_path: string; message: string }[],
  sample = 5,
): { label: string; count: number; paths: string[] }[] {
  const groups = new Map<string, { count: number; paths: string[] }>();
  for (const issue of issues) {
    const label = systemMessage(issue.message);
    const group = groups.get(label) ?? { count: 0, paths: [] };
    group.count += 1;
    if (group.paths.length < sample)
      group.paths.push(displayPath(issue.display_path));
    groups.set(label, group);
  }
  return [...groups.entries()]
    .map(([label, g]) => ({ label, ...g }))
    .sort((a, b) => b.count - a.count);
}

/** Remove repetições como `Cobertura incompleta (cobertura incompleta)`. */
export function tidyReason(reason: string): string {
  return reason
    .replace(
      /Cobertura incompleta \(cobertura incompleta\)/gi,
      'Cobertura incompleta',
    )
    .replace(
      /(\d+) arquivo\(s\)\/diretório\(s\) protegido\(s\) excluído\(s\)/g,
      (_, n) =>
        Number(n) === 1
          ? '1 item protegido fica de fora'
          : `${n} itens protegidos ficam de fora`,
    );
}

/** Avisos do executor: chaves internas viram frases. */
export function warningText(warning: string): string {
  const keyed = /^([a-z_]+): (\d+)$/.exec(warning.trim());
  if (keyed)
    return `${skipReasonLabel(keyed[1])}: ${formatNumber(Number(keyed[2]))}`;
  return systemMessage(tidyReason(warning));
}

const sourceLabels: Record<string, string> = {
  apt: 'APT',
  rpm: 'RPM',
  flatpak_user: 'Flatpak (usuário)',
  flatpak_system: 'Flatpak (sistema)',
  snap: 'Snap',
  steam: 'Steam',
};
export function sourceLabel(source: string): string {
  return t(sourceLabels[source] ?? source);
}

export interface ErrorInfo {
  title: string;
  detail: string;
}
/** Título específico para erros de desinstalação; o texto original fica como detalhe. */
export function uninstallError(raw: string): ErrorInfo {
  const text = raw.replace(/^Error:\s*/, '');
  const lower = text.toLocaleLowerCase('pt-BR');
  const pick = (title: string): ErrorInfo => ({
    title: t(title),
    detail: t(text),
  });
  if (lower.includes('polkit') || lower.includes('autorização negada'))
    return pick('Autorização não concedida');
  if (lower.includes('cancelad')) return pick('Desinstalação cancelada');
  if (lower.includes('dependem'))
    return pick('Outros programas dependem desta seleção');
  if (lower.includes('packagekit indisponível'))
    return pick('PackageKit indisponível');
  if (lower.includes('outro gerenciador'))
    return pick('Gerenciador de pacotes ocupado');
  if (
    lower.includes('bloqueado') ||
    lower.includes('protegido') ||
    lower.includes('essencial')
  )
    return pick('Pacote protegido');
  if (lower.includes('expirad') || lower.includes('mudou'))
    return pick('O plano expirou ou mudou');
  if (lower.includes('desconhecido')) return pick('Resultado desconhecido');
  return pick('Não foi possível desinstalar');
}

/** Único texto da ação de limpeza, derivado da ação do plano/relatório. */
export function actionLabel(
  action: 'trash' | 'permanent' | null | undefined,
): string {
  if (action === 'trash') return t('Lixeira');
  if (action === 'permanent') return t('exclusão permanente');
  return t('ação ainda não confirmada');
}
