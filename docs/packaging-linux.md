# Pacotes Linux

O Inodune gera `.deb` e `.rpm` x86_64 com Tauri 2. Os pacotes incluem o app desktop, o comando `inodune-cli`, ícone, licença e atalho de aplicativos. O app opera com o UID da pessoa que o iniciou; não inclui serviço privilegiado, script de instalação/remoção, conta, telemetria ou rede obrigatória.

## Compilar e empacotar

Depois de `npm ci`, build do frontend e build de release compatível:

```sh
npm run build
python3 scripts/build-linux.py
python3 scripts/package-linux.py
```

`build-linux.py` compila em container Debian 12 rootless e sem rede, usando a imagem builder local e cache de dependências somente para leitura. `package-linux.py` usa o bundler oficial do Tauri em outro container rootless, também sem rede. O bundler recebe cópias temporárias dos binários, porque ele anota o executável com metadados do formato do pacote. Ele não altera os binários de release no checkout.

As dependências de runtime são verificadas pelo próprio bundler: Debian declara `libwebkit2gtk-4.1-0` e `libgtk-3-0`; RPM declara as capacidades `libwebkit2gtk-4.1.so.0()(64bit)` e `libgtk-3.so.0()(64bit)`. Não há scriptlets de instalação, atualização ou remoção. `target/packages/0.1.2/` contém os dois pacotes, um tarball para instalação no usuário, inventários SPDX 2.3 separados para Rust e frontend, e `SHA256SUMS` local.

Para uma primeira preparação, o build dos containers precisa de acesso aos repositórios oficiais de pacotes. A geração dos pacotes em si roda sem rede. O build não configura o Git global e não usa assinatura. `SHA256SUMS` ajuda a detectar corrupção local; sem assinatura ou publicação autenticada, não comprova autoria ou origem.

## Instalação

Instale o pacote com o gerenciador da distribuição, que pode exigir autorização administrativa para gravar em `/usr`:

```sh
sudo apt install ./target/packages/0.1.2/Inodune_0.1.2_amd64.deb
sudo dnf install ./target/packages/0.1.2/Inodune-0.1.2-1.x86_64.rpm
```

A necessidade de privilégio é do gerenciador de pacotes ao instalar em caminhos de sistema. O executável não pede root e não tenta elevar privilégios.

Para instalar sem root, mantenha `install-user.py` ao lado do tarball e execute o instalador com um prefixo absoluto gravável pelo usuário:

```sh
python3 target/packages/0.1.2/install-user.py \
  --archive target/packages/0.1.2/Inodune_0.1.2_linux_x86_64.tar.gz \
  --prefix "$HOME/.local"
```

O instalador lê somente os membros regulares esperados, em tamanho limitado, do tarball e cria `lib/inodune/<versão>/` com GUI, CLI, licença, ícone e um manifesto privado de hashes. Deve ser executado pelo usuário desktop e recusa execução como root. Os arquivos são escritos em um diretório de staging exclusivo, validados pelo manifesto e publicados sem substituir uma versão existente; falha antes da publicação remove somente os arquivos e diretórios que esta execução criou e cuja identidade ainda confere. A publicação usa `renameat2(RENAME_NOREPLACE)` com descritores de diretório; se libc/kernel/filesystem não oferecerem essa operação, a instalação falha de forma fechada. `share/applications/inodune.desktop` só é atualizado depois que a árvore publicada foi validada, e uma repetição da mesma versão e conteúdo completa a troca do launcher com segurança. A versão anterior permanece instalada até ser removida explicitamente. O CLI fica em `~/.local/lib/inodune/<versão>/bin/inodune-cli`.

Para remover uma versão instalada pelo instalador novo, informe o prefixo e a versão. O primeiro comando só mostra o que pretende fazer; `--apply` aplica a remoção:

```sh
python3 scripts/install-user.py --prefix "$HOME/.local" --uninstall 0.1.2
python3 scripts/install-user.py --prefix "$HOME/.local" --uninstall 0.1.2 --apply
```

Em uma versão empacotada, use o `install-user.py` entregue junto do tarball. Os comandos acima usam o checkout de desenvolvimento.

