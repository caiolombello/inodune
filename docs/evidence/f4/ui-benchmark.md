# UI benchmark — Fase 4

Comando: `npm run test:e2e -w @inodune/desktop -- --project=chromium --workers=1 tests/virtual-list-f4.spec.ts`. Resultado: 3/3 passaram em Chromium Headless 156.0.8078.4; build Vite de produção executado antes da suíte. O teste injeta candidatos sintéticos pelo stub IPC do navegador e não chama o scanner nem o executor nativos.

| Itens sintéticos | DOM abaixo da revisão | Linhas antes da busca | Linhas após limpar busca | Teclado/paginação | Largura 420 px |
| ---: | ---: | ---: | ---: | --- | --- |
| 100 | 2.163 | 100 | 100 | Busca exata; lista integral | documento 420 px |
| 1.000 | 467 | 15 | 100 | PageDown, paginação para 100 por página, busca exata | documento 420 px |
| 10.000 | 467 | 15 | 100 | PageDown, página 2/100, busca exata | documento 420 px |

Tempos do fluxo até a revisão e 90 intervalos de `requestAnimationFrame` estão nos JSONs por cenário. Variam com carga e paralelismo local; não são um SLO nem prova de 60 fps em hardware desktop. O browser roda headless com IPC sintético, sem caracterização de GPU ou carga do scanner nativo.

Captura visual em 420×800: [ui-10000-review-420px.png](ui-10000-review-420px.png). O caso de 10.000 mantém 467 nós DOM abaixo da revisão e limita a lista visível a 15 linhas no viewport inicial; página acessível contém 100 linhas. Os JSONs individuais guardam tempos, contagens, user agent e amostras. Valores de tempo são evidência local deste ambiente, não metas de desempenho.
