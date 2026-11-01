# Ameaças de perda de dados

Ativos: arquivos pessoais, código, Git, estado Terraform, credenciais, sessões e memória de agentes. Adversários: renderer comprometido, manifest adulterado, concorrência local, árvores hostis e executáveis PATH maliciosos. Limite: processo sem root; outro processo do mesmo UID ainda pode modificar os alvos.

| Ameaça/cenário | Controle requerido antes de habilitar FS real | Evidência Fase 0 |
| --- | --- | --- |
| Symlink substituído entre scan e limpeza, inclusive ancestral | dirfd, openat2/NOFOLLOW, revalidação por entrada; não basta canonicalize | Executor ausente; teste adversarial pendente F2 |
| Bind mount/mount externo muda sob root | bloquear mudança de mount/dev; testar bind mount em ambiente isolado | Pendente F1/F2 |
| Hardlinks, reflinks, sparse, CoW e arquivos abertos | não prometer ganho; bytes alocados e lógico separados, medição posterior com limites de atribuição | UI não mede nem anuncia ganho real |
| ID/path adulterado, plano velho ou consentimento reutilizado | IDs opacos resolvidos internamente, hash/nonce/expiração e snapshot de seleção, revalidação | Planos sintéticos rejeitam desconhecidos/protegidos |
| Git dirty/locked/main/sem upstream/detached/commits exclusivos | inventário conservador, Git nativo sem force, revalidar; idade não prova desuso | Worktrees sintéticas protegidas |
| Terraform backend/estado/locks/plans/credenciais | whitelist de subpastas verificadas, nunca .terraform inteira | Inventário protegido |
| JSONL/memória de agentes confundidos com cache | inventário protegido sem botão de excluir, documentação por ferramenta/versão | Corpus + testes de seleção |
| Permissão, cancelamento, queda de energia, modificação concorrente | resultados item a item, parcial honesto, sem promessa de rollback | Cenários simulados de alterado/falha/cancelado |
| Lixeira no mesmo volume interpretada como recuperação | distinção entre lógico movido e livre medido; nunca esvaziar automaticamente | Simulador mostra potencial físico zero na mesma unidade |
| Sobreposição de detectores e worktree aninhada | ID/identidade de FS, decompor coberturas, não somar contêiner e filhos | IDs duplicados recusados; cobertura desconhecida bloqueia plano |
| Upload, XSS/path com markup, binário malicioso | render texto escapado, CSP, zero plugins remotos/shell; Git confiável com argv | Sem conteúdo remoto/invoke; paths são texto |

Risco residual: a Fase 0 NÃO demonstra segurança TOCTOU nem integridade em Linux real. Antes de F2 são obrigatórios testes hostis de filesystem temporário, revisão humana pré-release e validação de capabilities em runtime nativo. Confirmação visual nunca concede autoridade ao renderer.

## Atualização F1/F2 (01/11/2026)

O quadro acima preserva o estado histórico da Fase 0. O scanner agora usa descritores/openat2 e limites reais; o protocolo F2 está em [cleanup-protocol.md](cleanup-protocol.md). Testes temporários cobrem troca de folha/pai, symlinks, hardlinks posteriores ao scan, escrita por descritor aberto, cancelamento parcial, arquivos novos, fonte Python removida e armazenamento privado. Bind mounts são exercitados somente em namespace user/mount privado. Resultados exatos e integração nativa são registrados na evidência de cada fase, sem herdar PASS da F0.

A área privada não é uma sandbox contra outro processo do mesmo UID. A comparação pós-captura previne apagar substitutos observados; a última janela compare/unlink não pode ser eliminada pela API Linux. Aprovação de GUI resolve apenas IDs; confirmação nativa e revalidação continuam no Rust. Journals com intenções sem resultado são UNKNOWN após interrupção; não há rollback automático.