Ao criar novos artefatos, `package-linux.py` copia esta versão do instalador. Os arquivos já existentes em `target/packages/` são saídas locais antigas e não são atualizados por esta alteração; reempacote antes de prometer o novo ciclo de atualização/remoção em um download.

O instalador valida o manifesto, conteúdo, permissões, propriedade, links e todos os arquivos da árvore antes de agir. Diante de arquivos modificados, desconhecidos, symlinks, hardlinks, manifesto ilegível ou launcher alterado, recusa a remoção e preserva o conteúdo para inspeção manual. Ao iniciar a remoção, grava um diário privado com o manifesto verificado; se houver interrupção, `--uninstall VERSÃO --apply` pode ser repetido para retomar os arquivos restantes, validando hashes e identidade antes de cada remoção. Se a última versão ativa não tiver fallback, o launcher permanece até o executável da GUI ser removido. Uma falha posterior ao unlink pode deixar estado parcial; o relatório mostra erro e instrui a repetição, sem prometer rollback atômico das remoções já concluídas. Configuração, histórico e dados do usuário vivem fora da árvore e nunca são removidos. Instalações feitas pelo instalador anterior (que não gravava manifesto) não podem ser removidas automaticamente; seus launchers versionados antigos também não são editados.

Esse manifesto local detecta alterações acidentais nos arquivos instalados; não é uma assinatura do fornecedor. Os pacotes e tarballs seguem sem assinatura autenticada: um checksum local verifica integridade em relação ao próprio arquivo de checksums, mas não sua origem. Para uma distribuição pública confiável ainda é preciso estabelecer e validar a identidade de publicação e a assinatura dos artefatos. A publicação usa `renameat2(RENAME_NOREPLACE)` conforme a [documentação Linux de `rename(2)`](https://man7.org/linux/man-pages/man2/rename.2.html); a atualização do launcher usa `os.rename` e a remoção usa `os.unlink`/`os.rmdir` com `dir_fd` conforme a documentação da [API `os` do Python 3.14](https://docs.python.org/3.14/library/os.html#os.rename).

O binário é construído em **Debian 12 / glibc 2.36**, baseline de build. `readelf` observou GLIBC 2.34 como maior símbolo requerido no executável, mas isso não certifica uma distribuição anterior: Ubuntu 22.04 não foi testado e não é declarado suportado. Instalação limpa e abertura dos pacotes finais passaram em **Ubuntu 24.04.5** e **Fedora 44**, UID 1000, rede desligada e XDG vazio, sob Xvfb com fallback gráfico de teste. [Matriz, hashes e capturas atuais](evidence/automatic/platforms.md). Debian 12 teve compilação, extração/validação de pacotes e CLI; GUI instalada nesse sistema específico não foi exercitada.

Runtime GTK/WebKitGTK é fornecido pela distribuição. O pacote não incorpora bibliotecas de sistema nem altera a política de atualização do sistema operacional. Wayland/X11 dependem da sessão e dos pacotes de runtime já instalados.

## Verificações locais

O empacotamento é conferido sem `sudo` e sem instalar no host. A validação extrai DEB e RPM em diretórios temporários isolados, verifica executáveis, launcher, ícone, metadados de dependências e invoca `inodune-cli --help`. O instalador portátil deve ser testado somente com `--prefix` apontando para um diretório temporário; ele não abre escopos, não varre diretórios do usuário e não executa limpeza.

O workflow manual `Linux package artifacts` gera somente um artefato de CI; não publica release ou repositório. Não há assinatura de pacote, e os checksums são locais ao artefato. Os SPDX são inventários de dependências Rust e npm separados, não atestação assinada. Não anuncie estes arquivos como release confiável até estabelecer identidade de publicação, assinatura e validação em instalação limpa Ubuntu/Fedora.


O procedimento é reproduzível pelos scripts e lockfiles, com imagens base por digest e versões registradas. Reprodutibilidade bit a bit não foi demonstrada: repositórios apt/dnf e timestamps de pacotes variam. A inspeção Git avançada requer Git e Bubblewrap com `--ro-bind-fd` (0.12 no host verificado); ausência ou versão incompatível preserva o inventário como protegido, sem instalar serviços ou contornar permissões.
