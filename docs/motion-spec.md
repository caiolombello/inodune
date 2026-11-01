# Motion obrigatório

Tokens: instant 100ms, fast 160ms, standard 220ms, emphasis 300ms, stagger 32ms (máximo 5 itens), easing cubic-bezier(.2,0,0,1). Só opacity/transform para entradas de telas/detalhes; cores/bordas para controles. Nada de transition:all, números interpolados, confete, pressão, remoção de linhas antes de outcome ou progresso inventado.

Mapa: hover/pressed 100ms; estado/filtro 160ms; entrada de tela 220ms; dialog 300ms. Scan mostra quantidade emitida pelo provedor sintético; não porcentagem baseada em timer. Timers apenas entregam eventos da demonstração e são rotulados como tal. Pausa/cancelamento interrompem próximos eventos. Execução sintética emite um resultado por item e permite cancelar entre itens.

prefers-reduced-motion OU preferência manual `reduce` eliminam translados, animações e efeitos contínuos. Preferência manual não pode desativar redução do OS. Mantém status textual/aria-live. Indicador sem rotação obrigatória.

Gates futuros: <100ms de bloqueio em interações e alvo 60fps em desktop mediano, medir WebKitGTK/Wayland e Chromium, listas 100/1000/10000. F0 verifica CSS computado e fluxo com e sem redução; não anuncia benchmark 60fps sem medição.


F4: histórico/exportação/exclusões só exibem sucesso após resposta do backend; cancelamento do picker não anuncia exportação. Entrada no relatório continua acessível durante execução. Capturas esperam frames de render, sem alterar números. Manual reduced motion e `prefers-reduced-motion` continuam globais. Samples rAF em Chromium headless são apenas observação local: 60 fps no compositor/hardware representativo permanece pendente.


0.1.2: ScanExperience substitui o contador isolado por instrumento de atividade e descobertas reais (máximo quatro, atualização do core limitada a 100 ms). Entradas usam 220 ms; o giro indeterminado de atividade é lento (8/12 s), sem relação com percentual. Pausa interrompe o giro; preferência manual ou do OS elimina animações; aba oculta pausa movimento e fecha áudio. Nunca animar números calculados ou anunciar progresso percentual desconhecido. O painel mantém estados textuais e controles acessíveis.

Som: opt-in persistido, síntese sine local com envelope de 170–260 ms e ganho baixo; contexto só é desbloqueado por gesto. Encerramento completo e situação parcial/erro têm sinais distintos após confirmação, sem música contínua, som por arquivo ou reprodução atrasada de eventos em segundo plano. Mutar invalida desbloqueios pendentes. Falha de áudio não bloqueia scanner nem executor. Base: [AudioContext.resume](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/resume), [visibilidade](https://developer.mozilla.org/en-US/docs/Web/API/Page_Visibility_API) e [controle de movimento WCAG](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html).

Vídeo e amostra de frames locais: evidence/launch/scanner. São Chromium headless com IPC de fixture, não certificação de 60 fps em hardware nativo. Playwright usa a [API oficial de vídeo](https://playwright.dev/docs/videos).
