# Prontidão para release

Snapshot de evidências consultadas em 01/11/2026 para a versão local 0.1.2. Esta matriz orienta um release de desktop destrutivo; ela não certifica segurança nem substitui CI remoto, revisão independente ou teste humano. `PASS` descreve apenas o escopo identificado; `ABERTO` requer ação antes do beta/release público; `NÃO EXECUTADO` significa que não há evidência.

## Estado no snapshot

| Gate | Estado | Evidência / próximo passo |
| --- | --- | --- |
| Builds e testes locais isolados | PASS, no escopo citado | Testes Rust, Chromium, fixtures nativas, builds e pacotes estão registrados em [aceite automático](evidence/automatic/acceptance.md). As mutações dos testes usaram fixtures temporárias. |
| CI para PR/push | CONFIGURADO; execução remota NÃO EXECUTADA | `.github/workflows/ci.yml` executa typecheck, lint, formatos, unitários, build, Chromium/WebKit Playwright e gates Rust/CLI; build nativo Tauri é um job separado. Verifique conclusões dos jobs na hospedagem quando houver execução; o YAML local não comprova CI verde. |
| Testes Rust do backend desktop sob `fixture-tests` | CONFIGURADO; execução remota NÃO EXECUTADA | CI agora pede os testes unitários de `inodune-desktop` com `fixture-tests` e um teste da guarda que impede compilar essa feature em release. O runner ainda precisa executar com sucesso; o estado do YAML local não conta como resultado. |
| Distribuições | PASS limitado a Ubuntu 24.04.5 e Fedora 44 em Xvfb | Instalação e janela renderizada foram verificadas com usuário não-root, rede desativada e XDG vazio. Isso não validou análise, limpeza, aceleração gráfica nem sessão real. Debian 12 teve build e pacote inspecionado, sem abrir a GUI instalada. Ver [matriz de plataforma](evidence/automatic/platforms.md). |
| X11/Wayland e ambientes de desktop | ABERTO | Executar jornadas interativas do aplicativo final em GNOME e KDE, X11 e Wayland, com os pacotes suportados. Registrar falhas de permissões, janela, diálogo nativo, acessibilidade e fechamento/retomada. Xvfb e software rendering não substituem essas sessões. |
| Acessibilidade e experiência de primeira análise | ABERTO | Verificar teclado/foco, leitor de tela real, zoom a 200%, reflow, tema claro/escuro, movimento normal/reduzido e controles de som. Observar participantes (3 desenvolvedores e 2 pessoas não técnicas) e medir compreensão do que será alterado antes da confirmação. Não afirmar teste sem participantes. |
| Integridade dos pacotes | ABERTO | Builds locais têm checksums SHA-256 e inventários SPDX sem assinatura/autenticação de origem. Definir identidade de publicação e assinatura verificável separadamente para DEB, RPM e tarball; testar verificação usando chave pública divulgada por canal autenticado. Nenhuma chave ou artefato foi publicado neste snapshot. |
| Instalação, atualização e remoção por usuário | ABERTO até o ciclo completo ter testes verdes | CI agora executará `scripts/test-user-lifecycle.py` para primeira instalação, atualização e remoção em prefixos temporários. Confira o resultado local e remoto e acrescente recuperação de atualização interrompida. A remoção só pode atingir arquivos de manifesto íntegro pertencentes ao Inodune; arquivos editados, desconhecidos, links e preferências do usuário precisam sobreviver. Isso não autoriza executar o instalador no prefixo real durante o desenvolvimento. |
| Utilidade do scanner e caches | ABERTO | Demonstrar cobertura e potencial recuperável em homes sintéticas representativas e grandes; medir tempo, memória, itens protegidos, cancelamento e espaço observado após operações apenas em fixtures. Não tratar total do volume como quantidade examinada ou recuperável. Atualizar o suporte a formatos só com documentação vigente e fixtures adversariais. |
| Segurança da deleção | ABERTO; revisão humana pré-release obrigatória | Obter uma revisão humana independente do código de política, preparação do plano, persistência, captura, revalidação, recuperação e relatório parcial. Testes automatizados e revisão de agentes não satisfazem esse gate. Revisar os limites documentados contra processos hostis do mesmo UID e falhas após queda. |
| Avisos de dependências Rust | ABERTO para decisão de risco e reauditoria | O lockfile atual resolve `glib 0.18.5` no caminho Linux do app. A advisory RustSec indica a correção em glib `>=0.20.0`, enquanto a cadeia GTK 3 do app exige `glib ^0.18`. A tentativa upstream de backport à série 0.18 foi fechada sem merge; a issue de release 0.18.6 foi fechada, mas a versão não apareceu no índice oficial consultado em 01/11/2026. Não suprima o finding nem conclua “corrigido”. Antes de cada release, consulte a advisory, o índice/versão realmente publicada, o lockfile e a árvore Linux e escolha uma resolução documentada e testada. Veja [`RUSTSEC-2024-0429`](https://rustsec.org/advisories/RUSTSEC-2024-0429.html), [PR de backport #2009](https://github.com/gtk-rs/gtk-rs-core/pull/2009), [decisão de release #2010](https://github.com/gtk-rs/gtk-rs-core/issues/2010) e [índice oficial de crates](https://index.crates.io/gl/ib/glib). |
| Avisos npm e RustSec adicionais | ABERTO para reauditoria antes do release | A [auditoria local F4](evidence/f4/dependency-audit.md) registrou avisos de desenvolvimento npm e `proc-macro-error` sem manutenção. A consulta de vulnerabilidades da database RustSec tem data; repita ambas para a revisão de release e investigue as mudanças sem downgrade forçado ou supressão silenciosa. |
| Release e apoio | NÃO EXECUTADOS | Definir canal de downloads, verificação de artefatos, notas de limitações, canal para reportar falhas e procedimento de retirada de versão vulnerável. O workflow manual atual gera artefatos; não publica release. |

## Revisão atual da cadeia GTK/glib

No snapshot, `cargo tree --locked -p inodune-desktop -i glib --target x86_64-unknown-linux-gnu` mostrou `tauri 2.12.5 → gtk 0.18.2 / webkit2gtk 2.0.2 → glib 0.18.5`. A mesma cadeia deve ser consultada para cada release, porque uma atualização do lockfile altera o escopo.

A advisory descreve unsoundness na implementação de `Iterator`/`DoubleEndedIterator` de `VariantStrIter` e lista `glib >=0.20.0` como patched. O GTK 3 usado nesta cadeia depende da linha `glib ^0.18`; não é seguro forçar outro pacote `glib` ou famílias duplicadas só para remover um alerta. A investigação de 01/11 confirmou que a tentativa PR #2009 está fechada com `merged: false`, issue #2010 foi fechada em 31/07, e o índice crates.io ainda lista somente `glib 0.18.0`–`0.18.5`. Portanto, a antiga hipótese de um backport 0.18.6 disponível não está confirmada. Isso não demonstra alcance da função na aplicação; também não demonstra que o risco inexiste. O finding segue aberto para revisão humana e nova consulta imediatamente antes de release.

Comandos de verificação local que não escaneiam dados do usuário:

```sh
cargo tree --locked -p inodune-desktop -i glib --target x86_64-unknown-linux-gnu
cargo audit
npm audit --omit=dev
```

Use a versão instalada/atualizada da database de auditoria e registre seu identificador/data. Não converta resultados locais em resultados do CI hospedado nem em aprovação de risco.

## Condições para declarar um release público

Um release público do limpador só deve ser preparado quando todos os gates `ABERTO` acima tiverem evidência revisável ou uma decisão humana explícita de escopo/risco registrada. O pacote candidato precisa corresponder ao commit e aos inputs revisados, e passar a verificação pós-assinatura em instalação limpa. A aprovação de release, assinatura/publicação e testes com participantes são etapas separadas; esta matriz não as executa.

Permanece obrigatório manter `Protection Ledger`, explicabilidade por item, estimativas separadas de mudança real do filesystem, relatório de falhas/pulados, consentimento por ação destrutiva, movimento reduzido e som desligável. Nenhuma animação ou sinal sonoro pode antecipar uma resposta do backend nem declarar sucesso sem medição.


Atualização 0.1.2: [aceite corrente](evidence/launch/acceptance.md) registra scanner visual/áudio opt-in, proteções persistentes, leitor de journals UNKNOWN e ciclo testado do instalador. Instalação/atualização/remoção têm testes locais verdes; o gate remoto continua pendente. A revisão humana e os testes de usabilidade não foram realizados. A cobertura de limpeza ampliou apenas o índice conhecido de pytest; a utilidade em ambientes reais permanece não medida.
