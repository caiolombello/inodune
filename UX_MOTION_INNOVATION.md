# Inodune — UX/UI, motion design e inovação

> Especificação executável de produto e experiência · v0.2 · 31/10/2026.  
> Documento **obrigatório** para Claude Code, complementar ao `INODUNE_PLAN.md` e `INODUNE_BRAND.md`.

## 1. Norte de produto

O Inodune deve parecer um aplicativo desktop **premium, confiável e agradável** de usar, tanto para quem não conhece Linux profundamente quanto para SREs/desenvolvedores. A experiência deve transmitir controle, clareza e satisfação — **sem transformar decisões destrutivas em algo impulsivo**.

Princípios:

1. **Clareza antes de espetáculo:** visualização bonita que explica o espaço ocupado, não gráfico decorativo.
2. **Progressive disclosure:** telas simples por padrão e detalhes técnicos acessíveis a um clique.
3. **Honestidade operacional:** “estimado”, “selecionado”, “movido para Lixeira” e “efetivamente liberado” são estados distintos.
4. **Confiança visível:** mostrar a justificativa e as exclusões de cada detector; as ações perigosas exigem tempo para revisão.
5. **Foco e fluidez:** navegação e feedback ágeis, sem travamento em scans ou animações de longa duração.
6. **Sensação de cuidado:** polimento de ícones, spacing, tipografia, hover/focus, microcopy, skeletons e transições suaves em todas as telas.

## 2. Primeiro uso e jornada ideal

### Onboarding progressivo (3 etapas, dispensável)

1. **Boas-vindas:** “Seu espaço, sob seu controle.” Esclarecer: leitura local; nenhuma remoção automática. Mostrar quais dados o Inodune **não** toca.
2. **Escolha de pastas:** selecionar roots de projetos com diálogo nativo; detectar apenas diretórios XDG conhecidos. Explicar alcance e permitir “Só analisar minha pasta de cache”. Sem permissão ampla implícita.
3. **Primeira varredura:** scanner com estágio (“Encontrando caches conhecidos”, “Analisando projetos”), contador real, botão cancelar, indicação de que arquivos não estão sendo alterados. Ao concluir, resumo claro de categorias e de itens protegidos.

### Jornada principal de limpeza

**Dashboard → categoria/projeto → lista revisável → detalhes “Por que apareceu?” → seleção → simulação de impacto → confirmação → execução → relatório.**

- O usuário pode voltar sem perder a seleção enquanto o plano for válido; alterações de arquivos invalidam o plano e apresentam diff do que mudou.
- Preview por projeto mostra o tamanho único recuperável sem dupla contagem entre `target`, `.terraform`, caches de IA e worktrees aninhadas.
- “Abrir no gerenciador de arquivos” e “Copiar caminho” disponíveis; nunca abrir arquivos suspeitos automaticamente.
- Operação em execução aceita cancelar **entre itens**; não mascarar deleções já concluídas.
- Pós-limpeza evidencia bytes reais liberados, arquivos ignorados, economia potencial e custo de reconstrução.

## 3. Arquitetura de informação e telas

| Tela | Pergunta respondida | Diferencial |
| --- | --- | --- |
| Visão geral | “Onde está meu espaço e o que consigo recuperar?” | Disco real, candidatos, confiança e evolução desde o último scan |
| Projetos | “Quais repositórios ocupam mais?” | Agrupamento por root Git/Terraform, drill-down e tags de ferramentas |
| Git Worktrees | “Qual worktree merece revisão?” | Branch, modificações, commits locais, última evidência, motivo de bloqueio |
| IA & Agentes | “Quanto ocupam minhas ferramentas de IA?” | Diferenciar cache descartável de transcript, memória e sessões retomáveis |
| Sistema | “Quais caches do meu usuário são recriáveis?” | Filtros por aplicativo, custo de reaquisição e limites de escopo |
| Revisão | “Exatamente o que vai mudar?” | Plano legível, comparação antes/depois estimada e alterações pendentes |
| Histórico | “O que foi efetivamente recuperado?” | Eventos auditáveis, ações reversíveis e histórico sem telemetria |

### Layouts adaptativos

- **≥1280px:** sidebar e dashboard com duas colunas, cards e painel de detalhes contextual.
- **960–1279px:** grid de cards responsivo, drawer de detalhes; tabela com overflow controlado.
- **720–959px:** sidebar colapsável, filtros em popover, layouts em uma coluna quando necessário.
- **<720px:** preservar funcionalidade básica em janela estreita; comandos críticos sempre visíveis; sem cortes de texto perigoso.
- Suporte a escala do sistema 100–200%, textos longos PT-BR/EN, Wayland/X11, teclados e leitores de tela.

### Componentes obrigatórios

