# Arquitetura

Estado atual: Fases 0–4 implementadas conservadoramente. As seções da F0 abaixo são histórico; as evoluções e o protocolo de limpeza definem os boundaries atuais.

## Stack e módulos

Rust workspace com `inodune-core` (contratos, política conservadora e validação de planos), `inodune-clean` (dry-run puro, sem FS) e `inodune-cli` (demonstração explícita). `apps/desktop` usa Svelte 5, TypeScript estrito, Vite e CSS tokens. `src-tauri` integra o mesmo core; nenhum plugin FS, shell, opener ou rede. Fonte system-ui local, sem download de fontes. Decisão: [ADR 0001](adr/0001-stack.md).

O frontend é não confiável. Na Fase 0 ele só interpreta um corpus JSON compartilhado com Rust e mantém seleção em memória. Nenhum caminho vira argumento de exclusão. O Tauri não registra comandos invoke de aplicação, tem capability sem permissões e CSP restritiva. Não há persistência de paths, conteúdo, histórico, conta ou telemetria. Preferências visuais só persistem no armazenamento local do renderer.

## Contratos e transição futura

Tipos serializáveis: Detector, Candidate, ApprovedScope, Plan, Risk, ItemOutcome, Report e ScanEvent. Bytes físicos desconhecidos e metadata inexistente usam Option/null, nunca zero como prova. Um plano da Fase 0 tem modo `simulation`, IDs conferidos contra inventário confiável, sem paths fornecidos pela UI. Ainda não é autorização de execução nem plano seguro para FS real.

Fase 1: scanner Rust ancorado em root explícita e dirfd, eventos assíncronos limitados/canceláveis, GUI/CLI usando mesma implementação e DTOs gerados/checados. Fase 2: planos efêmeros internos com expiração, seleção vinculada à confirmação, revalidação contínua e resultados por item. Não estender o dry-run sintético para deleção.

## Verificação

Testes Rust de política e plano, contratos do corpus JSON, testes frontend de seleção/deduplicação, E2E de navegação, cancelamento, consentimento e falhas. Nenhum teste lê HOME como alvo. Ver comandos e limites no README e checklist.

Fontes consultadas em 31/10/2026: [Svelte 5 runes](https://svelte.dev/docs/svelte/$state), [derived](https://svelte.dev/docs/svelte/$derived), [Vite](https://vite.dev/guide/), [Tauri/Vite](https://v2.tauri.app/start/frontend/vite/), [capabilities](https://v2.tauri.app/security/capabilities/), [permissions](https://v2.tauri.app/security/permissions/). Versões resolvidas devem constar nos lockfiles. Nenhuma regra de detector real é declarada validada nesta fase.

## Evolução F1–F3

O texto da F0 acima preserva as decisões iniciais. O desktop atual tem comandos tipados limitados a scopes por picker, scan, planos opacos e controle/relatório de execução; sem API genérica de filesystem/shell. `scan` usa descritores Linux e agora `git`, `cache_rules`, `terraform` implementam regras conservadoras. `inodune-clean` guarda a autoridade do plano efêmero e opera folhas capturadas em área privada; renderer só recebe views. Histórico/journals privados locais guardam paths e resultados, sem conteúdos ou upload. Scopes/seleção continuam em memória. [Protocolo F2](cleanup-protocol.md) e [regras F3](f3-detector-policy.md) descrevem as fronteiras e riscos residuais atuais.


## Evolução F4

`ApprovedRoot` carrega exclusões relativas imutáveis. O walker as verifica antes de abrir metadados; detectores de contêiner com exclusão interna não concedem limpeza. Git externo é bloqueado conservadoramente em escopos com exclusões. Configuração de escopo, criação de scan/plano e reserva de execução compartilham uma trava curta; nenhum guard fica durante diálogo nativo. Mudar/remover/aprovar novamente uma raiz revoga snapshots e planos. A UI também descarta seleção e resultados.

`inodune-clean::history` lê somente relatórios privados do namespace gerado. JSON persistido é dado de apresentação, nunca `PlanSession`/`AuthorizedPlan`. I/O de histórico/exportação roda fora da thread de UI. Exportação escolhe destino no diálogo Rust, cria 0600 e publica com rename NOREPLACE; não substitui arquivos existentes. Planos limitam folhas/paths; journals/histórico têm limites antes de mutação, sem poda automática. `diagnostics` guarda somente marcadores de panic Rust (versão, timestamp, basename de código e linha), sem payload, dados inspecionados ou uploads. Falhas de subprocessos nativos não estão cobertas pelo hook Rust.

O catálogo explícito PT-BR/EN-US afeta apresentação, não IDs, paths ou nomes de projetos do usuário. Preferências do renderer não concedem acesso a arquivos. Svelte `$derived` memoiza listas/agrupamentos/simulador por dependência; o scanner sempre reconstrói metadados e witnesses. Cache persistente de prova de exclusão não foi habilitado. Essa alternativa sacrifica aceleração entre scans para não reutilizar autoridade obsoleta.

Builds de produção não incluem `fixture-tests`, que falha em release. O builder Debian 12 rootless usa cache Cargo read-only e rede desligada na compilação; o packager copia os binários antes de o Tauri inserir metadados. Pacotes e tarball incluem CLI/GUI e não executam serviços ou scriptlets. SPDX representa os grafos resolvidos, não atestação criptográfica ou análise de alcance de cada dependência.

Referências F4 verificadas em 01/11/2026: [Tauri dialog](https://v2.tauri.app/plugin/dialog/) / fonte instalada 2.8.4; [Tauri DEB](https://v2.tauri.app/distribute/debian/) e [RPM](https://v2.tauri.app/distribute/rpm/); [problemas gráficos/Xvfb](https://v2.tauri.app/develop/debug/linux-graphics/); [Desktop Entry Exec](https://specifications.freedesktop.org/desktop-entry/latest/exec-variables.html) e [TryExec](https://specifications.freedesktop.org/desktop-entry/latest/recognized-keys.html). A política fail-closed permanece quando recursos oficiais não estão disponíveis.
