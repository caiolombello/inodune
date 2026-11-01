# Checklist de implementação

Fases 0–4 implementadas conservadoramente; código e builds locais verificados, com gates de release e validação externa abertos — 01/11/2026. Os documentos normativos originais são preservados.

## Fase 0

- [x] Workspace Rust, shell Tauri 2 sem invoke/plugins, Svelte/TS, lockfiles e CI básico configurado (CI remoto não executado).
- [x] Contratos de detector/candidato/escopo/plano/risco/resultado/evento.
- [x] Corpus sintético compartilhado, testes sem HOME real.
- [x] Arquitetura, ameaças, política de risco e ADR revisáveis.
- [x] Personas, wireframes, design system, motion e backlog P0 definidos.
- [x] Protótipo onboarding → scan de demonstração → revisão → impacto → confirmação → execução simulada → relatório.
- [x] Explainability, Project Lens, Ledger e Simulator com dados sintéticos.
- [x] Claro/escuro, janela estreita, teclado, erro/vazio/cancelamento/reduced motion verificados.
- [x] cargo fmt/clippy/test dos default-members; frontend typecheck/lint/format/test/build; 7 E2E Chromium, axe e screenshots.
- [x] Build Tauri nativo e smoke visual WebKitGTK: [evidência](evidence/native/acceptance.md).
- [x] E2E Playwright WebKit: 24 testes funcionais na 0.1.2 com bibliotecas locais e MiniBrowser direto; ver evidence/launch/webkit-environment.md. Não confundir com o smoke nativo.
- [x] Abre em Fedora 44 via WebKitGTK em X11/Xvfb isolado; dependências instaladas com autorização expressa posterior. Standalone/pacotes verificados na F4; Wayland interativo pendente.

## Fases seguintes

- [x] F1: scanner read-only de roots aprovadas, fontes oficiais, GUI/CLI compartilhados, contagens/cancelamento/pausa, bytes/deduplicação/limites, testes adversariais e native E2E em fixtures. Git permanece marcador protegido até F3. Ver [evidência](evidence/f1/acceptance.md).
- [x] F2: executor de folhas baixo risco por dirfd/captura privada, planos efêmeros de uso único, confirmação nativa, Lixeira/permanente, journal/histórico privado, variação observada sem atribuição, relatórios parciais e testes adversariais/CLI PTY/WebKitGTK em temporários. [Evidência e limites](evidence/f2/acceptance.md).
- [x] F3: Git em sandbox read-only, providers h1 e JSON Cargo conhecidos com revisão individual, witnesses e cobertura particionada. Estado preservado e 73 testes; [evidência/limites](evidence/f3/acceptance.md).
- [ ] Opcional: remover/prune Git, módulos e formatos opacos permanecem indisponíveis enquanto atividade/publicação/integridade não puderem ser comprovadas; não reduzir a política de proteção para habilitar botões.

## Fase 4

- [x] PT-BR/EN-US e formatos locais, sem alterar paths/IDs/nomes de projeto; preferências de tema/motion/idioma persistidas.
- [x] Histórico persistido privado/versionado, erro por registro, outcomes paginados e exportação local sem substituir arquivos.
- [x] Escopos protegidos explícitos, revogação e exclusões literais antes da leitura; invalidação de snapshots/planos em GUI/CLI.
- [x] Limites de manifesto/journal/histórico e marcadores locais de panic sem payload ou uploads.
- [x] Benchmarks scanner e UI 100/1.000/10.000 com versões, hashes, reflow e teclado; cache somente de apresentação, filesystem sempre relido.
- [x] Release standalone, DEB/RPM/portable com GUI+CLI/launcher, instalador sem root, SPDX/checksums locais e workflow de artefatos sem publicar.
- [x] fmt/clippy e 84 testes Rust; 2 mounts privados; typecheck/lint/format, 9 unitários frontend, build e 17 E2E Chromium. Native WebKitGTK/IPC e produção sem Vite também verificados. [Evidência F4](evidence/f4/acceptance.md).
- [x] Falhas encontradas e avisos de dependências registrados; nenhuma supressão transformada em correção.
- [x] Instalação limpa e GUI renderizada em Ubuntu 24.04.5/Fedora 44 isolados, sem rede, UID 1000 e XDG vazio. [Matriz e hashes](evidence/f4/platforms.md); escopo Xvfb/software rendering.
- [ ] Sessão interativa Wayland/KDE, leitor de tela real e zoom 200%; Debian 12 GUI instalada também não exercitada. Não herdar PASS de outro sistema/harness.
- [ ] 60 fps em hardware representativo e usabilidade com participantes; pendência documentada explicitamente conforme o plano.
- [ ] Assinatura/publicação, CI remoto e revisão humana pré-release. Nenhum release remoto foi publicado.

