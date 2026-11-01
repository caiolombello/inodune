# Wireframes e estados — Fase 0

Desktop ≥1280: sidebar 224px com marca/rotas/status; toolbar com escopo e preferências; título editorial; painel de volume com anel e legenda textual; 4 categorias; lista de itens e acesso a projetos/protegidos. Revisão tem filtros e seleção + rodapé de impacto. Detalhes em dialog lateral. Impacto inclui comparação antes/potencial/depois real indisponível. Confirmação separa permanente/Lixeira. Relatório conserva linhas com outcomes.

960–1279: cartões em 2 colunas e conteúdo principal fluido. 720–959: navegação compacta horizontal, painel em coluna. <720: 1 coluna, linhas de lista empilhadas e ações quebram linha; caminho integral com wrap. Zoom 200% precisa manter reflow.

Rotas: visão geral, projetos, desenvolvimento, Git, IA, sistema, revisão, protegidos, histórico, configurações e catálogo de estados. Navegação sem dependência de hover, skip link, foco em título ao trocar tela. Dialog nativo prende/restaura foco, Escape cancela; ação destrutiva real inexistente.

Estados: inicial, carregando/pausado/cancelado, resultados parciais, vazio, erro de permissão, filtro vazio; item desmarcado/selecionado/protegido; confirmação com política; execução pendente/processando/simulado/pulado/falhou/cancelado. Cenários explícitos no onboarding/configurações permitem reproduzir os estados sem falhas artificiais ocultas.

Evidências reais do protótipo serão ligadas em [evidence/phase0.md](evidence/phase0.md); PNG original continua sendo só referência.
