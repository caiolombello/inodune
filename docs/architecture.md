# Arquitetura — Fase 0

Status: base implementada; aceite nativo pendente conforme evidências da Fase 0. Escopo deste incremento: protótipo sintético navegável. Nenhum acesso a roots reais, scanner ou executor destrutivo.

## Stack e módulos

Rust workspace com `inodune-core` (contratos, política conservadora e validação de planos), `inodune-clean` (dry-run puro, sem FS) e `inodune-cli` (demonstração explícita). `apps/desktop` usa Svelte 5, TypeScript estrito, Vite e CSS tokens. `src-tauri` integra o mesmo core; nenhum plugin FS, shell, opener ou rede. Fonte system-ui local, sem download de fontes. Decisão: [ADR 0001](adr/0001-stack.md).

O frontend é não confiável. Na Fase 0 ele só interpreta um corpus JSON compartilhado com Rust e mantém seleção em memória. Nenhum caminho vira argumento de exclusão. O Tauri não registra comandos invoke de aplicação, tem capability sem permissões e CSP restritiva. Não há persistência de paths, conteúdo, histórico, conta ou telemetria. Preferências visuais só persistem no armazenamento local do renderer.

## Contratos e transição futura

Tipos serializáveis: Detector, Candidate, ApprovedScope, Plan, Risk, ItemOutcome, Report e ScanEvent. Bytes físicos desconhecidos e metadata inexistente usam Option/null, nunca zero como prova. Um plano da Fase 0 tem modo `simulation`, IDs conferidos contra inventário confiável, sem paths fornecidos pela UI. Ainda não é autorização de execução nem plano seguro para FS real.

Fase 1: scanner Rust ancorado em root explícita e dirfd, eventos assíncronos limitados/canceláveis, GUI/CLI usando mesma implementação e DTOs gerados/checados. Fase 2: planos efêmeros internos com expiração, seleção vinculada à confirmação, revalidação contínua e resultados por item. Não estender o dry-run sintético para deleção.

## Verificação

Testes Rust de política e plano, contratos do corpus JSON, testes frontend de seleção/deduplicação, E2E de navegação, cancelamento, consentimento e falhas. Nenhum teste lê HOME como alvo. Ver comandos e limites no README e checklist.

Fontes consultadas em 31/10/2026: [Svelte 5 runes](https://svelte.dev/docs/svelte/$state), [derived](https://svelte.dev/docs/svelte/$derived), [Vite](https://vite.dev/guide/), [Tauri/Vite](https://v2.tauri.app/start/frontend/vite/), [capabilities](https://v2.tauri.app/security/capabilities/), [permissions](https://v2.tauri.app/security/permissions/). Versões resolvidas devem constar nos lockfiles. Nenhuma regra de detector real é declarada validada nesta fase.