## Gates pendentes

Usabilidade com 3 desenvolvedores e 2 pessoas não técnicas: não executada; sem participantes. Segurança real de deleção não certificada. Nenhum release/publicação/PR autorizado por este incremento. Não executar testes sobre dados reais. Não promover validações indisponíveis a aprovadas nem chamar este build local de release certificado.

Evidências e comandos exatos: [Fase 0](evidence/phase0.md). Na F0 não houve deleção; na F2 operações reais foram executadas exclusivamente sobre fixtures temporárias criadas pelo teste. Nenhuma limpeza/teste sobre dados reais nem publicação. Provisionamento nativo com sudo foi autorizado expressamente; o app não usa root. 14 registros npm de build/dev (2 low, 12 moderate), sem vulnerabilidade de produção na consulta registrada. Rust 1.93.0 local; GCC no host e builder Debian rootless. [Auditoria atual](evidence/f4/dependency-audit.md) inclui os avisos RustSec GTK3 abertos.


## Incremento automático 0.1.1

- [x] Pedido posterior do usuário substitui seleção manual obrigatória: CTA Analisar computador, sem onboarding/picker obrigatório; modo avançado opcional.
- [x] Descoberta backend de home/árvores próprias/volumes locais montados; `/` não é root destrutiva, outros UIDs/rede/FS incerto/sistema protegidos e cobertura parcial explícita.
- [x] Exclusões/proteções/revogações preservadas na redescoberta; root parcial não é licença para remover conteúdo incerto.
- [x] Manifestos completos selecionados de scan automático parcial podem ser revisados; cancelados/incompletos seguem bloqueados e confirmação nativa exibe a cobertura parcial.
- [x] 93 testes Rust, 3 lanes isoladas adicionais (2 mounts + 1 ownership), 9 unitários frontend e 20 E2E Chromium; novo fluxo native/IPC em fixtures e pacotes 0.1.1 abertos em Ubuntu/Fedora isolados. [Evidências](evidence/automatic/acceptance.md).
- [x] Efeito do kernel sobre atime de diretórios root-owned documentado, sem ocultar o teste inicial que o revelou. Nenhum scan/teste sobre dados pessoais foi executado.


## Incremento de experiência e confiança 0.1.2

- [x] Descobertas reais durante análise, sem porcentagem/tempo inventado, até quatro itens e pausa de movimento.
- [x] Som opt-in, silencioso em segundo plano, sem antecipar confirmação do backend; Inter local e licença.
- [x] Proteções/exclusões/revogações persistidas privadamente; JSON incompleto, links e versões desconhecidas bloqueiam a análise.
- [x] Guia de recuperação por outcome; journals sem encerramento confirmado aparecem como UNKNOWN, sem replay ou restauração automática.
- [x] Índice pytest nodeids verificado com revisão individual e witnesses; estado de falhas/plugins preservado; orçamento de leitura conservador.
- [x] Instalação em staging; atualização por versão; desinstalação simulada por padrão, manifesto verificado e retomada de interrupção.
- [x] Gates adicionais de CI configurados; execução remota não realizada.
- [ ] Revisão humana de segurança, participantes, sessões físicas GNOME/KDE/Wayland e assinatura/publicação permanecem externos.
- Evidência corrente: [aceite 0.1.2](evidence/launch/acceptance.md). Resultados de versões anteriores não aprovam automaticamente este incremento.
