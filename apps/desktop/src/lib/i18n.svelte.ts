import { catalog as demoCatalog } from 'virtual:inodune-demo';
export type Locale = 'pt-BR' | 'en-US';

const catalog: Record<string, string> = {
  'Análise automática · limpeza com confirmação':
    'Automatic analysis · cleanup requires confirmation',
  'A home e as unidades locais montadas são descobertas automaticamente. Áreas protegidas ficam fora da limpeza.':
    'Your home and mounted local volumes are discovered automatically. Protected areas are excluded from cleanup.',
  'Clique em Analisar computador. Não é necessário escolher pastas.':
    'Click Analyze computer. No folder selection is required.',
  'Subpasta de miniaturas na localização XDG confirmada pelo backend.':
    'Thumbnail subfolder in the XDG location confirmed by the backend.',

  'Movido à Lixeira; ainda ocupa espaço.': 'Moved to Trash; still uses space.',
  'Removido; verifique variação observada do volume.':
    'Removed; check the observed volume change.',
  'VISÃO GERAL': 'OVERVIEW',
  'Analise o exemplo para encontrar candidatos.':
    'Analyze the sample to find candidates.',
  'Escolha pastas específicas para analisar. Nenhum scan é iniciado automaticamente.':
    'Choose specific folders to analyze. No scan starts automatically.',
  'Explore o fluxo com pastas de exemplo. Nada será lido do seu disco.':
    'Explore the flow with sample folders. Your disk will not be read.',
  'Escolher pastas': 'Choose folders',
  'Escolher escopos de exemplo': 'Choose sample scopes',
  'Seus projetos': 'Your projects',
  'Seus projetos de exemplo': 'Your sample projects',
  'Escolha uma pasta de projetos e inicie a análise.':
    'Choose a project folder and start analyzing.',
  'Inicie uma análise de exemplo para explorar o Project Lens.':
    'Start a sample scan to explore Project Lens.',
  'no total': 'in total',
  '0 B na mesma unidade': '0 B on the same volume',
  'exclusão permanente': 'permanent deletion',
  'Ação de exemplo reconhecida.': 'Recognized sample action.',
  'Inodune · dados locais': 'Inodune · local data',
  'Prévia de produto · dados sintéticos': 'Product preview · synthetic data',
  'Seu espaço, sob seu controle.': 'Your space, under your control.',
  'Acompanhar limpeza': 'Follow cleanup',
  'Ver último relatório': 'View latest report',
  'Exportando…': 'Exporting…',
  'Exportar JSON local': 'Export local JSON',
  'Relatório exportado': 'Report exported',
  'O relatório inclui caminhos locais. Arquivos existentes nunca são substituídos.':
    'The report includes local paths. Existing files are never replaced.',
  'Ação suportada:': 'Supported action:',
  'Custo / como recriar': 'Cost / how to rebuild',
  'lógicos estimados': 'estimated logical',
  lógicos: 'logical',
  'válido por': 'valid for',
  segundos: 'seconds',
  'alocado não estimado': 'allocation not estimated',
  'alocados estimados': 'allocated estimate',
  'itens concluídos': 'items completed',
  'Não atribuídos': 'Not attributed',
  'Ação:': 'Action:',
  'Relatório do executor local com cada arquivo processado.':
    'Local executor report with each processed file.',
  'Executando limpeza aprovada.': 'Running approved cleanup.',
  'Limpeza concluída.': 'Cleanup complete.',
  'Limpeza cancelada.': 'Cleanup cancelled.',
  'Limpeza encerrada com itens pendentes.': 'Cleanup ended with pending items.',
  'EM ANDAMENTO': 'IN PROGRESS',
  'RELATÓRIO DE LIMPEZA': 'CLEANUP REPORT',
  'RELATÓRIO DE DEMONSTRAÇÃO': 'DEMO REPORT',
  'Item concluído': 'Item complete',
  'Item incompleto': 'Item incomplete',
  Dispositivo: 'Device',
  Antes: 'Before',
  depois: 'after',
  'não medida': 'not measured',
  'Variações observadas não são atribuídas à limpeza. Arquivos enviados para a Lixeira não são contabilizados como espaço liberado.':
    'Observed changes are not attributed to cleanup. Files sent to Trash are not counted as freed space.',
  'Arquivos movidos para a Lixeira continuam ocupando espaço. Variações observadas no volume não são ganho atribuído à limpeza.':
    'Files moved to Trash continue to use space. Observed volume changes are not attributed to cleanup.',
  'Variação observada no filesystem; arquivos abertos, CoW, snapshots e atividade externa podem adiar ou alterar a liberação. Sem garantia de apagamento seguro.':
    'Observed filesystem change; open files, CoW, snapshots, and external activity may delay or change reclamation. Secure deletion is not guaranteed.',
  'Manifesto ausente': 'Manifest missing',

  'Seu espaço, em perspectiva.': 'Your space, in perspective.',
  'Cada projeto tem seu espaço.': 'Every project has its own space.',
  'Construa. Revise. Recomece.': 'Build. Review. Begin again.',
  'Código merece cuidado.': 'Code deserves care.',
  'Contexto que vale preservar.': 'Context worth preserving.',
  'Leveza no dia a dia.': 'A lighter everyday workflow.',
  'Revise com tranquilidade.': 'Review with confidence.',
  'O essencial fica.': 'Keep what matters.',
  'Cada decisão, às claras.': 'Every decision, clearly explained.',
  'Do seu jeito.': 'Your way.',
  'Um sistema, todos os estados.': 'One system, every state.',
  'Antes de decidir, entenda.': 'Understand before you decide.',
  'O resultado, sem atalhos.': 'The result, with nothing hidden.',
  'Seu espaço, sob seu controle': 'Your space, under your control',
  Relatório: 'Report',
  'Pular para conteúdo': 'Skip to content',
  'Navegação principal': 'Main navigation',
  'Áreas do aplicativo': 'App sections',
  'ESPAÇO DE TRABALHO': 'WORKSPACE',
  'Visão geral': 'Overview',
  Projetos: 'Projects',
  Desenvolvimento: 'Development',
  'Git Worktrees': 'Git worktrees',
  'IA & Agentes': 'AI & agents',
  Sistema: 'System',
  'Itens protegidos': 'Protected items',
  Histórico: 'History',
  Configurações: 'Settings',
  'Local por natureza': 'Local by nature',
  'Sem conta. Sem telemetria.': 'No account. No telemetry.',
  'Linux desktop': 'Linux desktop',
  'Alternar tema': 'Toggle theme',
  '100% local': '100% local',
  'Liberte espaço. Preserve o essencial.':
    'Free up space. Preserve what matters.',
  'Veja o que preservamos e entenda cada motivo.':
    'See what we preserve and why.',
  'Artefatos, custos e proteções no contexto de cada projeto.':
    'Artifacts, costs, and protections in each project context.',
  'Uma estimativa para revisar. Nenhuma operação será executada.':
    'An estimate for review. No operation will run.',
  'Decisões claras, com você no controle.':
    'Clear decisions, with you in control.',
  'Analisar pastas': 'Analyze folders',
  'Analisar computador': 'Analyze computer',
  'Analisa automaticamente volumes locais acessíveis sem privilégios. Áreas protegidas e limitações serão relatadas; nada será removido.':
    'Automatically scans accessible local volumes without elevated privileges. Protected areas and limitations are reported; nothing is removed.',
  'Analisa automaticamente volumes locais acessíveis sem privilégios. Áreas do sistema, outros usuários, rede, links e caminhos sem leitura permanecem protegidos e podem aparecer como avisos. Nenhuma remoção ocorre durante a análise.':
    'Automatically scans accessible local volumes without elevated privileges. System areas, other users, network paths, links, and unreadable paths stay protected and may appear as warnings. Nothing is removed during analysis.',
  'Modo avançado: escolha escopos específicos. Links, mounts externos, sessões e estado são preservados.':
    'Advanced mode: choose specific scopes. Links, external mounts, sessions, and state are preserved.',
  'Modo avançado: escolha escopos específicos no diálogo nativo. A aprovação não inicia o scan.':
    'Advanced mode: choose specific scopes in the native dialog. Approval does not start the scan.',
  'O computador será analisado automaticamente em modo somente leitura. Áreas sem acesso ou protegidas serão informadas, nunca contornadas.':
    'The computer is scanned automatically in read-only mode. Inaccessible or protected areas are reported, never bypassed.',
  'O scanner Rust vai descobrir volumes locais acessíveis, sem privilégios e sem seguir links. Áreas do sistema, outros usuários, rede e caminhos sem leitura serão preservados e relatados. Você poderá pausar, cancelar e revisar os resultados.':
    'The Rust scanner discovers accessible local volumes without elevated privileges or following links. System areas, other users, network paths, and unreadable paths are preserved and reported. You can pause, cancel, and review results.',
  'Analisar exemplo': 'Analyze sample',
  'Analisar novamente': 'Analyze again',
  'Analisando volumes locais': 'Scanning local volumes',
  'A análise tem áreas não examinadas. Este plano contém somente os itens selecionados com manifesto completo; nada fora dele será alterado.':
    'Some areas were not examined. This plan contains only the selected items with complete manifests; nothing outside it will be changed.',
  'Análise local': 'Local analysis',
  'Conhecer o fluxo': 'See how it works',
  'Meu computador': 'My computer',
  'Ambiente de demonstração': 'Demo environment',
  'Escopos aprovados · limpeza com confirmação':
    'Approved scopes · cleanup requires confirmation',
  'Somente dados sintéticos': 'Synthetic data only',
  'Analisar pastas aprovadas': 'Analyze approved folders',
  'Analisar cenário': 'Analyze scenario',
  'Aparência e acessibilidade': 'Appearance and accessibility',
  Tema: 'Theme',
  'O modo automático acompanha o sistema.':
    'Automatic mode follows your system.',
  Claro: 'Light',
  Escuro: 'Dark',
  'Reduzir animações': 'Reduce motion',
  'A preferência do sistema também é respeitada.':
    'Your system preference is also respected.',
  'Pastas e origem dos dados': 'Folders and data source',
  'Fonte de dados': 'Data source',
  Demonstração: 'Demo',
  'Selecione pastas específicas. Links, mounts externos, sessões e estado são preservados.':
    'Select specific folders. Links, external mounts, sessions, and state are preserved.',
  'Os caminhos abaixo são fictícios e não são acessados.':
    'The paths below are samples and are not accessed.',
  Idioma: 'Language',
  'Escolha o idioma da interface.': 'Choose the interface language.',
  'Português (Brasil)': 'Portuguese (Brazil)',
  'Inglês (Estados Unidos)': 'English (United States)',
  'Análise cancelada': 'Analysis cancelled',
  'Análise com cobertura parcial / exclusões':
    'Analysis with partial coverage / exclusions',
  'Análise encerrada': 'Analysis complete',
  'Análise de exemplo concluída': 'Sample analysis complete',
  'Arquivos relatados': 'Reported files',
  'Avisos e limites': 'Warnings and limits',
  'Ação primária': 'Primary action',
  'Baixo risco': 'Low risk',
  'Bytes liberados atribuídos': 'Attributed bytes freed',
  'Bytes lógicos processados': 'Logical bytes processed',
  'Cancelar entre itens': 'Cancel between items',
  'Cancelar limpeza': 'Cancel cleanup',
  'Carregamento indeterminado': 'Indeterminate loading',
  Cenário: 'Scenario',
  'Como recriar': 'How to rebuild',
  'Componentes em contexto': 'Components in context',
  'Demonstração, sem operações reais': 'Demo, with no real operations',
  'Depois real': 'Real afterward',
  'Desmarcar todos': 'Deselect all',
  'Detector e evidências técnicas': 'Detector and technical evidence',
  'Erro de permissão': 'Permission error',
  'Erro de permissão no scan': 'Scan permission error',
  'Escolha outro escopo para continuar.': 'Choose another scope to continue.',
  'Escolher thumbnails': 'Choose thumbnails',
  'Espaço efetivamente recuperado': 'Space actually recovered',
  'Exclusão permanente': 'Permanent deletion',
  'Execução parcial': 'Partial execution',
  'Experimentar cenários completos': 'Try complete scenarios',
  'Explorar sem analisar': 'Explore without analyzing',
  'Falha e item alterado na execução':
    'Failure and changed item during execution',
  'Feito para abrir espaço. Projetado para preservar.':
    'Made to free up space. Designed to preserve.',
  'Filtrar por risco': 'Filter by risk',
  'Geralmente reversível · sem ganho imediato na mesma unidade':
    'Usually reversible · no immediate gain on the same volume',
  Hash: 'Hash',
  Indisponível: 'Unavailable',
  'Ir para revisão': 'Go to review',
  'Irreversível em operação real · confirmação explícita':
    'Irreversible in a real operation · explicit confirmation',
  Lixeira: 'Trash',
  'Maior tamanho': 'Largest size',
  'Nenhum projeto analisado': 'No projects analyzed',
  'Nenhum resultado': 'No results',
  'Nenhuma escrita em arquivos': 'No files written',
  'Nenhuma porcentagem sem total conhecido.':
    'No percentage without a known total.',
  Nome: 'Name',
  'Não medido': 'Not measured',
  'O que pode mudar': 'What may change',
  'O que será preservado': 'What will be preserved',
  'Operações reais': 'Real operations',
  'Ordenar itens': 'Sort items',
  'Permissão negada. Este item não foi processado.':
    'Permission denied. This item was not processed.',
  Plano: 'Plan',
  'Por que apareceu': 'Why it appeared',
  'Potencial físico': 'Physical potential',
  'Preparando o plano validado pelo executor.':
    'Preparing the executor-validated plan.',
  'em inventário': 'in inventory',
  'Custos que você deve considerar': 'Costs to consider',
  Nenhuma: 'None',
  'Ainda não há relatórios.': 'No reports yet.',
  'Cenário de demonstração': 'Demo scenario',
  'Abrir catálogo de componentes e estados': 'Open component and state catalog',
  'não medido': 'not measured',
  'Preservado em:': 'Preserved at:',
  'Preservar também é cuidar do seu espaço.':
    'Preserving is another way to care for your space.',
  'Pronto para olhar mais de perto?': 'Ready to take a closer look?',
  Protegido: 'Protected',
  'Reproduza estados sem usar arquivos reais.':
    'Reproduce states without using real files.',
  'Resultados normais': 'Normal results',
  'Rever cenário': 'Review scenario',
  Revisar: 'Review',
  'Revisar candidatos': 'Review candidates',
  'SUA SELEÇÃO': 'YOUR SELECTION',
  Secundária: 'Secondary',
  'Sem abrir mão do que é seu.': 'Without giving up what is yours.',
  'Sem resultados': 'No results',
  'Seu primeiro passo é uma análise.': 'Your first step is a scan.',
  Snapshot: 'Snapshot',
  'Tamanho lógico / alocado': 'Logical / allocated size',
  'Tamanho lógico selecionado': 'Selected logical size',
  'Tentar consultar novamente': 'Try checking again',
  'Todos os riscos': 'All risks',
  'Variação observada por volume': 'Observed change by volume',
  'Ver motivos e caminhos': 'View reasons and paths',
  'Ver todos os itens': 'View all items',
  'Você escolhe o que entra na simulação.':
    'You choose what goes into the simulation.',
  Voltar: 'Back',
  'Voltar à revisão': 'Back to review',
  'Voltar à seleção': 'Back to selection',
  'Análise pausada': 'Analysis paused',
  'Analisando pastas aprovadas': 'Analyzing approved folders',
  'Analisando exemplos locais': 'Analyzing local samples',
  Continuar: 'Continue',
  Pausar: 'Pause',
  Cancelar: 'Cancel',
  'A confirmação do sistema será exibida pelo backend antes de':
    'System confirmation will be shown by the backend before',
  'Seleção indisponível durante análise. Aguarde ou cancele.':
    'Selection unavailable during analysis. Wait or cancel.',
  'Resultado somente desta sessão. Nenhum arquivo foi alterado.':
    'Result from this session only. No files were changed.',
  'Adicionar pasta de projetos': 'Add project folder',
  'Inventário protegido de IA': 'Protected AI inventory',
  Livre: 'Free',
  Ocupado: 'Used',
  'Plano local preparado pelo executor': 'Local plan prepared by executor',
  'Processamento local': 'Local processing',
  'Sem remoção automática': 'No automatic removal',
  'Sessões e estado protegidos': 'Sessions and state are protected',
  'A confirmação do sistema será exibida pelo backend antes de qualquer alteração.':
    'System confirmation will be shown by the backend before any change.',
  'Arquivos do relatório': 'Report files',
  'Este relatório não contém linhas de arquivo.':
    'This report contains no file entries.',
  'Movido para a Lixeira': 'Moved to Trash',
  Removido: 'Removed',
  'Alterado e preservado': 'Changed and preserved',
  Falha: 'Failed',
  preservado: 'preserved',
  Cancelado: 'Cancelled',
  Quarentena: 'Quarantined',
  'bytes lógicos': 'logical bytes',
  'Sem mensagem registrada.': 'No message recorded.',
  'Preservado em': 'Preserved at',
  'Durabilidade confirmada': 'Durability confirmed',
  Sim: 'Yes',
  Não: 'No',
  Paginação: 'Pagination',
  Anterior: 'Previous',
  Próxima: 'Next',
  'Histórico local': 'Local history',
  'Relatórios salvos no dispositivo. Os caminhos exibidos vêm dos relatórios persistidos.':
    'Reports saved on this device. Displayed paths come from saved reports.',
  'Carregando…': 'Loading…',
  Atualizar: 'Refresh',
  Erro: 'Error',
  'Nenhum relatório salvo': 'No saved reports',
  'Relatórios finalizados aparecerão aqui.':
    'Completed reports will appear here.',
  'Data indisponível': 'Date unavailable',
  'Remoção permanente': 'Permanent removal',
  Concluído: 'Complete',
  Incompleto: 'Incomplete',
  arquivos: 'files',
  'bytes processados': 'bytes processed',
  'Relatório indisponível': 'Report unavailable',
  'Detalhes do relatório': 'Report details',
  'Execução cancelada.': 'Execution cancelled.',
  'Execução incompleta.': 'Execution incomplete.',
  'Carregando relatório…': 'Loading report…',
  'Selecione um relatório para ver os detalhes.':
    'Select a report to view details.',
  'Conheça os itens protegidos': 'Learn about protected items',
  'Estado Terraform, código de worktrees e sessões de IA permanecem em inventário. Nenhum desses itens pode ser selecionado.':
    'Terraform state, worktree code, and AI sessions remain in the inventory. None of these items can be selected.',
  'Esta análise antecede a última tentativa de limpeza. Analise novamente antes de selecionar novos itens.':
    'This scan predates the last cleanup attempt. Scan again before selecting new items.',
  'Este candidato exige revisão individual. Remova-o antes de selecionar outro item; confira impacto e como recriar antes de preparar a limpeza.':
    'This candidate requires individual review. Deselect it before choosing another item; check the impact and rebuild guidance before preparing cleanup.',
  'Selecionar todos': 'Select all',
  'Limpar com 1 clique': 'Clean in one click',
  'entradas ignoradas no scan e na descoberta. Symlinks internos de um item apagável não entram nesta conta. Itens incertos permanecem protegidos.':
    'entries skipped by the scan and discovery. Internal symlinks of a deletable item are not included. Uncertain items stay protected.',
  'O estado da limpeza ainda não foi confirmado. A origem, os escopos e novas análises permanecem bloqueados até a consulta responder.':
    'Cleanup status has not been confirmed. Changing the source, scopes, or starting a new scan stays blocked until the status check responds.',
  'Variações observadas não são atribuídas à limpeza. Arquivos enviados à Lixeira não são contabilizados como espaço liberado.':
    'Observed changes are not attributed to cleanup. Files sent to Trash are not counted as freed space.',
  'Lixeira e exclusão permanente não foram executadas. Não existe desfazer para este exemplo, pois nada foi modificado.':
    'Trash and permanent deletion were not performed. This sample has nothing to undo because no files were changed.',
  'O histórico de demonstração aparece depois de concluir um exemplo e dura somente nesta sessão.':
    'Demo history appears after completing an example and lasts for this session only.',
  'Mais espaço para o que importa.': 'More room for what matters.',
  'O Inodune ajuda você a entender caches e artefatos, revisar cada item e preservar o essencial.':
    'Inodune helps you understand caches and artifacts, review each item, and preserve what matters.',
  'Revisão individual obrigatória. Confira o impacto e o custo de reconstrução informados no plano antes de continuar.':
    'Individual review required. Check the impact and rebuild cost in the plan before continuing.',
  'POTENCIAL LÓGICO ESTIMADO': 'ESTIMATED LOGICAL POTENTIAL',
  'potencial lógico estimado': 'estimated logical potential',
  'Última atividade': 'Last activity',
  'A análise é somente leitura. Você aprova cada pasta no diálogo nativo; nada é removido ao iniciar.':
    'Scanning is read-only. You approve each folder in the native dialog; starting a scan removes nothing.',
  'Você está em uma demonstração. Todos os caminhos e valores são exemplos; nenhum acesso aos seus arquivos será solicitado.':
    'You are in a demonstration. Every path and value is an example; no access to your files will be requested.',
  'Escolha pastas específicas no diálogo nativo. A aprovação não inicia o scan.':
    'Choose specific folders in the native dialog. Approval does not start a scan.',
  'Escolha quais exemplos entram na análise. Nenhuma pasta real será aberta.':
    'Choose which samples to include in the scan. No real folders will be opened.',
  'O scanner Rust percorrerá somente as pastas aprovadas, sem seguir links ou mounts. Você poderá pausar, cancelar e revisar os resultados.':
    'The Rust scanner will inspect only approved folders without following links or mounts. You can pause, cancel, and review the results.',
  'O provedor de demonstração emitirá itens e estados de análise. Você poderá pausar, cancelar e revisar tudo antes de concluir um exemplo.':
    'The demo provider will show sample items and scan states. You can pause, cancel, and review everything before completing an example.',
  'Controles dos escopos': 'Scope controls',
  'Exclusões usam caminhos relativos literais. “.” exclui todo o escopo.':
    'Exclusions use literal relative paths. “.” excludes the entire scope.',
  'Remover exclusão': 'Remove exclusion',
  'Caminho relativo a excluir': 'Relative path to exclude',
  'Adicionar exclusão': 'Add exclusion',
  'Atualizando escopo…': 'Updating scope…',
  'Revogar aprovação': 'Revoke approval',
  'Builds e caches de projetos': 'Project builds and caches',
  'Código e branches preservados': 'Preserved code and branches',
  'Sessões e memória protegidas': 'Protected sessions and memory',
  'Caches do seu usuário': 'Your user caches',
  Categorias: 'Categories',
  'Análise de demonstração': 'Demo analysis',
  'Aguardando resultados completos': 'Waiting for complete results',
  'Buscar item ou caminho': 'Search items or paths',
  'Buscar item ou caminho…': 'Search items or paths…',
  'Avisos do executor': 'Executor warnings',
  'Cenário inicial': 'Starting scenario',
  'Onde você quer olhar?': 'Where would you like to look?',
  'Tudo começa pela leitura.': 'It all starts with a scan.',
  'Iniciar análise somente leitura': 'Start read-only scan',
  'Iniciar análise de exemplo': 'Start sample scan',
  'Por que este item apareceu?': 'Why did this item appear?',
  'Copiar caminho': 'Copy path',
  'Copiar caminho de exemplo': 'Copy sample path',
  'Não contabilizado separadamente': 'Not counted separately',
  'conteúdo de cache': 'cache contents',
  'conteúdo de cache com revisão individual':
    'cache contents with individual review',
  'A identidade observada pelo scanner local é conferida pelo executor antes da operação.':
    'The executor checks the identity observed by the local scanner before acting.',
  'Item / origem': 'Item / source',
  'Tamanho lógico': 'Logical size',
  Política: 'Policy',
  'Item protegido': 'Protected item',
  Selecionar: 'Select',
  'Não somado': 'Not included in total',
  'Revisão individual': 'Individual review',
  'Somente análise': 'Scan only',
  'Por que apareceu:': 'Why it appeared:',
  'Nenhum item nesta seleção': 'No items in this selection',
  'Ajuste os filtros ou escolha outro escopo de exemplo.':
    'Adjust the filters or choose another sample scope.',
  'Fechar diálogo': 'Close dialog',
  itens: 'items',
  'paginação acessível': 'accessible pagination',
  'lista virtualizada': 'virtualized list',
  'Usar rolagem virtual': 'Use virtual scrolling',
  'Usar paginação': 'Use pagination',
  'Lista virtualizada de itens; paginação disponível acima':
    'Virtualized item list; pagination is available above',
  'Páginas da lista': 'List pages',
  Página: 'Page',
  de: 'of',
  'Volumes analisados': 'Analyzed volumes',
  'capacidade do filesystem': 'filesystem capacity',
  'Sem medição': 'Not measured',
  'Espaço medido, no seu disco.': 'Measured space on your disk.',
  'Seu disco ainda não foi analisado.': 'Your disk has not been scanned yet.',
  'Somente pastas que você aprovar entram na análise.':
    'Only folders you approve are included in the scan.',
  'Ocupado ou reservado': 'Used or reserved',
  'Disponível ao usuário': 'Available to the user',
  'Escolha uma pasta para começar. Nenhum número de demonstração é usado neste modo.':
    'Choose a folder to begin. No demo values are used in this mode.',
  Volume: 'Volume',
  candidatos: 'candidates',
  'itens protegidos': 'protected items',
  'Nenhum arquivo removido pelo scan.': 'No files removed by the scan.',
  'entrada inspecionada': 'entries inspected',
  'entradas inspecionadas': 'entries inspected',
  'itens emitidos pelo provedor de demonstração':
    'items emitted by the demo provider',
  'nenhum próximo item será lido': 'no next item will be read',
  'total ainda não informado': 'total not yet reported',
  'Não foi possível concluir a análise': 'Could not complete the scan',
  'Não foi possível concluir o exemplo': 'Could not complete the sample',
  'Permissão negada neste exemplo. Nenhum arquivo foi alterado. Escolha outro cenário em Configurações.':
    'Permission denied in this example. No files were changed. Choose another scenario in Settings.',
  'itens parciais disponíveis. Nenhuma seleção automática foi mantida.':
    'partial items are available. No automatic selection was kept.',
  'Relatório com erro.': 'Report with an error.',
  'Volume de exemplo': 'Sample volume',
  'Cache do usuário (exemplo)': 'User cache (sample)',
  'Projetos de exemplo': 'Sample projects',
  'Inventário de IA (exemplo)': 'AI inventory (sample)',
  'Cache do usuário': 'User cache',
  'Atlas API': 'Atlas API',
  Orbit: 'Orbit',
  Infraestrutura: 'Infrastructure',
  'Ferramentas locais': 'Local tools',
  Miniaturas: 'Thumbnails',
  'Cache Python': 'Python cache',
  'Índice de testes vistos (pytest --new-first)':
    'Seen test index (pytest --new-first)',
  'Índice de ordem do pytest validado. A limpeza remove somente a lista conhecida de testes vistos; a opção --new-first perde a ordem anterior até o próximo teste.':
    'Verified pytest ordering index. Cleanup removes only the recognized list of previously seen tests; --new-first loses its previous ordering until the next test run.',
  'O pytest recria este índice ao rodar a suíte; --new-first não separa arquivos vistos até então.':
    'pytest recreates this index when the suite runs; --new-first cannot distinguish previously seen tests until then.',
  'O pytest recria o índice ao rodar a suíte; --new-first não separa arquivos vistos até então.':
    'pytest recreates this index when the suite runs; --new-first cannot distinguish previously seen tests until then.',
  'Código, histórico em v/cache/lastfailed, v/cache/stepwise e estado arbitrário de plugins permanecem protegidos.':
    'Code, v/cache/lastfailed history, v/cache/stepwise, and arbitrary plugin state remain protected.',
  'Build Rust': 'Rust build',
  'Providers Terraform': 'Terraform providers',
  'Módulos Terraform': 'Terraform modules',
  'Worktree de feature': 'Feature worktree',
  'Worktree principal': 'Main worktree',
  'Sessões de agentes': 'Agent sessions',
  'Memória de agentes': 'Agent memory',
  'Estado Terraform': 'Terraform state',
  'Miniaturas podem ser recriadas pelos aplicativos.':
    'Thumbnails can be recreated by apps.',
  'As próximas pastas podem abrir mais devagar.':
    'The next folders may open more slowly.',
  'Recriação automática.': 'Recreated automatically.',
  'Fotos e arquivos originais.': 'Photos and original files.',
  'Bytecode de um projeto Python de exemplo.':
    'Bytecode from a sample Python project.',
  'O interpretador compilaria novamente o bytecode.':
    'The interpreter would compile the bytecode again.',
  'Recriação ao executar Python.': 'Recreated when Python runs.',
  'Código, ambiente virtual e dados.': 'Code, virtual environment, and data.',
  'Saídas de build de um projeto Cargo de exemplo.':
    'Build outputs from a sample Cargo project.',
  'Uma nova compilação pode ser demorada.': 'A new build may take a while.',
  'Novo build; custo não medido.': 'New build; cost not measured.',
  'Cargo.toml, Cargo.lock e código.': 'Cargo.toml, Cargo.lock, and code.',
  'Inventário: elegibilidade e atividade ainda não verificadas.':
    'Inventory: eligibility and activity have not been verified.',
  'Pode exigir download e inicialização futura.':
    'May require a future download and initialization.',
  'Não autorizado nesta fase.': 'Not authorized at this stage.',
  'Estado, backend, locks e credenciais.':
    'State, backend, locks, and credentials.',
  'Inventário: formato e uso precisam de validação real.':
    'Inventory: format and use need real validation.',
  'Pode exigir reinicialização futura.': 'May require future initialization.',
  'Terraform state e configuração.': 'Terraform state and configuration.',
  'Branch com mudanças locais; idade não determina desuso.':
    'Branch with local changes; age does not mean it is unused.',
  'Código e commits podem ser irrecuperáveis.':
    'Code and commits may be unrecoverable.',
  'Não se aplica: conteúdo não regenerável.':
    'Not applicable: content cannot be regenerated.',
  'Worktree inteira e metadados Git.': 'Entire worktree and Git metadata.',
  'Worktree principal: nunca elegível.': 'Main worktree: never eligible.',
  'Contém código e outros candidatos aninhados.':
    'Contains code and other nested candidates.',
  'Repositório principal inteiro.': 'Entire main repository.',
  'Conversas podem permitir retomar trabalho.':
    'Conversations may help you resume work.',
  'Sessões não são cache descartável.': 'Sessions are not disposable cache.',
  'Não é possível garantir recriação.': 'Recreation cannot be guaranteed.',
  'Todas as sessões, JSONL e transcripts.':
    'All sessions, JSONL files, and transcripts.',
  'Memória persistente e decisões pessoais.':
    'Persistent memory and personal decisions.',
  'Remoção pode perder contexto insubstituível.':
    'Removal may lose irreplaceable context.',
  'Não regenerável.': 'Cannot be regenerated.',
  'Memória, skills, instruções e autenticação.':
    'Memory, skills, instructions, and authentication.',
  'Estado de infraestrutura, nunca um cache.':
    'Infrastructure state, never cache.',
  'Perda pode comprometer o gerenciamento da infraestrutura.':
    'Loss may compromise infrastructure management.',
  'Não regenerável com segurança.': 'Cannot be safely regenerated.',
  'Estado completo e backups.': 'Complete state and backups.',
  'Atividade não medida • exemplo': 'Activity not measured • sample',
  'Ação simulada reconhecida.': 'Sample action acknowledged.',
  'Ação secundária reconhecida.': 'Secondary action acknowledged.',
  'Remova o item em revisão antes de selecionar outro item.':
    'Remove the item under review before selecting another item.',
  'O item em revisão foi selecionado individualmente; os demais itens foram desmarcados.':
    'The item under review was selected individually; all other items were deselected.',
  'O item em revisão foi selecionado individualmente.':
    'The item under review was selected individually.',
  'Caminho copiado.': 'Path copied.',
  'Caminho de exemplo copiado.': 'Sample path copied.',
  'Não foi possível copiar. O caminho permanece disponível para seleção.':
    'Could not copy. The path remains available to select.',
  'Ação para os arquivos': 'Action for files',
  'Política do exemplo': 'Example policy',
  'Mover para a Lixeira no mesmo volume não libera espaço físico. Você precisaria esvaziá-la em outra operação.':
    'Moving files to Trash on the same volume does not free physical space. You would need to empty it in a separate operation.',
  'Exclusão permanente é irreversível. Bytes lógicos não garantem espaço físico por hardlinks, CoW, snapshots ou arquivos abertos.':
    'Permanent deletion is irreversible. Logical bytes do not guarantee physical space because of hard links, CoW, snapshots, or open files.',
  'Inclui cache com custo de recriação. Revise individualmente.':
    'Includes cache with a rebuild cost. Review each item individually.',
  'Somente caches de baixo risco com ação de limpeza suportada podem ser executados.':
    'Only low-risk caches with a supported cleanup action can be processed.',
  'Apenas exemplos de caches recriáveis selecionados.':
    'Only sample recreatable caches are selected.',
  'Preparando plano…': 'Preparing plan…',
  'Preparar limpeza segura': 'Prepare safe cleanup',
  'Revisar impacto': 'Review impact',
  'Simular impacto': 'Simulate impact',
  'Preparar limpeza': 'Prepare cleanup',
  'Como seria o descarte?': 'What would cleanup look like?',
  'A seleção gera um plano explícito. Nenhum arquivo será alterado antes da confirmação do sistema.':
    'The selection creates an explicit plan. No files will change before system confirmation.',
  'Escolha apenas para explorar a confirmação. Nenhum arquivo será movido ou apagado.':
    'Choose an option to preview the confirmation. No files will be moved or deleted.',
  'Antes · último scan': 'Before · latest scan',
  'Antes · exemplo': 'Before · sample',
  '130 GiB livres': '130 GiB free',
  'Medição por volume': 'Measurement by volume',
  'Não determinado': 'Undetermined',
  'Revisar confirmação': 'Review confirmation',
  'Confirmar envio para a Lixeira': 'Confirm sending to Trash',
  'Confirmar exclusão permanente': 'Confirm permanent deletion',
  'Enviar para a Lixeira neste exemplo?': 'Send to Trash in this example?',
  'Excluir permanentemente neste exemplo?':
    'Delete permanently in this example?',
  'Confirmar exemplo': 'Confirm example',
  'Enviar arquivos à Lixeira normalmente preserva a possibilidade de restauração e não libera espaço na mesma unidade. A confirmação nativa do backend ainda será necessária.':
    'Sending files to Trash usually preserves the option to restore them and does not free space on the same volume. Native backend confirmation will still be required.',
  'A exclusão permanente é irreversível. A confirmação nativa do backend ainda será necessária antes de executar o plano.':
    'Permanent deletion is irreversible. Native backend confirmation will still be required before the plan runs.',
  'Na mesma unidade, mover para a Lixeira não libera espaço. Uma operação real seria em geral reversível.':
    'Moving files to Trash on the same volume does not free space. A real operation would generally be reversible.',
  'Uma exclusão permanente real não pode ser desfeita. Esta demonstração não exclui arquivos.':
    'A real permanent deletion cannot be undone. This demonstration does not delete files.',
  'Revisei os caminhos e autorizo o executor a prosseguir para a confirmação nativa do sistema.':
    'I reviewed the paths and authorize the executor to continue to native system confirmation.',
  'Revisei os itens deste exemplo. Nenhum arquivo do computador será alterado.':
    'I reviewed the items in this example. No files on the computer will be changed.',
  'Prosseguir para confirmação nativa': 'Continue to native confirmation',
  'Proteção é uma decisão explícita.': 'Protection is an explicit decision.',
  'Idade não é evidência de desuso.': 'Age is not evidence of disuse.',
  'Sua memória não é descartável.': 'Your memory is not disposable.',
  'Inventário protegido': 'Protected inventory',
  'Revisão de itens': 'Item review',
  'items displayed': 'items displayed',
  selecionados: 'selected',
  'Resultado parcial': 'Partial results',
  'Dados não concluídos': 'Incomplete data',
  'Selecionar ': 'Select ',
  'Relatório salvo em': 'Report saved to',
  'Exclusões atualizadas.': 'Exclusions updated.',
  'Aprovação revogada.': 'Approval revoked.',
  'Conclua ou pause a operação atual antes de alterar os escopos aprovados.':
    'Pause or finish the current operation before changing approved scopes.',
  'Exemplo concluído.': 'Example completed.',
  'Processando exemplos, item a item.':
    'Processing samples, one item at a time.',
  'Exemplo com itens não concluídos.': 'Example with incomplete items.',
  'Concluído no exemplo': 'Completed in the example',
  'Falhou no exemplo': 'Failed in the example',
  'Pulado · item alterado': 'Skipped · item changed',
  'Cancelado · não processado': 'Cancelled · not processed',
  'Potencial lógico estimado': 'Estimated logical potential',
  'Inventário · não recuperável': 'Inventory · not recoverable',
  'Motivo do bloqueio': 'Why it is blocked',
  item: 'item',
  analisados: 'analyzed',
  'no exemplo': 'in sample',
  protegidos: 'protected',
  'itens selecionados': 'items selected',
  'itens exibidos': 'items shown',
  'estimativa lógica, sem recuperação medida':
    'logical estimate, no measured recovery',
  'entradas ignoradas. Itens incertos permanecem protegidos; não há seleção automática para um scan parcial.':
    'entries skipped. Uncertain items remain protected; partial scans do not get automatic selections.',
  'grupos · tamanhos sem contêineres sobrepostos':
    'groups · sizes exclude overlapping containers',
  'tamanho lógico estimado': 'estimated logical size',
  Etapa: 'Step',
  variação: 'change',
  disponíveis: 'available',
  // 0.1.9 UX review
  'Mostrar pastas de': 'Show folders of',
  'Ocultar pastas de': 'Hide folders of',
  'Pastas de': 'Folders of',
  'Entrada em outro sistema de arquivos (ponto de montagem); não examinada':
    'Entry on another file system (mount point); not examined',
  'Recurso ocupado': 'Resource busy',
  'Argumento inválido': 'Invalid argument',
  'Sem espaço no dispositivo': 'No space left on device',
  'Sistema de arquivos somente leitura': 'Read-only file system',
  'Operação não suportada': 'Operation not supported',
  'Montagem desconectada': 'Mount disconnected',
  'Cota de disco excedida': 'Disk quota exceeded',
  'linha de arquivo omitida neste relatório salvo; os totais estão completos.':
    'file line omitted from this saved report; totals are complete.',
  'linhas de arquivo omitidas neste relatório salvo; os totais estão completos.':
    'file lines omitted from this saved report; totals are complete.',
  'ação ainda não confirmada': 'action not yet confirmed',
  Limpar: 'Clean',
  'Revisar itens': 'Review items',
  Protegidos: 'Protected',
  'Detalhes técnicos': 'Technical details',
  'Espaço liberado atribuído à limpeza': 'Freed space attributed to cleanup',
  'Não atribuído': 'Not attributed',
  'Programas & Jogos': 'Apps & Games',
  'Programas e jogos': 'Apps and games',
  'Encontre, filtre e remova programas com um plano revisável.':
    'Find, filter and remove apps with a reviewable plan.',
  'Idioma, aparência e como a análise funciona.':
    'Language, appearance and how analysis works.',
  'item parcial disponível.': 'partial item available.',
  'itens parciais disponíveis.': 'partial items available.',
  'Nenhuma seleção automática foi mantida.': 'No automatic selection was kept.',
  'item selecionado': 'item selected',
  'item exibido': 'item shown',
  'Nada selecionado em': 'Nothing selected in',
  'Itens de baixo risco já vêm marcados. Itens para revisar só entram se você marcar.':
    'Low-risk items come pre-selected. Items that need review are only included if you select them.',
  'Nenhum item marcado. Marque o que deseja limpar.':
    'Nothing selected. Select what you want to clean.',
  'estimativa, sem recuperação medida': 'estimate, no measured recovery',
  'tamanho estimado': 'estimated size',
  'Tamanho processado': 'Processed size',
  'Espaço liberado confirmado': 'Confirmed freed space',
  'Arquivos no relatório': 'Files in report',
  'Volume analisado': 'Analyzed volume',
  'A demonstração só existe em builds de desenvolvimento.':
    'The demo only exists in development builds.',
  'no disco': 'on disk',
  'Enviar itens para a Lixeira?': 'Move items to Trash?',
  'Excluir itens permanentemente?': 'Delete items permanently?',
  'Inclui itens marcados por você que exigem revisão: podem pedir novo download ou recompilação.':
    'Includes items you selected that need review: they may require a new download or rebuild.',
  'Os arquivos vão para a Lixeira e podem ser restaurados. Na mesma unidade, o espaço só é liberado quando você esvaziar a Lixeira.':
    'Files go to Trash and can be restored. On the same volume, space is only freed when you empty the Trash.',
  'A exclusão permanente é irreversível.':
    'Permanent deletion is irreversible.',
  'O sistema ainda mostra uma confirmação final antes de alterar arquivos.':
    'The system still shows a final confirmation before changing files.',
  Enviar: 'Move',
  'para a Lixeira': 'to Trash',
  Excluir: 'Delete',
  'plano válido por': 'plan valid for',
  'Item do plano': 'Plan item',
  estimados: 'estimated',
  'Algumas áreas ficaram fora da análise': 'Some areas were not analyzed',
  'entrada não foi examinada.': 'entry was not examined.',
  'entradas não foram examinadas.': 'entries were not examined.',
  'Itens incertos continuam protegidos, e a limpeza só altera o que estiver no plano que você confirmar.':
    'Uncertain items stay protected, and cleanup only changes what is in the plan you confirm.',
  'Ver caminhos afetados': 'Show affected paths',
  'e mais': 'and',
  caminho: 'more path',
  caminhos: 'more paths',
  'Protegido pela política de segurança do sistema.':
    'Protected by the system safety policy.',
  'Inventário de programas e jogos': 'Apps and games inventory',
  'Escolha os programas e confirme uma vez. O plano mostra o que sai, inclusive dependências. Indícios de atividade não comprovam desuso.':
    'Choose apps and confirm once. The plan shows what is removed, including dependencies. Activity hints do not prove an app is unused.',
  'Consultando fontes…': 'Querying sources…',
  'Atualizar lista': 'Refresh list',
  'Listar programas e jogos': 'List apps and games',
  'Disponível no aplicativo desktop. Nenhum inventário real é carregado nesta visualização web.':
    'Available in the desktop app. No real inventory is loaded in this web preview.',
  'Não foi possível listar os programas': 'Could not list apps',
  'Aguardando o gerenciador de pacotes. Responda à janela do sistema, se aparecer; não repita a operação.':
    'Waiting for the package manager. Answer the system window if it appears; do not repeat the operation.',
  'Consultando inventário local…': 'Querying local inventory…',
  'Fontes consultadas': 'Queried sources',
  'Buscar programa': 'Search apps',
  'Buscar por nome, pacote ou versão…': 'Search by name, package or version…',
  'Filtrar por fonte': 'Filter by source',
  'Todas as fontes': 'All sources',
  'Filtrar por proteção': 'Filter by protection',
  'Qualquer estado': 'Any state',
  'Somente removíveis': 'Removable only',
  'Somente protegidos': 'Protected only',
  'Filtrar por tamanho': 'Filter by size',
  'Qualquer tamanho': 'Any size',
  '100 MiB ou mais': '100 MiB or more',
  '1 GiB ou mais': '1 GiB or more',
  'Tamanho desconhecido': 'Unknown size',
  'Ordenar programas': 'Sort apps',
  'Limpar filtros': 'Clear filters',
  'Versão desconhecida': 'Unknown version',
  'Último uso registrado pelo Steam': 'Last use recorded by Steam',
  'Alteração da pasta de dados Flatpak': 'Flatpak data folder change',
  confiança: 'confidence',
  baixa: 'low',
  média: 'medium',
  'Abrir no Steam': 'Open in Steam',
  Desinstalar: 'Uninstall',
  'Nenhum programa com esses filtros': 'No apps match these filters',
  'Nenhum programa encontrado': 'No apps found',
  'Ajuste a busca ou limpe os filtros.':
    'Adjust the search or clear the filters.',
  'As fontes consultadas não retornaram programas.':
    'The queried sources returned no apps.',
  'Veja o que está instalado': 'See what is installed',
  'A consulta é somente leitura: APT/RPM, Flatpak, Snap e Steam. Nada é removido sem um plano confirmado.':
    'The query is read-only: APT/RPM, Flatpak, Snap and Steam. Nothing is removed without a confirmed plan.',
  programa: 'app',
  programas: 'apps',
  'programa selecionado': 'app selected',
  'programas selecionados': 'apps selected',
  instalados: 'installed',
  'Simulando remoção…': 'Simulating removal…',
  'Abrir desinstalação no Steam?': 'Open uninstall in Steam?',
  'Confirmar desinstalação': 'Confirm uninstall',
  'O Steam mostra a própria confirmação e faz a remoção. O Inodune não confirma o resultado.':
    'Steam shows its own confirmation and performs the removal. Inodune does not confirm the result.',
  dependência: 'dependency',
  dependências: 'dependencies',
  pacote: 'package',
  pacotes: 'packages',
  'sem tamanho conhecido': 'without known size',
  Selecionados: 'Selected',
  'Dependências que também serão removidas':
    'Dependencies that will also be removed',
  'Sem autoremove nem purge. O plano vale por 2 minutos e é conferido de novo antes de executar. O sistema pode pedir sua senha; o Inodune não a recebe.':
    'No autoremove or purge. The plan is valid for 2 minutes and is checked again before running. The system may ask for your password; Inodune never receives it.',
  'Identificadores técnicos': 'Technical identifiers',
  Notas: 'Notes',
  'medido nesta análise': 'measured in this analysis',
  capacidade: 'capacity',
  totais: 'total',
  'item protegido': 'protected item',
  candidato: 'candidate',
  'Nenhum item marcado. Marque itens na lista abaixo.':
    'Nothing selected. Select items in the list below.',
  'e outras': 'and others',
  'nunca entram na limpeza': 'never included in cleanup',
  pasta: 'folder',
  pastas: 'folders',
  'Pastas do sistema sem permissão de leitura':
    'System folders without read permission',
  'Pastas do sistema indisponíveis': 'System folders unavailable',
  'Áreas do sistema protegidas': 'Protected system areas',
  'Arquivos com hardlinks, preservados': 'Hard-linked files, preserved',
  'Links simbólicos, não seguidos': 'Symbolic links, not followed',
  'Entradas que não puderam ser lidas': 'Entries that could not be read',
  'Pastas sem permissão de acesso': 'Folders without access permission',
  'Exclusões definidas por você': 'Exclusions you defined',
  'Credenciais preservadas': 'Credentials preserved',
  'Arquivos especiais (sockets, dispositivos)':
    'Special files (sockets, devices)',
  'Itens de outros usuários': 'Items owned by other users',
  'Outros volumes ou montagens protegidas': 'Other volumes or protected mounts',
  'Volumes sem medição': 'Volumes without measurement',
  'Áreas protegidas da pasta pessoal': 'Protected home areas',
  'Tabela de montagens ilegível': 'Unreadable mount table',
  'Limite de áreas analisadas atingido': 'Analyzed area limit reached',
  'Montagens internas não puderam ser isoladas':
    'Nested mounts could not be isolated',
  'Outros motivos de segurança': 'Other safety reasons',
  'Operação não permitida': 'Operation not permitted',
  'O arquivo ou a pasta não existe mais': 'The file or folder no longer exists',
  'Erro de leitura no disco': 'Disk read error',
  'Permissão negada': 'Permission denied',
  'Um arquivo está onde se esperava uma pasta':
    'A file is where a folder was expected',
  'Uma pasta está onde se esperava um arquivo':
    'A folder is where a file was expected',
  'Nome de arquivo longo demais': 'File name too long',
  'Links simbólicos em excesso': 'Too many symbolic links',
  'Arquivo de rede desatualizado': 'Stale network file',
  'Cobertura incompleta': 'Incomplete coverage',
  'Flatpak (usuário)': 'Flatpak (user)',
  'Flatpak (sistema)': 'Flatpak (system)',
  'Autorização não concedida': 'Authorization not granted',
  'Desinstalação cancelada': 'Uninstall cancelled',
  'Outros programas dependem desta seleção':
    'Other apps depend on this selection',
  'PackageKit indisponível': 'PackageKit unavailable',
  'Gerenciador de pacotes ocupado': 'Package manager busy',
  'Pacote protegido': 'Protected package',
  'O plano expirou ou mudou': 'The plan expired or changed',
  'Resultado desconhecido': 'Unknown result',
  'Não foi possível desinstalar': 'Could not uninstall',
  'SIMULADOR DE ESPAÇO': 'SPACE BUDGET SIMULATOR',
  'LENTE DE PROJETOS': 'PROJECT LENS',
  'REGISTRO DE PROTEÇÕES': 'PROTECTION LEDGER',
  'PROGRAMAS INSTALADOS': 'INSTALLED APPS',
  PREFERÊNCIAS: 'PREFERENCES',
  HISTÓRICO: 'HISTORY',
  RELATÓRIO: 'REPORT',
  'REVISÃO DE ITENS': 'ITEM REVIEW',
  'Transação concluída pelo gerenciador de pacotes.':
    'Transaction completed by the package manager.',
  'Confirmação aberta no Steam. O inodune não confirma a remoção.':
    'Confirmation opened in Steam. Inodune does not confirm the removal.',
  arquivo: 'file',
  processados: 'processed',
  selecionado: 'selected',
};

export const language = $state<{ value: Locale }>({ value: 'pt-BR' });

export function t(text: string): string {
  return language.value === 'en-US'
    ? (catalog[text] ?? demoCatalog[text] ?? text)
    : text;
}

export function formatNumber(
  value: number,
  options?: Intl.NumberFormatOptions,
): string {
  return new Intl.NumberFormat(language.value, options).format(value);
}

export function formatDate(
  value: Date | number,
  options?: Intl.DateTimeFormatOptions,
): string {
  return new Intl.DateTimeFormat(language.value, options).format(value);
}
