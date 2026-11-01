# WebKit Playwright local — 01/11/2026

O wrapper MiniBrowser do fallback Ubuntu substitui LD_LIBRARY_PATH; simplesmente exportar as bibliotecas no processo pai não resolve. Não alteramos o browser instalado nem bibliotecas do sistema. Quatro bibliotecas oficiais presentes na imagem Ubuntu isolada já provisionada foram copiadas para .tools/playwright-libs (origem e hashes em webkit-libraries.json).

Configuração temporária .tools/playwright-webkit.config.ts importa a configuração do projeto e altera somente o launch: executablePath aponta para webkit-2370/minibrowser-wpe/bin/MiniBrowser; env define WEBKIT_EXEC_PATH=.../bin, WEBKIT_INJECTED_BUNDLE_PATH=.../lib, WEBKIT_INSPECTOR_RESOURCES_PATH=.../share e LD_LIBRARY_PATH=<repo>/.tools/playwright-libs:<browser>/lib:<browser>/sys/lib. testDir é o diretório absoluto apps/desktop/tests, webServer.cwd é apps/desktop. As demais regras/asserts do projeto são preservadas. Playwright 1.64.1; WebKit 27.2 build 2370.

Executado com cwd apps/desktop:

```sh
../../node_modules/.bin/playwright test --config ../../.tools/playwright-webkit.config.ts \
  --grep-invert 'records scanner motion|captures local scanner' --workers=2
```

Resultado: 24 PASS em 42,7 s. As duas provas de imagem/vídeo permanecem demonstradas em Chromium. Falha inicial: bibliotecas ICU74/JPEG8 ausentes. Primeira execução com browser direto passou 13 testes e falhou ao escrever screenshots porque cwd raiz resolvia ../../docs fora do projeto; corrigir o cwd fez todos os 24 passarem. Nenhuma permissão foi alterada, nenhum sudo foi usado e nenhuma falha foi convertida em PASS. Isso não certifica o WebKitGTK nativo, verificado separadamente no harness IPC.
