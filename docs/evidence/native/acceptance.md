# Complemento do aceite nativo — Fase 0

01/11/2026 (UTC), em Fedora 44. Autorização de instalação: mensagem do usuário após a entrega 3c9f77d. Dependências instaladas via DNF após prévia de transação, sem remoções; senha consumida somente pelo stdin do sudo e arquivo privado temporário descartado. Não foi concedida elevação ao Inodune.

- GCC 16.2.1, GTK3 3.24.52, WebKitGTK 2.54.2 e DBus 1.16.2 verificados via executável/pkg-config.
- `cargo build --locked -p inodune-desktop` passou, com Rust 1.93.0 e GCC do sistema, sem linker Zig.
- `scripts/native-smoke.py` abriu o binário Tauri em Xvfb próprio e com XDG_CONFIG/CACHE/DATA/STATE isolados em temporário. O processo permaneceu vivo e [desktop.png](desktop.png) foi inspecionado: onboarding e dashboard foram renderizados no WebKitGTK real. O processo e o X server iniciados pelo teste foram encerrados pelo próprio harness.
- Shell debug usa devUrl; o conteúdo mostrado veio do build de produção servido via `npm run preview -w @inodune/desktop`. Isso ainda não prova standalone/offline ou pacote instalado, gates reservados à distribuição.
- Avisos EGL/DRI3 do Xvfb estão em [app.log](app.log); não houve crash. Não usar este ambiente sem aceleração para alegar 60fps.
- Não houve scan de dados reais, exclusão, Git prune/remove ou Terraform init. E2E WebKit do Playwright continua separado e indisponível no fallback Ubuntu; a abertura nativa foi efetivamente verificada em X11 virtual, não em uma sessão Wayland interativa.

Com esta abertura nativa verificada, a implementação pode avançar à Fase 1; os gates de distribuição e usabilidade permanecem explícitos.
