# Evidência de runtime dos pacotes Linux

**Resultado:** a instalação limpa do DEB em Ubuntu 24.04.5 e do RPM em Fedora 44 resolveu as dependências e abriu a janela desktop Inodune como UID 1000. O processo continuou ativo depois que a janela apareceu. As capturas mostram a interface renderizada no onboarding inicial.

## Artefatos verificados

Os pacotes correspondem ao build de aplicação do commit `a61e7cc`; hashes conferidos em `target/packages/SHA256SUMS`:

| Distribuição e pacote | SHA-256 | Runtime GTK / WebKitGTK / Xvfb |
| --- | --- | --- |
| Ubuntu 24.04.5, `Inodune_0.1.0_amd64.deb` | `4e0bdc401b7607803abdf92fe95cbf129fd0be206a7a0eb5631cd4e060622f1f` | GTK 3.24.41-4ubuntu1.3; WebKitGTK 2.52.6-0ubuntu0.24.04.1; Xvfb 2:21.1.12-1ubuntu1.9 |
| Fedora 44, `Inodune-0.1.0-1.x86_64.rpm` | `6b84dd106e862977118ae8ce6fa4c5f6ad2923c1794ac283d637cbd15996434e` | GTK 3.24.52-2.fc44; WebKitGTK 2.54.2-1.fc44; Xvfb 21.1.25-2.fc44 |

As imagens base amd64 foram fixadas por digest: `docker.io/library/ubuntu@sha256:78d4744ead1f5b19b04d0df558ab182a5300903900755e515b2ca4d10eb2044d` e `registry.fedoraproject.org/fedora@sha256:7a04a2f9d11bba3ff8016ae79cc01ba49965578746c930702e141a3be1f8d4de`. A evidência JSON em cada diretório inclui o digest e ID da imagem, hash do pacote, versão instalada e versões dos componentes auxiliares.

## Verificações

| Checagem | Ubuntu 24.04 | Fedora 44 |
| --- | --- | --- |
| Instalação do pacote no container descartável (`apt` / `dnf`) | PASS | PASS |
| `ldd` sem bibliotecas GUI não resolvidas | PASS | PASS |
| Janela visível com título `Inodune` | PASS | PASS |
| Processo permaneceu ativo após mostrar a janela | PASS | PASS |
| Captura PNG 1440 × 960 com conteúdo (mais de 100 cores) | [3.100 cores](ubuntu-24.04/inodune-desktop.png) | [2.777 cores](fedora-44/inodune-desktop.png) |
| Versão do pacote | `0.1.0` | `0.1.0-1` |

Evidência de execução: [Ubuntu JSON](ubuntu-24.04/runtime-smoke.json), [Ubuntu log](ubuntu-24.04/desktop.log), [Fedora JSON](fedora-44/runtime-smoke.json), [Fedora log](fedora-44/desktop.log). O harness registra também os logs do Xvfb.

## Isolamento e limites

- Os containers foram construídos e executados com Podman rootless. Nenhum pacote foi instalado no host, nenhuma configuração pessoal foi alterada e nenhum diretório do host foi montado no container.
- A instalação do pacote e dos requisitos ocorreu durante a construção das imagens descartáveis. O processo desktop foi iniciado com UID 1000, `--network=none` e um `HOME`/conjunto XDG temporário e vazio. Nenhuma varredura ou limpeza foi solicitada.
- Para o display sintético Xvfb, o processo do app recebeu `LIBGL_ALWAYS_SOFTWARE=1` e o fallback gráfico de teste `WEBKIT_DISABLE_COMPOSITING_MODE=1`. A variável é específica deste harness e não altera o binário nem o ambiente de usuários. A [documentação do Tauri sobre problemas gráficos Linux](https://v2.tauri.app/develop/debug/linux-graphics/) descreve janelas WebKitGTK em branco e workarounds de composição.
- Esta é uma prova de instalação, carregamento da janela e renderização em X11/Xvfb. Não é validação em uma sessão física Wayland/X11, não exercita a interação com o seletor nativo nem é um teste WebDriver; esse trabalho permanece pendente.

## Tentativas do harness que não foram aprovadas

1. O primeiro harness combinava `xvfb-run` e `dbus-run-session`. O processo ficou parado antes de iniciar Python; a inspeção mostrou somente o shell de `xvfb-run` e Xvfb, sem o processo DBus ou o harness. O harness passou a iniciar e encerrar Xvfb diretamente, sem o wrapper DBus, e então abriu a aplicação.
2. A primeira captura usou a janela raiz do X11 e resultou em uma imagem vazia. Ela não foi aceita como evidência visual. A captura foi alterada para selecionar a própria janela visível `Inodune`, e o harness agora rejeita capturas com menos de 100 cores; as capturas finais acima mostram o onboarding renderizado.
3. O primeiro nome de ferramenta de captura sugerido para Fedora (`xorg-x11-apps`) não existe no repositório Fedora 44. A transação `dnf` falhou com “No match for argument: xorg-x11-apps”; foi substituído pelo pacote Fedora `xwd`, e a instalação/runtime final passou.


A jornada WebDriver com backend real (revisão/execução de fixtures/histórico/exclusões) foi testada separadamente no host Fedora 44 e está em [aceite F4](acceptance.md). Essa evidência não é promovida a interação dos pacotes nos dois containers; aqui o escopo é instalação e abertura renderizada.