- Disk ring/usage chart **acessível** com legendas e valores em texto; gráfico nunca é o único meio de compreender números.
- Category card com tamanho, risco, número de itens, custo de rebuild, ação “Revisar” e estados de seleção/erro.
- Data table virtualizada (milhares de itens) com seleção em lote, expansão de detalhes, busca, sort, tooltips e atalhos claros.
- Side panel “Por que este arquivo?”: detector, evidência, intervalo de atividade, risco, “o que é preservado”, “como recriar” e links de ajuda opcionais.
- Compare bar **Antes / Potencial / Depois real**, sendo “Depois real” exibido somente após medir o filesystem.
- Dialog de consentimento distinto para **Lixeira** e **permanente**, com mensagem direta e foco intencional.
- Toasts discretos que não somem antes de mensagens de erro importantes; histórico recuperável.
- Loading skeletons para cards e listas, mas progressos indeterminados não devem se disfarçar de porcentagem real.
- Preferências com “modo simples / detalhes técnicos”, sem criar permissões diferentes por estética.

## 4. Sistema de animações e microinterações

**Objetivo:** feedback e orientação espacial. Animação deve reforçar causa/efeito, nunca esconder estado destrutivo.

### Tokens iniciais (ajustáveis após protótipo)

| Token | Duração | Uso |
| --- | --- | --- |
| `motion.instant` | 80–120 ms | Hover, pressionamento de botão e troca discreta de estado |
| `motion.fast` | 140–180 ms | Tooltip, highlight, toggle e ícone |
| `motion.standard` | 180–240 ms | Card expand/collapse, filtros, transição pequena |
| `motion.emphasis` | 260–320 ms | Drawer, transição entre telas, conclusão de scan |
| `motion.stagger` | 25–40 ms por item, máximo 5–6 itens | Entrada inicial de cards, uma única vez; sem atrasar interações |

- Entradas: easing tipo `cubic-bezier(0.2, 0, 0, 1)`; saídas mais rápidas. Evitar springs com overshoot em diálogos de exclusão.
- Preferir `opacity` e `transform` com composição eficiente; não animar tamanho de listas extensas, grandes `box-shadow`/blur nem fazer repaints caros contínuos.
- **Ring do scanner:** pulso lento e sutil somente durante scan; usar progresso verdadeiro quando houver total conhecido, caso contrário animação indeterminada claramente rotulada.
- **Cards de resultado:** entrada suave com fade/translate curto e uma única vez; expansão inline de detalhes mantém contexto e scroll.
- **Seleção:** checkbox dá feedback instantâneo; soma estimada pode interpolar visualmente **apenas entre valores já calculados**, sem inventar dados nem atrasar o número exato.
- **Delete:** item deve permanecer com estado “Processando” até confirmação do backend; somente então transição para “Concluído”/“Movido”; erro mostra estado persistente e opção de detalhes. Não remover linhas antes de sucesso.
- **Reclaim:** animação discreta de confirmação (até 350 ms), sem confete, gamificação ou “vitória” antes de medir ganho real.
- **Pesquisa e filtros:** transições rápidas que preservam foco e posição; nunca fazer elementos pularem para locais inesperados.
- **Alerta de risco:** nada piscando, tremendo ou pressionando o usuário.

### Acessibilidade de movimento

- Respeitar `prefers-reduced-motion: reduce`, preferência de OS e opção manual de “Reduzir animações”. Nesse modo, eliminar translados/zoom/efeitos contínuos, mantendo informação via estados instantâneos e texto.
- Sem autoplay decorativo de duração indefinida; indicador de scan pode ser pausável e substituído por texto estático quando necessário.
- Todos os controles têm foco visível, navegação por teclado, sem dependência de hover; mensagens de status anunciadas de forma não invasiva.
- Garantir que motion não reduza contraste nem cause flicker. Evitar parallax, zoom agressivo e movimentos desnecessários.

### Performance gates

- Orçamento de referência: **60 fps** em hardware desktop mediano nas microanimações; sem degradação de scan causada pelo renderer. Medir, não apenas afirmar.
- UI não deve bloquear por mais de ~100 ms em ações normais; operações de scan são background, streaming de lotes e atualização agregada limitada (ex.: 5–10 Hz) para evitar floods.
- Virtualizar listas grandes e desacoplar atualização de progresso dos itens detalhados.
- Evitar mover/animar milhares de elementos simultaneamente; se GPU/CPU saturar, reduzir motion automaticamente.
- Verificar comportamento real no WebKitGTK Linux/Wayland e no tema escuro, não usar apenas screenshots estáticos.

## 5. Inovações propostas, com prioridade

### P0 — Deveriam existir já no MVP

1. **Cleanup Explainability:** cada candidato tem “Por que isto é seguro?”, “O que pode mudar?” e “Como recriar?”. É o antídoto para cleaners opacos.
2. **Space Budget Simulator:** usuário marca categorias e vê **ganho estimado**, risco e custo provável de rebuild/download **sem executar nada**. Atualiza na hora e evita dupla contagem.
3. **Project Lens:** organizar restos por projeto (Git root) em vez de mostrar uma longa árvore de arquivos soltos. Revela onde Docker/build/TF/IA consomem espaço, mas só oferece ações implementadas.
4. **Protection Ledger:** painel transparente de exclusões: “Preservamos estes 34 itens por serem sessões/estado/dados não regeneráveis”. Reforça segurança sem assédio.

