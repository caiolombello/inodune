# Política de risco

`low`: apenas padrão positivamente validado, recriável, inativo e dentro de escopo. Pode ser sugerido após scan; nunca executa automaticamente. `review`: consentimento por item, custo de rebuild/download explicado, inicia desmarcado. `protected`: nenhuma ação; toda incerteza, estado, sessões, memória, mounts não aprovados e dados pessoais. Risco não é reduzido por idade.

Fase 0: todos os itens são sintéticos. Terraform, Git e IA são inventário protegido. Rust target é exemplo de revisão, sem autorização real. Thumbnails e Python são exemplos de baixo risco, sem detector implementado. Campos de identidade e bytes alocados são null. Ações são sempre `none`; a simulação de seleção é somente UX e não elegibilidade real. CLI exige `--demo`, não aceita roots ou apply. Duplicatas e cobertura sobreposta não resolvida impedem plano. Nenhum fallback silencioso de medição desconhecida para zero.
