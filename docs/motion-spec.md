# Motion obrigatório

Tokens: instant 100ms, fast 160ms, standard 220ms, emphasis 300ms, stagger 32ms (máximo 5 itens), easing cubic-bezier(.2,0,0,1). Só opacity/transform para entradas de telas/detalhes; cores/bordas para controles. Nada de transition:all, números interpolados, confete, pressão, remoção de linhas antes de outcome ou progresso inventado.

Mapa: hover/pressed 100ms; estado/filtro 160ms; entrada de tela 220ms; dialog 300ms. Scan mostra quantidade emitida pelo provedor sintético; não porcentagem baseada em timer. Timers apenas entregam eventos da demonstração e são rotulados como tal. Pausa/cancelamento interrompem próximos eventos. Execução sintética emite um resultado por item e permite cancelar entre itens.

prefers-reduced-motion OU preferência manual `reduce` eliminam translados, animações e efeitos contínuos. Preferência manual não pode desativar redução do OS. Mantém status textual/aria-live. Indicador sem rotação obrigatória.

Gates futuros: <100ms de bloqueio em interações e alvo 60fps em desktop mediano, medir WebKitGTK/Wayland e Chromium, listas 100/1000/10000. F0 verifica CSS computado e fluxo com e sem redução; não anuncia benchmark 60fps sem medição.
