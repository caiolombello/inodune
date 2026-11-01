# Fase 3 — verificação local (01/11/2026)

Código integrado até `ef2d88a`; bytes exatos em `source-sha256.txt` e binário nativo de teste em `native-binary-sha256.txt`. Fedora 44, Rust 1.93.0, GTK 3.24.52, WebKitGTK 2.54.2, Git 2.56.0, Bubblewrap 0.12.0. Nenhum teste sobre dados reais ou publicação.

## PASS local

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo test --workspace --all-features`: 73 testes do projeto (core 25, scan 11, clean 22, revisão 7, CLI 8); dois testes de mount ficam ignorados no runner comum e foram executados explicitamente com PASS por `python3 scripts/mount-fixture-tests.py`, somente sobre bind mounts de fixtures em namespace user/mount privado, sem sudo ou mudanças nos mounts do host. O teste filho de thumbnail não é contado duas vezes.
- `npm run typecheck` (zero erros/avisos), `npm run lint`, `npm run format:check`, `npm test` (7 testes), `npm run build` e `npm run test:e2e -- --project=chromium` (11 testes). Capturas e baseline de renderer 100/1.000/10.000 em `screenshots/` e `chromium-renderer-*.json`; IPC mockado nesses benchmarks não mede o scanner nem o compositor nativo.
- `npm run desktop -- build --debug --no-bundle --features fixture-tests` e `INODUNE_NATIVE_STANDALONE=1 python3 scripts/native-reviewed-e2e.py`: UI WebKitGTK/Tauri real **sem Vite**, somente fábrica de fixtures/XDG temporários. Seleção exclusiva de revisão → plano Rust → consentimento de teste restrito à fábrica → 77 bytes de provider efetivamente enviados à Lixeira temporária, estado/backend/lock/módulos preservados → novo scan → somente JSON Cargo de 148 bytes efetivamente removido, hash e demais artefatos preservados. Replay recusado. Relatórios e screenshots reais em `native/`.
- Native window resize para 420 px: sem overflow horizontal do documento; tema claro efetivamente escolhido pelo controle; redução manual de motion ativada e duração computada verificada. Capturas `native/report-narrow.png`, `report-light.png`, `preferences-reduced.png` inspecionadas. Escuro também observado no mesmo fluxo.
- Revisão estática independente dos novos caminhos de plano/executor/hash/scan não confirmou defeito consequencial; não executou testes nem certifica kernel/GTK, publisher de provider, produção ou aprovação humana.

## Cobertura de segurança e integração

Git real em sandbox: principal/locked/detached/dirty/untracked/ignored/sem upstream/tracking/ahead, submódulos protegidos, saída malformada, cancelamento/timeout, sandbox ausente, mountinfo, gitdir externo, bytes não Unicode e tentativa de escapar pelo FD0, include e fsmonitor externo de fixture. Scanner → Git é testado com raiz tanto no pai quanto no próprio repositório e o índice permanece byte/mtime idêntico.

Providers: h1 exato de nomes+conteúdos e versão/endereço do lock, conjunto flat completo, ELF/plataforma, links/arquivos extras/scripts recusados, metadados/witnesses revalidados, lock alterado antes ou entre folhas com resultado parcial sem rollback falso. Cargo: somente JSON diagnóstico do schema conhecido; hash/manifest/binários preservados, cobertura particionada sem dupla contagem. Artefatos opacos continuam protegidos.

Guardas adicionais foram provadas com teste vermelho antes da correção: ancestrais de armazenamento graváveis por terceiros recusados antes da criação; aprovação direta de descendente de `.codex` não reduz proteção; witnesses públicos não tornam caches elegíveis. O prefixo `./` do scanner e raiz exata do Git também têm teste de integração. Git exclui namespaces de agentes da consulta de conteúdo; JSONL de fixture rastreado e sem leitura não é aberto pelo status.

## Limites e decisões

Ação Git remove/prune permanece indisponível: refs locais não provam publicação remota atual, nem ausência de atividade. Módulos Terraform e caches não autenticados/opaque não recebem deleção; Cargo só libera o JSON documentado. Essa é a alternativa conservadora exigida pelo requisito de proteger estados incertos, detalhada em [f3-detector-policy.md](../../f3-detector-policy.md). Não há alegação de limpeza genérica de todo target/`.terraform`.

O diálogo GTK de produção é implementado/compilado, mas o harness usa consentimento exclusivo de build debug/factory, nunca autoridade do renderer. Release proíbe `fixture-tests`. A revisão humana antes de release destrutivo, participantes, Ubuntu/Debian/Wayland interativo, leitor de tela e compositor/hardware representativo seguem pendentes. Nenhuma garantia absoluta contra processos hostis do mesmo UID; ver protocolo F2. Espaço na Lixeira não foi chamado de recuperado; delta observado pode ser negativo.

Durante construção do harness, seletores iniciais atingiram um botão mobile oculto e um passo tentou iniciar novo scan diretamente do relatório; foram corrigidos para os controles visíveis e retorno à revisão. Uma inicialização do WebDriver perdeu conexão, sem causa confirmada; execuções completas posteriores passaram. Não promovemos esses ensaios a PASS nem inferimos estabilidade universal do driver.
