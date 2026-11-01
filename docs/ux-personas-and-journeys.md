# Personas e jornadas

Hipóteses de design, não pesquisa realizada.

- Marina, desenvolvedora: quer entender custo de recompilar target sem perder mudanças em worktrees. Entra por Projetos, escolhe projeto, inspeciona detector/evidência/preservação, seleciona individualmente e compara impacto.
- Paulo, usuário comum: quer espaço sem conhecer caches. Entra pelo onboarding dispensável, escolhe escopo explícito, revisa apenas baixo risco e lê diferença entre Lixeira e permanente.

Primeiro uso: boas-vindas → pastas de exemplo (sem acesso nativo nesta fase) → scan sintético. Sempre mostrar demonstração. Dispensar onboarding leva ao estado sem scan, nunca dispara atividade real.

Jornada: visão geral → categoria/revisão ou projeto → detalhe Por que apareceu? → seleção → simulador → diálogo com consentimento desmarcado → processamento simulado → relatório. Voltar preserva seleção; novo scan limpa plano/seleção. Cancelar scan preserva só itens já emitidos e os marca parciais; cancelar execução mantém outcomes e registra restante cancelado. Erros persistem no relatório; item alterado é pulado.

Validação futura: observar 3 devs e 2 não técnicos explicarem o que muda, o que se preserva e se há reversão; encontrar TF, entender IA protegida, cancelar e distinguir potencial de livre medido. Meta: todos reconhecem ausência de ganho físico na Lixeira no mesmo volume. Status: pendente, sem participantes.


F4: as jornadas de teclado PT/EN, histórico persistido/exportação e exclusões foram verificadas por automação em fixtures; não equivalem a testes com as personas reais. A rodada com participantes continua pendente e não recebeu pontuações de confiança/tempo inventadas.
