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


## Atualização F4

Exclusões são limites de leitura, não apenas filtros visuais. Trava de operações serializa sua alteração/revogação com reserva de limpeza e invalida planos anteriores. Relatórios persistidos são JSON não confiável: namespace restrito, schema versionado, limite de tamanho, arquivo 0600/UID correto/link único, open NOFOLLOW/NONBLOCK e ancestrais confiáveis. Nenhum relatório reconstitui autoridade de exclusão. Exportar usa somente destino de picker nativo e NOREPLACE. Capacidade de histórico/journal é verificada antes de executar; não há retenção por deleção silenciosa. Panic hooks não registram payloads que possam conter segredos.

Dependências nativas são uma superfície residual: avisos RustSec do GTK3 e tooling npm estão registrados na [auditoria F4](evidence/f4/dependency-audit.md), sem supressão que os transforme em corrigidos. Sem rede obrigatória/CSP não prova ausência de todos os bugs do WebKit/GTK. Testes automatizados e revisão de IA não substituem revisão humana pré-release, hardware representativo ou certificação de segurança.


## Descoberta automática — 0.1.1

A expansão solicitada para home/discos dispensa consentimento por pasta para **ler**, sem conceder exclusão arbitrária. A raiz `/` fornece observações de volume/enumeração estrutural, nunca uma capability destrutiva. Directory FDs/openat2 impedem seguir links e cruzar mounts durante travessia; mounts locais são tratados separadamente, sem dupla autoridade sobre a mesma subárvore. Outros usuários não são percorridos. Network/FS desconhecido e falhas reduzem cobertura explicitamente. O leitor de mountinfo tem limite de 8 MiB/4096 registros; raízes têm orçamento e travessia estrutural limitada.

Broad scan contém regiões protegidas por definição. Preparar os itens inteiramente verificados de um snapshot automático parcial não autoriza arquivos ignorados: cada target completo, política, witnesses, identidade, versão/hash/TTL e confirmação nativa continuam obrigatórios. Snapshot cancelado nunca autoriza plano. O atime de diretório root-owned pode mudar por leitura do kernel; atime de arquivos/árvores próprias permanece protegido por NOATIME. Não confundir isso com ausência de todas as alterações possíveis de OS ou com permissão de escrita do app nos dados.
