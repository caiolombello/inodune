# Aceite local — Inodune 0.1.2

Escopo: desenvolvimento local, fontes do worktree sobre HEAD 33a0cf0; os hashes de entrada/binários constam em source-inputs.json e na evidência de pacotes. Nenhum teste analisou ou limpou dados pessoais. Operações destrutivas ficaram em fixtures temporárias criadas pelos próprios testes. Não há release publicado ou assinatura.

## Implementado

- ScanExperience com etapa e até quatro descobertas reais, risco visível e inventário provisório. Anel indeterminado, sem porcentagem, estimativa de tempo ou ganho inventado. Pausa, aba oculta e redução manual/OS interrompem o movimento.
- Áudio local opt-in, sem contexto enquanto desligado; espera confirmação do resultado e descarta eventos de segundo plano. Mutar invalida desbloqueio pendente; áudio indisponível não interrompe análise/limpeza. Inter 4.1 é distribuída localmente com licença OFL nas configurações.
- Exclusões, proteções e revogações persistidas antes de alterar a política em memória, com armazenamento privado e schema estrito. Regressão RED reproduziu aceitação de JSON sem campos de proteção; GREEN exige campos obrigatórios. Erros bloqueiam novos scans/limpezas.
- Índice nodeids do pytest com marcadores/schema exatos, revisão individual e orçamento conservador. lastfailed, stepwise e plugins permanecem protegidos. Fixture de 12 mil IDs demonstrou mais de 512 KiB revisáveis; não demonstra ganho representativo em máquinas reais. npm/pip/Next, worktrees e grandes artefatos Rust continuam protegidos.
- Guia de resultados parciais, quarentena e irreversibilidade. O histórico inspeciona journals privados: ausência de encerramento, corrupção, limites ou links aparecem UNKNOWN. Nunca abre caminhos de recuperação registrados, reexecuta planos ou restaura automaticamente. Limites: 4 MiB/journal, 16 MiB total, 10 mil registros, até oito locais exibidos, 4.096 entradas de diretório. Journals maiores exigem inspeção manual.
- Instalador: staging validado antes da publicação, launcher atômico, preservação da versão anterior; uninstall dry-run padrão com --apply separado, manifesto exato e journal de retomada. Alterações/links/arquivos desconhecidos bloqueiam a operação.

## Verificação

- cargo fmt --all --check; cargo clippy --locked --workspace --all-targets --all-features -- -D warnings: PASS.
- cargo test --locked --workspace --all-features: 112 testes de projeto PASS, três testes dependentes do ambiente ignorados no runner comum; não contar o subprocesso de thumbnails duas vezes. Dois testes de mounts foram executados separadamente em namespaces privados e passaram. Ownership em container foi verificado na 0.1.1; não reexecutado neste incremento.
- WebKit Playwright: 24 testes funcionais PASS com MiniBrowser WPE direto e bibliotecas Ubuntu copiadas de imagem rootless para .tools. As duas provas de captura/gravação foram excluídas para preservar os artefatos Chromium. Tentativas anteriores falharam por dependências ausentes e, após provisionamento, pelo cwd incorreto do harness; a execução final usou apps/desktop e não suprimiu asserts. Comando/configuração em webkit-environment.md.
- Frontend: typecheck sem erros/avisos, lint, format:check, 13 testes unitários, build e 26 E2E Chromium PASS. ui_lint --changed: zero erros/avisos; achados de linhas antigas não foram tratados como novos.
- Native WebKitGTK/IPC standalone: PASS (native/result.json), confirmação por harness de fixtures, provider 77 bytes movidos para Lixeira temporária e JSON Cargo removido; fontes/estado/locks/módulos preservados. Histórico, revalidação/replay, exclusões, tema claro, reflow e motion reduzido verificados. Não é teste humano de diálogo destrutivo de produção.
- Instalador: 14 testes lifecycle em TempDirs PASS; smoke do instalador com tarball corrente PASS após empacotamento.
- Capturas scanner: scanner/light-running.png, dark-running.png, dark-paused.png, narrow-paused.png e reduced-running.png. Pixels inspecionados; corrigidos preenchimento indevido do SVG, contraste durante captura de transição e texto abaixo de 12px. Vídeo scanner/fixture-motion.webm usa IPC sintético e mostra a implementação real do renderer; não acessa o disco do usuário.
- Amostra de frames Chromium headless em scanner/frame-sample.json: observação de software, sem certificação de 60 fps em hardware representativo.
- Pacotes: DEB, RPM e tarball locais 0.1.2, checksums e inventários SPDX separados. Instalação e abertura em Ubuntu 24.04.5/Fedora 44 isolados, XDG vazio, UID não root, sem rede/mount de dados do host; resultados em runtime-smoke-summary.json. Xvfb/software rendering não certifica sessão física/GPU/Wayland.

## Revisão e limites

Revisão independente automatizada encontrou campos serde opcionais que relaxavam política e falhas na retomada de instalação/desinstalação. Corrigidos com regressões. O subconjunto pytest recebeu revisão independente e testes direcionados; o leitor adicional de journals recebeu revisão local/testes, sem nova revisão humana. Parecer automatizado não substitui o gate humano.

Ainda abertos: revisão humana de segurança, participantes (3 desenvolvedores/2 pessoas não técnicas), GNOME/KDE/Wayland físicos, leitor de tela real, desempenho representativo, assinatura/identidade de publicação e CI hospedado. Avisos GTK/glib e dependências documentados em release-readiness.md permanecem sem supressão. A ampliação de limpeza continua conservadora: não há promessa de liberar gigabytes em qualquer computador.
