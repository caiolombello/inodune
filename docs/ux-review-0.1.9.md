# Revisão de UX/UI — 0.1.9

Capturas feitas com Playwright (Chromium, 1440×960, 720×900 e tema escuro) sobre o
build de desenvolvimento, com IPC do Tauri simulado por dados fictícios e anonimizados:
análise automática "incompleta" (sistema ignorado), 44 itens de Desenvolvimento,
25 avisos `Not a directory (os error 20)`, caminhos com `/./`, relatório com
`2000000` bytes e desinstalação com IDs crus do PackageKit. Nenhum dado real foi lido.

## Problemas encontrados (antes)

| Área | Problema | Categoria |
| --- | --- | --- |
| Visão geral | Análise automática termina "incompleta", então nada vem marcado: "Desenvolvimento · 44 · 0 B" contradiz "Itens seguros já entram na limpeza". | Honestidade, G1 |
| Visão geral | Botão "Limpar" sem contagem nem tamanho; clicar sem seleção mostra texto solto que empurra o layout ~25 px e persiste. | Hierarquia, estados |
| Visão geral | Banner de cobertura com chaves internas (`structural_denied: 3`, `system_protected: 12`, `hardlink`) e 20+ linhas "Not a directory (os error 20)". | Textos pt-BR |
| Todas | Banner de cobertura e faixa "Análise local" repetidos em todas as telas, inclusive Configurações e Relatório. | Densidade |
| Visão geral | Lista "Revisar itens" com 40 linhas idênticas "Bytecode Python", sem caminho; protegidos com "Cobertura incompleta (cobertura incompleta)". | Densidade, texto |
| Visão geral | Confirmação aparece no fim do painel, fora da área visível; foco por `scrollIntoView`. | M4, foco |
| Visão geral | Anel do volume corta "capacidade do filesystem"; legenda "statvfs" técnica. | Visual |
| Cards | Botão "Limpar" ativo com zero itens; card cresce ao marcar o primeiro item. | Estados, salto de layout |
| Revisão | Linhas protegidas repetem "Protegido" + rótulo curto + "Somente análise". | Consistência |
| Eyebrows | "SPACE BUDGET SIMULATOR", "PROJECT LENS", "PROTECTION LEDGER" na UI pt-BR. | Textos pt-BR |
| Confirmação de limpeza | Hash e snapshot do plano expostos; "1 itens"; "válido por 120 segundos". | Textos, plural |
| Relatório | "2.000.000 bytes lógicos", "Dispositivo 42", `/./` nos caminhos, avisos com chaves internas e duplicados, "Sem mensagem registrada." em toda linha. | Unidades, textos |
| Histórico | "N arquivos · 2048214 bytes processados"; ID do plano em destaque. | Unidades |
| Programas & Jogos | Lista sem busca, filtro, ordenação ou contador; fonte com chave crua (`flatpak_user`); motivo de proteção escondido em "Notas e limites"; "Último uso desconhecido." repetido. | H, densidade |
| Programas & Jogos | Confirmação de desinstalação no topo da página com `aisleriot;1:3.22.35-5;amd64;manual:…` como texto principal; sem tamanho nem separação de dependências. | G2, M4 |
| Programas & Jogos | Erros como texto solto: "PackageKit recusou a transação", "PackageKit não concluiu com sucesso". | F |
| Diálogo nativo | Mesmo ID cru e sem tamanho. | G2 |
| Item "revisar" | Texto "Inventário não autoriza remoção" num item que é limpo quando marcado. | G4 |
| Release | Corpus demo (atlas, `/demo/…`) e "Protótipo navegável" no bundle de produção. | G |
| CLI | Ajuda cita `inodune apps`; `clean --demo` só funciona com `--dry-run`; `Some(-32768)`; plano de desinstalação só depois de "Digite UNINSTALL". | G, I |
| Pacote .deb | Lintian: malformed-contact, missing-dependency-on-libc, no-changelog, no-copyright-file. | G |
| Acessibilidade | `aria-label` em `span` sem `role` (cadeado); textos compostos em vários nós dificultam leitura por leitor de tela. | A11y |

## Decisões

- Pré-seleção: só risco baixo com ação suportada e fora de "Exige cuidado", em análises não
  canceladas que o executor aceita (completas ou automáticas). Review e protegidos nunca.
- Confirmações usam `<dialog>` modal (`showModal`), que move o foco para dentro e fica
  visível qualquer que seja a posição da rolagem.
- O aviso de cobertura aparece só na Visão geral; os detalhes ficam recolhidos.
- A lista da Visão geral continua recolhida por padrão (decisão da 0.1.8); o resumo mostra
  "N de M itens selecionados".
- Demo: alias de build `virtual:inodune-demo` e constante `__INODUNE_DEMO__`; `vite build`
  usa um corpus vazio e descarta a UI de demonstração.
- Mensagens do PackageKit: códigos `PkErrorEnum`/`PkExitEnum` conferidos no
  `PackageKitGlib-1.0.typelib` instalado (NOT_AUTHORIZED=48, DEP_RESOLUTION_FAILED=13,
  CANNOT_REMOVE_SYSTEM_PACKAGE=20, CANNOT_GET_LOCK=26, TRANSACTION_CANCELLED=17).

## Verificação

`tests/ux-review.spec.ts` cobre G1, G2, M4 e (d)–(h) com IPC simulado, inclusive axe.
`src/lib/present.test.ts` cobre caminhos, plurais, chaves de cobertura e erros.
`crates/inodune-apps/src/presentation.rs` cobre o plano legível e as mensagens do PackageKit.
`scripts/tests/test_packager_metadata.py` cobre os metadados do `.deb`.
