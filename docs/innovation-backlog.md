# Inovações e validação

| Prioridade | Entrega | Fase 0 | Validação |
| --- | --- | --- | --- |
| P0 | Cleanup Explainability | Detalhe de candidato: detector/evidências/atividade/efeito/recriação/preservação | Antes do consentimento pessoa explica o que perde |
| P0 | Space Budget Simulator | Soma única por ID, cobertura sobreposta recusada; seleção, risco/custo, Lixeira vs permanente | Duplicata não aumenta potencial; depois real indisponível |
| P0 | Project Lens | Agrupamento e drill-down de corpus por projeto | Soma de projeto consistente; protegidos separados |
| P0 | Protection Ledger | Inventário intocável com motivos, IA/Git/TF | Nenhum protegido selecionável, mesmo via plano |
| P1 | Timeline/What Changed | Não implementado | Precisam scans reais e retenção local |
| P1 | Rebuild hints quantitativos | Só indicação qualitativa no exemplo | Não inventar tempo/banda |
| P1 | Presets/palette | Não implementado | Sem atalho de consentimento |
| P2 | Pressure Coach/Insights/Undo Planner | Não implementado | Opt-in, sem deleção automática/rollback falso |
| P2 | Detectores declarativos | Não implementado | Sem código arbitrário |

Todos P0 compartilham estado inicial/loading/empty/error do scan e busca vazia. Integração real só na F1. Usabilidade e métricas de tempo/confiança permanecem pendentes; não atribuir resultados a pessoas inexistentes.


F1–F4: os quatro P0 operam também sobre o inventário real de fixtures via core Rust, com cobertura particionada e incerteza preservada. O relatório diferencia bytes processados, Lixeira e delta observado; histórico persistido pode ser reaberto sem gerar um novo plano. P1/P2 continuam fora do MVP. Benchmarks scanner/UI 100/1.000/10.000 estão em `evidence/f4`; validação por participantes permanece pendente.
