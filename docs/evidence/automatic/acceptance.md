# Análise automática — 0.1.1

O pedido do usuário em 01/11/2026 substituiu a seleção obrigatória de pastas/home restrita pelo fluxo **Analisar computador**. Nenhum teste deste incremento escaneou ou limpou dados pessoais do host.

## Comportamento

A UI desktop abre diretamente no computador local. Um clique inicia a descoberta automática da home, árvores do UID atual e volumes locais montados. Não abre picker nem exige cadastrar projetos. A escolha de pastas continua no modo manual avançado. `/` pode fornecer uso de volume e nomes estruturais, mas nunca vira root autorizada para exclusão. Sistema, rede/FS não confirmado, outros usuários, links e falhas de permissão são protegidos ou ficam sem exame; cobertura parcial é mostrada sem prometer que todos os bytes foram inspecionados.

Exclusões e roots revogadas são preservadas durante a sessão e reaplicadas na redescoberta. Não há execução de limpeza ao abrir ou analisar. Apenas manifestos selecionados e completos de um scan automático não cancelado podem ser revisados mesmo com regiões não examinadas. Snapshot parcial manual continua bloqueado. A confirmação do backend e a UI informam `inventory_complete=false`; cancelamento/manifesto incompleto nunca ganha autoridade.

`scan --auto --json` usa o mesmo core; testes positivos da CLI injetam apenas mounts/roots de fixture em API Rust privada de teste. A CLI `clean` continua explícita e não aceita `--auto`. O frontend nunca fornece o path da home ou dos discos à nova API.

## Verificação

- `cargo fmt --all --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`: PASS.
- `cargo test --workspace --all-features`: **93 PASS**, mais três testes ignorados no runner comum. Os dois mounts foram executados via `scripts/mount-fixture-tests.py` em namespaces privados: **2 PASS**. O teste `discovery-ownership` foi executado por `scripts/test-discovery-ownership.py` dentro de Fedora rootless, somente com diretórios criados no container: **1 PASS**. Não contar duas vezes o subprocesso de thumbnail.
- Ownership: diretório estrutural root-owned → projeto UID1000 encontrado; subtree UID1001 protegida; link não seguido; timestamps do projeto/marker próprios intactos; limites e cancelamento conservadores. A primeira asserção de atime do diretório root-owned falhou: leitura normal do kernel atualizou esse atime. Esse efeito foi documentado, não transformado em ausência de alteração. Não há escrita de conteúdo ou chmod durante descoberta.
- `npm run typecheck` (zero erros/avisos), lint, format:check, **9 unitários**, build e **20 E2E Chromium**: PASS. Incluem início automático sem `choose_scope`, modo avançado opcional, atualização das fontes descobertas e revisão explícita de item completo de análise parcial. WebKit Playwright permanece indisponível no host por libs do seu fallback; não foi chamado de PASS.
- `INODUNE_NATIVE_STANDALONE=1 python3 scripts/native-automatic-e2e.py`: **PASS** em Tauri/WebKitGTK real, sem Vite. Factory debug criou todos os arquivos/XDG; o novo endpoint descobriu a root temporária, reconheceu thumbnails sem scope manual, reportou symlink de fixture como parcial, preparou revisão de manifests completos, moveu 77 bytes de provider para Lixeira temporária e removeu apenas JSON Cargo de 148 bytes. Estado/backend/lock/módulos/hash/artefatos e conteúdo incerto preservados. Histórico, revogação/exclusão, replay recusado, light/narrow/reduced motion verificados. Consentimento automatizado restrito à factory, nunca um teste humano do diálogo de produção.
- `scripts/build-linux.py` e `scripts/package-linux.py`: produção 0.1.1 sem fixtures, compilação rootless offline e extração/validação DEB/RPM PASS, incluindo CLI/licença/launcher. `scripts/test-user-install.py`: instalação somente em prefixo temporário e recusa de sobrescrita PASS. Checksums e grafo SPDX locais em `target/packages/0.1.1/`; artefatos 0.1.0 preservados.
- [Instalação e abertura dos pacotes Ubuntu/Fedora](platforms.md): as provas de startup são separadas da jornada nativa com fixtures, sem clicar em analisar no runtime de produção. Os hashes dos pacotes e dos inputs identificam os bytes verificados; [proveniência de fontes/build](build-provenance.json) registra os inputs finais.

## Correções observadas

A revisão encontrou erros de enumeração e outros proprietários omitidos sem marcação parcial; agora são tratados conservadoramente. Cancelamento e instalação de roots são serializados. O teste nativo inicial revelou que o detector XDG só reconhecia uma root explicitamente marcada thumbnails: a localização XDG confirmada agora também é reconhecida dentro de uma home descoberta. Roots com controles/bytes ambíguos são recusadas, com teste de mount que contém newline. Os testes antigos que verificam o fluxo manual agora escolhem explicitamente o modo avançado e não esperam o onboarding que deixou de ser obrigatório.

## Limites

A enumeração estrutural de diretórios pertencentes ao root pode atualizar atime por ação do kernel quando NOATIME não é permitido; detectores/arquivos próprios mantêm NOATIME. A descoberta não lê conteúdo de arquivos regulares. A travessia é limitada e cooperativamente cancelável; caminhos fora do orçamento são cobertura parcial, nunca ganho inventado. Discos não montados, locais inacessíveis, diretórios do sistema e FS não confirmados não são desbloqueados com root. Exclusões não são persistidas entre sessões nesta versão.

Permanecem as limitações F4: sessão física Wayland/GPUs, leitor de tela, participantes, performance representativa e revisão humana pré-release. Avisos de dependências descritos em `../f4/dependency-audit.md` não foram resolvidos por este incremento. Nenhum release assinado/publicação, teste sobre dados reais, remoção de worktrees ou terraform init foi executado. Nova funcionalidade não é limpeza automática nem garantia contra código hostil com o mesmo UID.
