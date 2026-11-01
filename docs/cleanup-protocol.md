# Protocolo de limpeza local — Fase 2

Implementação Linux; revisão humana pré-release continua obrigatória. Nenhuma evidência abaixo certifica resistência absoluta a código hostil do mesmo UID.

## Autoridade e seleção

`PlanSession` existe somente no Rust. Um JSON exportado nunca é autorização. A seleção é limitada a 1.000 candidatos completos `CacheContents`, de baixo risco, provenientes do snapshot atual, com arquivos regulares reconhecidos de thumbnails XDG ou `__pycache__` junto à fonte. Paths não Unicode ficam protegidos nesta versão. Planos têm nonce aleatório de 128 bits, SHA-256 sobre ação, política, scan, paths e fingerprints, TTL de 120 segundos e consumo por ownership. A GUI guarda apenas ID/hash e solicita uma confirmação **nativa** com o resumo resolvido pelo Rust. Após o diálogo, TTL e aprovações são conferidos novamente. CLI exige `--apply` e confirmação textual em terminal; dry-run é o padrão.

## Mutação limitada

1. Reabrir a raiz sem links; comparar dev/inode com a aprovação retida. Validar identidade do cache e seus ancestrais; recusar pastas graváveis por outros usuários. Não corrigir permissões silenciosamente.
2. Criar journal local privado, com intenção durável antes de operações. Falha inicial de journal bloqueia a mutação. Um registro de intenção sem resultado significa **estado desconhecido**, nunca sucesso inferido.
3. Criar área `.inodune-stage-<nonce>` exclusiva 0700 sob a raiz aprovada. Reter descritor; revalidar identidade/localização. Ela nunca entra como candidato de limpeza; inventário posterior mostra recuperação protegida.
4. Para cada folha do manifesto: abrir com `openat2` BENEATH/NO_SYMLINKS/NO_MAGICLINKS/NO_XDEV, conferir todos os metadados, proprietário, tipo e link único. Revalidar assinatura mínima do cache e fonte Python correspondente. Não ler código ou prompts.
5. Registrar origem e caminho de recuperação antes de capturar. `renameat2(NOREPLACE)` usa descritor do **pai já aberto** e basename único, evitando resolver ancestrais durante a mutação. Conferir o objeto capturado contra o descritor original e manifesto. Rename muda ctime; os demais campos devem permanecer iguais, e ambos os descritores devem referenciar o mesmo fingerprint posterior.
6. Divergência: restaurar somente com NOREPLACE e pai ainda correspondente. Se isso não for possível, preservar na área privada e relatar `quarantined` com localização. Nunca sobrescrever o novo arquivo da origem.
7. Só a folha capturada e validada pode ir à Lixeira ou ser desvinculada. Nenhuma exclusão recursiva. As pastas de cache permanecem; arquivos novos/desconhecidos permanecem e tornam o relatório parcial. A área privada só é removida quando vazia.
8. Cancelar entre folhas. Operações já concluídas não são revertidas nem apresentadas como canceladas. Sincronizar diretórios; falhas de durabilidade permanecem explícitas, mesmo se a syscall de mutação já aconteceu.

## Lixeira e medição

A implementação suporta apenas a home Trash (`XDG_DATA_HOME/Trash`) no mesmo filesystem. Cria diretórios novos 0700 e arquivos `.trashinfo` 0600, rejeita symlinks/permissões existentes inseguras, não modifica permissões do usuário. Reserva `.trashinfo` com O_EXCL **antes** do movimento; usa Path absoluto com percent-encoding dos bytes, data local e NOREPLACE. Não copia entre volumes e não faz fallback para exclusão permanente. Não esvazia a Lixeira.

Bytes lógicos processados e estimativa alocada são distintos. `statvfs` mede disponibilidade antes/depois por device; `observed_delta` pode ser negativo e não é atribuído exclusivamente ao Inodune. `attributed_freed_bytes` fica null. Conteúdo movido para a Lixeira continua ocupando espaço. Arquivos abertos, CoW, snapshots e atividade externa afetam o ganho físico; exclusão não equivale a apagamento seguro.

## Limites e recuperação

Linux não oferece uma syscall de comparação de inode com unlink condicional. Entre a última verificação e uma mutação há uma janela residual. A área 0700 reduz interferência de aplicações comuns e outros usuários, mas não isola um processo malicioso do mesmo UID, que pode alterar namespace, descritores abertos e o próprio processo. Não há promessa de rollback, contenção absoluta contra esse adversário, imunidade a falhas de hardware ou recuperação automática após queda. Journals locais preservam intenções/resultados e mapeamento de captura; recuperação desconhecida exige inspeção, nunca limpeza automática.

Histórico contém paths e decisões, não conteúdo. Está em `XDG_STATE_HOME/inodune/reports`, privado, sem upload/telemetria. Limite de retenção e visualização histórica persistente são acompanhados na Fase 4; a Fase 2 mostra o relatório da execução atual e sua localização.

`fixture-tests` é uma feature exclusiva de builds debug. A fábrica cria raízes e todos os XDG em TempDir. Apenas `INODUNE_FIXTURE_CONFIRM=1` **e** candidatos sob esse TempDir permitem consentimento de teste. Builds release com a feature falham na compilação. Binários distribuídos não incluem essa autorização.

## Referências verificadas

- [rustix 1.1.6 renameat_with](https://docs.rs/rustix/1.1.6/rustix/fs/fn.renameat_with.html), [openat2 Linux](https://man7.org/linux/man-pages/man2/openat2.2.html), [renameat2 Linux](https://man7.org/linux/man-pages/man2/rename.2.html).
- [Freedesktop Trash 1.0](https://specifications.freedesktop.org/trash/latest/), seções 5–6; permissões privadas são uma decisão adicional do Inodune.
- [Tauri dialog](https://v2.tauri.app/plugin/dialog/), API Rust instalada `tauri-plugin-dialog 2.8.4`, `blocking_show`, `OkCancel`; diálogo não concede paths vindos da interface.
- [getrandom 0.3.4](https://docs.rs/getrandom/0.3.4/getrandom/fn.fill.html), [sha2 0.10.9](https://docs.rs/sha2/0.10.9/sha2/), [chrono 0.4.45 Local](https://docs.rs/chrono/0.4.45/chrono/struct.Local.html).
