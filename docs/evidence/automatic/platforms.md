# Runtime smoke dos pacotes 0.1.1

**Resultado:** os pacotes DEB e RPM finais instalaram em containers limpos Ubuntu 24.04.5 e Fedora 44, e a GUI abriu sob Xvfb como UID 1000. A captura contém a tela inicial do Inodune, incluindo o novo fluxo sem escolha de pastas. O teste não clicou em **Analisar computador** nem ativou uma limpeza.

## Pacotes e runtime

O harness capturou o Git HEAD no início da execução: `39c534f2f3b3bf299c7e8010a775f6c89d3674a6`. Os hashes dos pacotes abaixo também são validados por `target/packages/0.1.1/SHA256SUMS`.

| Plataforma | Pacote e SHA-256 | GTK / WebKitGTK / Xvfb |
| --- | --- | --- |
| Ubuntu 24.04.5 | `Inodune_0.1.1_amd64.deb` — `60f60c952655caa936c6bd4eaac032120c1b54db680461200a880d013b45fc51` | GTK 3.24.41-4ubuntu1.3; WebKitGTK 2.52.6-0ubuntu0.24.04.1; Xvfb 2:21.1.12-1ubuntu1.9 |
| Fedora 44 | `Inodune-0.1.1-1.x86_64.rpm` — `cb3650f75b4615e9d6cec659bc0f176d8b5d840bbacf89ec431c196133098648` | GTK 3.24.52-2.fc44; WebKitGTK 2.54.2-1.fc44; Xvfb 21.1.25-2.fc44 |

O pacote foi instalado com `apt` ou `dnf` dentro da imagem. `ldd` não encontrou bibliotecas gráficas ausentes. A GUI apareceu com o título `Inodune` e permaneceu ativa após abrir.

## Proveniência do HEAD e worktree

O `sourceRevision` completo foi obtido com `git rev-parse HEAD` ao iniciar o harness. O snapshot `sourceInputs`, salvo no [resumo JSON](runtime-smoke-summary.json) e nos JSONs por plataforma, registra:

- `trackedInputStatus: clean` para `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `LICENSE`, `crates/`, `tests/fixtures/`, `assets/brand/`, `package.json`, `package-lock.json`, `apps/desktop/src/`, `apps/desktop/src-tauri/` e `apps/desktop/tests/`.
- `trackedInputDiffSha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`, hash do diff binário vazio nesses caminhos fonte.
- `generatedFrontendDistTreeSha256: e10670cb10fa578f886e0237c8d8c9dc49c938cf40add96ed16d46ca1899dbec` para os arquivos gerados em `apps/desktop/dist/`.
- Hashes dos binários de entrada do empacotador: GUI `7615f5c219645a9f5934a57397313ef6fd013a46540d985be457e2855f753fa6` e CLI `c6c5f1ed17fe5ea0793fc1c8d4cc62e8b903bb51f8a0a9de282688ace1d4fa0e`.

Os hashes dos pacotes são os valores de `target/packages/0.1.1/SHA256SUMS`. Esse checksum local detecta alteração acidental, mas os pacotes não têm assinatura nem proveniência autenticada.

| Evidência | Ubuntu | Fedora |
| --- | --- | --- |
| Screenshot 1440 × 960 | [inodune-desktop.png](ubuntu-24.04/inodune-desktop.png) | [inodune-desktop.png](fedora-44/inodune-desktop.png) |
| Runtime JSON | [runtime-smoke.json](ubuntu-24.04/runtime-smoke.json) | [runtime-smoke.json](fedora-44/runtime-smoke.json) |
| Saída do app | [desktop.log](ubuntu-24.04/desktop.log) | [desktop.log](fedora-44/desktop.log) |

O screenshot mostra “Seu disco ainda não foi analisado”, a ação **Analisar computador**, descoberta local automática e “0 candidatos”. O harness não emitiu clique nem tecla para iniciar análise. Assim, esta evidência confirma a abertura do novo fluxo e a ausência de um seletor de pastas; ela não certifica a descoberta nem a varredura automática em execução.

## Isolamento e limites

- Podman rootless usou imagens base amd64 fixadas por digest: `docker.io/library/ubuntu@sha256:78d4744ead1f5b19b04d0df558ab182a5300903900755e515b2ca4d10eb2044d` e `registry.fedoraproject.org/fedora@sha256:7a04a2f9d11bba3ff8016ae79cc01ba49965578746c930702e141a3be1f8d4de`.
- A instalação aconteceu durante a construção da imagem descartável. A GUI rodou com UID 1000, `--network=none`, um `HOME` temporário e diretórios XDG vazios. Não houve bind mount de diretório com dados do host; o teste não acessou o home real, dispositivos de armazenamento reais, não solicitou análise e não executou limpeza.
- `LIBGL_ALWAYS_SOFTWARE=1` e `WEBKIT_DISABLE_COMPOSITING_MODE=1` foram definidos apenas para o processo dentro do display sintético. Esse fallback de composição segue a [documentação gráfica Linux do Tauri](https://v2.tauri.app/develop/debug/linux-graphics/) e não altera o comportamento padrão do app instalado.
- A evidência cobre somente instalação, resolução de bibliotecas e abertura/renderização X11 sob Xvfb. Sessões reais Wayland/X11, interação com o seletor nativo e WebDriver permanecem pendentes.
