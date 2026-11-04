# ADR 0003 — Gráficos mostram só valores medidos; estimativas ficam ao lado

Status: aceito, 03/11/2026 (0.1.14).

## Contexto

O conceito visual (`inodune-ui-concept.png`) desenha no anel do disco um segmento "pronto para revisar". Durante a 0.1.14 chegou a ser implementado um segmento menta com a seleção dentro do anel de capacidade. A revisão independente (GPT-6.1 Terra) rejeitou a ideia, e a objeção procede:

- bytes lógicos não são necessariamente bytes físicos ocupados: arquivos esparsos, hardlinks, CoW, snapshots e sobreposição de caminhos;
- igualdade de `st_dev` não prova que a fração desenhada seja válida;
- com a política Lixeira no mesmo volume, nada é liberado até a Lixeira ser esvaziada.

Desenhar a seleção como fatia do disco sugere espaço que talvez nunca seja liberado. Isso contradiz a regra do projeto de nunca anunciar ganho antes de medir.

## Decisão

- O anel mostra apenas **ocupado** e **disponível**, medidos por `statvfs` na análise.
- A seleção aparece fora do gráfico, como **"Selecionado para limpar"**, com a nota "Estimativa lógica" e a consequência da política: na Lixeira os itens continuam ocupando espaço, e na exclusão permanente o ganho real só é medido no relatório. Com análise parcial, um aviso fica ao lado do botão de limpar.
- As barras de composição por risco (cards de categoria e de projeto) somam só tamanhos somáveis (ver ADR 0002), em categorias mutuamente exclusivas (`low`, `review`, `protected`), com legenda em texto. Desconhecidos aparecem como contagem, nunca como zero.
- Relatório: os contadores derivam dos estados reais dos arquivos. Quarentena, preservação por alteração e cancelamento não contam como sucesso. A nota de recuperação aparece só se houver arquivos naquele destino. A variação de volume é exibida em cor neutra, sem atribuição à limpeza.

## Consequências

O hero perde o efeito do anel com a fatia de seleção, mas não promete espaço. Qualquer gráfico novo deve seguir a mesma regra: medido dentro do gráfico, estimado em texto ao lado.
