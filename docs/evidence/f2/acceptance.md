# Fase 2 — verificação local (01/11/2026)

Implementação e verificação em Fedora 44, Rust 1.93.0, Node 24.18.0, npm 11.16.0, GTK 3.24.52, WebKitGTK 2.54.2. Base do incremento: `df125f8`; bytes da versão verificada em `source-sha256.txt`, binário de teste em `native-binary-sha256.txt`. Não é evidência de CI remoto, aprovação humana ou produção.

## PASS

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `cargo test --workspace --all-features`: 37 testes aprovados; dois testes de mount ignorados no runner comum e executados explicitamente com PASS em namespace **user/mount privado** sobre bind mounts de fixtures novas. Não houve alteração de mounts do host nem sudo.
- `npm run typecheck`: zero erros/avisos; `npm run lint`; `npm run format:check`; `npm test`: 6 testes; `npm run build`.
- `npm run test:e2e -- --project=chromium`: 11 testes. Demonstração, teclado, contraste/axe, claro/escuro, reflow, erro/vazio/cancelamento, motion reduzido e listas 100/1.000/10.000. O teste novo de limpeza usa **IPC mockado**, verifica somente renderer/contrato e não prova o executor. Mostra relatório parcial/recuperação, bloqueio de controles e delta sem atribuição. Screenshots em `screenshots/`.
- `python3 scripts/cli-fixture-smoke.py`: CLI real com stdin PTY, confirmação correta/recusada, Trash e exclusão permanente. Exclusivamente bytecode de 64 bytes e fontes criados em TempDir, XDG isolados. Sem limpeza de dados existentes. Resultado em `cli-pty.json`.
- `npm run desktop -- build --debug --no-bundle --features fixture-tests` e `INODUNE_NATIVE_STANDALONE=1 python3 scripts/native-cleanup-e2e.py`: aplicativo **Tauri/WebKitGTK real standalone**, sem Vite, em Xvfb isolado. Scanner Rust → revisão → impacto → plano Rust → consentimento exclusivo da fábrica de fixtures → movimento real de um PNG de **68 bytes criado pelo teste** à Lixeira temporária → relatório Rust. Path original ausente, conteúdo da Trash presente, journal/histórico privados presentes; ID adulterado e replay recusados. Evidência em `native/`.
- Inspeção visual das capturas Chromium claro/escuro/estreita e do relatório WebKitGTK escuro: paths/resultados legíveis, estado parcial explícito, nenhuma animação de sucesso antecipado, bytes movidos separados de disponibilidade observada.

## Segurança exercitada

Troca de folha regular e de symlink depois de pin, troca de pai depois de captura, hardlink criado depois do scan, arquivo alterado por descritor aberto, fonte Python removida, arquivo pessoal novo durante a limpeza, erro de permissão, cancelamento entre folhas, armazenamento público/symlinkado, metadados Trash com percent-encoding, raízes/IDs desconhecidos e planos expirados/adulterados. Área de recuperação não vazia é preservada e relatada; cache pai e código não são removidos. Não há shell, recursão de deleção, fallback permanente da Trash nem plano desserializado com autoridade.

## Limites explícitos

O consentimento automático do harness existe só no build debug `fixture-tests`, com candidatos exclusivamente sob TempDir da fábrica e XDG privados. A confirmação GTK **de produção** é implementada no Rust e compilada, mas não foi automatizada nem exercitada por uma pessoa neste teste. Release com a feature é proibido em compile time. Não distribuir esse binário de testes.

Playwright WebKit continua indisponível por libicu74/libjpeg-turbo8 do fallback Ubuntu. O WebKitGTK nativo acima foi realmente executado; os resultados não são intercambiáveis. Ubuntu, Wayland interativo, leitor de tela real, participantes de usabilidade e revisão humana pré-release seguem pendentes. Pacotes ainda são F4.

O protocolo não é uma sandbox contra processos hostis do mesmo UID. Janela residual compare/unlink, falhas de energia/hardware e diferenças de filesystem são descritas em [cleanup-protocol.md](../../cleanup-protocol.md). Journals incompletos são UNKNOWN; não há rollback automático. Trash cross-filesystem é recusada, não copiada. Ganho físico não é atribuído ao app; arquivos abertos e CoW podem reter espaço. Nenhum teste sobre dados reais, nenhuma publicação e nenhum uso de root no aplicativo.

## Ocorrências resolvidas durante a verificação

Fixtures iniciais tinham magic de CPython 3.13 com nome 3.11; o scanner as protegeu corretamente e os fixtures foram corrigidos para A7 0D. O primeiro harness contou também o checkbox desabilitado de Cargo; passou a contar somente controles elegíveis. Outro erro era do wrapper WebDriver, que confundia erro esperado do IPC com erro do protocolo. Lint encontrou um parâmetro catch não usado, corrigido antes dos checks finais. Nenhuma dessas execuções foi registrada como PASS antecipadamente.