### P1 — Diferenciação após primeiro beta

5. **Space Timeline:** comparar scans locais e identificar crescimento por projeto/categoria, com retenção limitada e opção de apagar histórico. Nunca monitoramento oculto; coleta só de metadados necessários.
6. **Rebuild Cost Hint:** além de GB, indicar se o cache exigirá download, `cargo build`, `terraform init`, etc. Tempo/banda só se houver base confiável; do contrário, rótulo qualitativo “Pode demandar rebuild longo”.
7. **Smart Presets:** “Limpeza rápida”, “Dev”, “Infra/Terraform”, “IA & Agentes — revisar”, todos com seleções conservadoras, exclusões claras e execução sempre confirmada.
8. **What Changed?** comparar o novo scan com o anterior: itens novos, removidos, crescimento, mudanças de risco. Padrão local; sem serviço remoto.
9. **Command Palette:** `Ctrl+K` para buscar telas, filtros e ações não destrutivas; exclusões só por fluxo normal de revisão/consentimento.

### P2 — Pesquisa / inovação futura

10. **Disk Pressure Coach:** sugestão opt-in quando disco se aproxima de limite definido; recomenda revisar categorias, jamais faz limpeza em background sem confirmação.
11. **Project Health Insights:** “Este projeto tem caches antigos em três ferramentas”; “worktree órfã no Git” com links para revisão, sem presumir que é descartável.
12. **Undo Planner:** quando a ação for Lixeira, listar o que pode ser restaurado e prazo/local; nunca prometer restaurar exclusões permanentes.
13. **Plugin ecosystem controlado:** detectores declarativos versionados, validação de schema e políticas estritas; sem scripts de terceiros executados automaticamente.

**O que não chamar de inovação:** um chatbot que adivinha o que apagar, pontuação de “saúde” inventada, barras dramáticas sem métrica, otimização automática do sistema ou alegações imprecisas de GB recuperados.

## 6. Experimentos e critérios de aceite de UX

- **Protótipo navegável antes da limpeza destrutiva:** dashboard, listagem, detalhes, diálogo, conclusão e estados vazios/erro; desktop 1440px e janela estreita.
- **Fluxo crítico:** pessoa consegue responder “o que será removido, por quê e se consigo recuperar” **antes** de confirmar; validar com 3 desenvolvedores e 2 usuários menos técnicos, ou registrar impossibilidade e manter teste pendente.
- **Tarefas para observar:** encontrar cache de Terraform, entender por que sessão de IA está protegida, excluir uma categoria segura, cancelar, diferenciar Lixeira de exclusão permanente.
- **Acessibilidade:** alvo WCAG 2.2 AA para contraste/foco/teclado e suporte a reduced motion; audit manual + automatizada, sem tratar ferramenta automatizada como suficiente.
- **Consistência:** cada botão tem hover/focus/pressed/disabled/loading/success/error quando aplicável. Componentes têm screenshot/story de estados.
- **Motion QA:** gravar/simular navegação com motion normal e reduzido; sem itens desaparecendo antes do sucesso, sem progressos falsos, sem layouts instáveis.
- **Performance:** comparar lista de 100, 1.000 e 10.000 candidatos fictícios e escaneamento em background; definir baseline e guardar resultados de benchmark.
- **No dark patterns:** nenhuma opção destrutiva pré-selecionada por destaque visual, cor vermelha usada apenas com contexto e nenhum diálogo enganoso.

## 7. Entregáveis de design exigidos de Claude Code

1. `docs/ux-personas-and-journeys.md`: personas e jornadas reais, fluxos de primeira utilização, revisão, erro e cancelamento.
2. `docs/ux-wireframes.md`: hierarquia de telas, layout, estados e decisões de navegação; pode ter links para capturas de protótipo.
3. `docs/design-system.md`: tokens CSS, tipografia, cores, contrastes, escalas, componentes e estados.
4. `docs/motion-spec.md`: tokens, easing, mapa de animações, reduced motion e regras de performance.
5. `docs/innovation-backlog.md`: P0/P1/P2, hipóteses, dependências e como validar impacto sem perder segurança.
6. Storybook ou catálogo leve de componentes/estados, se compatível com a stack e sem comprometer cronograma.
7. Evidência visual por fase: screenshots de tela real + notas sobre navegação, foco, erro e tema escuro. **Não confundir PNG conceitual com implementação.**

## 8. Referências de princípios (não copiar estética literalmente)

- GNOME HIG (navegação/adaptatividade): https://developer.gnome.org/hig/
- GNOME HIG adaptive layouts: https://developer.gnome.org/hig/guidelines/adaptive.html
- W3C WCAG 2.2: https://www.w3.org/TR/WCAG22/
- W3C reduced motion: https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions
- Material motion (timing/easing como referência): https://m1.material.io/motion/duration-easing.html

**Decisão:** Inodune deve ter motion refinado e opcional; a segurança é visível, e o verdadeiro diferencial é uma limpeza explicável e contextualizada por projetos.
