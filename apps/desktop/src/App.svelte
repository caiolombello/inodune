<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    House,
    Code,
    GitBranch,
    Bot,
    HardDrive,
    History,
    Settings,
    Shield,
    ArrowUpRight,
    ArrowRight,
    Search,
    RefreshCw,
    Check,
    Folder,
    Layers,
    SlidersHorizontal,
    CircleAlert,
    X,
    Pause,
    Play,
    Copy,
    Sun,
    Moon,
    ShieldCheck,
    ChevronRight,
    FlaskConical,
    CircleDashed,
  } from '@lucide/svelte';
  import logo from '../../../assets/brand/inodune-logo.svg';
  import icon from '../../../assets/brand/inodune-icon.svg';
  import RiskBadge from './lib/RiskBadge.svelte';
  import ItemList from './lib/ItemList.svelte';
  import Modal from './lib/Modal.svelte';
  import {
    demo,
    formatBytes,
    selectDefaults,
    toggleSelection,
    simulateBudget,
    executeDemo,
    outcomeLabel,
    type Candidate,
    type Scenario,
    type ScanState,
    type Policy,
    type Outcome,
  } from './lib/model';

  type Page =
    | 'overview'
    | 'projects'
    | 'development'
    | 'git'
    | 'ai'
    | 'system'
    | 'review'
    | 'protected'
    | 'history'
    | 'settings'
    | 'catalog'
    | 'impact'
    | 'report';
  const nav = [
    { id: 'overview' as Page, label: 'Visão geral', icon: House },
    { id: 'projects' as Page, label: 'Projetos', icon: Layers },
    { id: 'development' as Page, label: 'Desenvolvimento', icon: Code },
    { id: 'git' as Page, label: 'Git Worktrees', icon: GitBranch },
    { id: 'ai' as Page, label: 'IA & Agentes', icon: Bot },
    { id: 'system' as Page, label: 'Sistema', icon: HardDrive },
  ];
  const pageNames: Record<Page, string> = {
    overview: 'Seu espaço, em perspectiva.',
    projects: 'Cada projeto tem seu espaço.',
    development: 'Construa. Revise. Recomece.',
    git: 'Código merece cuidado.',
    ai: 'Contexto que vale preservar.',
    system: 'Leveza no dia a dia.',
    review: 'Revise com tranquilidade.',
    protected: 'O essencial fica.',
    history: 'Cada decisão, às claras.',
    settings: 'Do seu jeito.',
    catalog: 'Um sistema, todos os estados.',
    impact: 'Antes de decidir, entenda.',
    report: 'O resultado, sem atalhos.',
  };
  let page = $state<Page>('overview');
  let onboarding = $state<number | null>(0);
  let scopes = $state(['demo-cache', 'demo-projects', 'demo-agents']);
  let scenario = $state<Scenario>('normal');
  let scanState = $state<ScanState>('idle');
  let candidates = $state<Candidate[]>([]);
  let selected = $state<string[]>([]);
  let query = $state('');
  let riskFilter = $state('all');
  let projectFilter = $state('');
  let sort = $state('size');
  let detail = $state<Candidate | null>(null);
  let confirmation = $state(false);
  let consent = $state(false);
  let policy = $state<Policy>('trash');
  let theme = $state('system');
  let reduced = $state(false);
  let status = $state('');
  let outcomes = $state<{ id: string; title: string; outcome: Outcome }[]>([]);
  let executing = $state(false);
  let reportAvailable = $state(false);
  let reportBytes = $state(0);
  let scanTimer: ReturnType<typeof setTimeout>;
  let executionTimer: ReturnType<typeof setTimeout>;
  let scanQueue: Candidate[] = [];
  let heading: HTMLHeadingElement;
  let executionIndex = 0;

  const budget = $derived(simulateBudget(candidates, selected, policy));
  const protectedItems = $derived(
    candidates.filter((c) => c.risk === 'protected'),
  );
  const available = $derived(candidates.filter((c) => c.risk !== 'protected'));
  const potential = $derived(
    available.reduce((n, c) => n + c.logical_bytes, 0),
  );
  const projects = $derived([...new Set(candidates.map((c) => c.project))]);
  const filtered = $derived(
    candidates
      .filter((c) => {
        const category =
          page === 'development'
            ? 'Desenvolvimento'
            : page === 'git'
              ? 'Git Worktrees'
              : page === 'ai'
                ? 'IA & Agentes'
                : page === 'system'
                  ? 'Sistema'
                  : null;
        return (
          (!category || c.category === category) &&
          (page !== 'protected' || c.risk === 'protected') &&
          (!projectFilter || c.project === projectFilter) &&
          (riskFilter === 'all' || c.risk === riskFilter) &&
          `${c.title} ${c.display_path} ${c.detector_id}`
            .toLocaleLowerCase('pt-BR')
            .includes(query.toLocaleLowerCase('pt-BR'))
        );
      })
      .sort((a, b) =>
        sort === 'size'
          ? b.logical_bytes - a.logical_bytes
          : a.title.localeCompare(b.title, 'pt-BR'),
      ),
  );
  const scanBusy = $derived(scanState === 'scanning' || scanState === 'paused');
  const categories = [
    {
      label: 'Desenvolvimento',
      icon: Code,
      page: 'development' as Page,
      caption: 'Builds e caches de projetos',
    },
    {
      label: 'Git Worktrees',
      icon: GitBranch,
      page: 'git' as Page,
      caption: 'Código e branches preservados',
    },
    {
      label: 'IA & Agentes',
      icon: Bot,
      page: 'ai' as Page,
      caption: 'Sessões e memória protegidas',
    },
    {
      label: 'Sistema',
      icon: HardDrive,
      page: 'system' as Page,
      caption: 'Caches do seu usuário',
    },
  ];

  onMount(() => {
    try {
      theme = localStorage.getItem('inodune-theme') ?? 'system';
      reduced = localStorage.getItem('inodune-motion') === 'reduce';
    } catch {
      /* Preferences are optional; no user files are accessed. */
    }
    applyPreferences();
    return () => {
      clearTimeout(scanTimer);
      clearTimeout(executionTimer);
    };
  });
  function applyPreferences() {
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.motion = reduced ? 'reduce' : 'system';
    try {
      localStorage.setItem('inodune-theme', theme);
      localStorage.setItem('inodune-motion', reduced ? 'reduce' : 'system');
    } catch {
      /* Keep in-memory preferences. */
    }
  }
  async function go(next: Page) {
    page = next;
    query = '';
    riskFilter = 'all';
    projectFilter = '';
    status = '';
    await tick();
    heading?.focus();
  }
  function chooseScope(id: string) {
    scopes = scopes.includes(id)
      ? scopes.filter((s) => s !== id)
      : [...scopes, id];
  }
  function startScan() {
    clearTimeout(scanTimer);
    candidates = [];
    selected = [];
    outcomes = [];
    reportAvailable = false;
    scanState = 'scanning';
    onboarding = null;
    page = 'overview';
    query = '';
    projectFilter = '';
    riskFilter = 'all';
    status = '';
    scanQueue =
      scenario === 'empty' || scenario === 'error'
        ? []
        : demo.candidates.filter((c) => scopes.includes(c.scope_id));
    scanTimer = setTimeout(emitScan, 420);
  }
  function emitScan() {
    if (scanState !== 'scanning') return;
    if (scenario === 'error') {
      scanState = 'error';
      return;
    }
    const next = scanQueue.shift();
    if (next) {
      candidates = [...candidates, next];
      scanTimer = setTimeout(emitScan, 220);
    } else {
      scanState = 'complete';
      selected = selectDefaults(candidates);
    }
  }
  function pauseScan() {
    clearTimeout(scanTimer);
    scanState = 'paused';
  }
  function resumeScan() {
    scanState = 'scanning';
    scanTimer = setTimeout(emitScan, 220);
  }
  function cancelScan() {
    clearTimeout(scanTimer);
    scanState = 'cancelled';
    selected = [];
  }
  function toggle(item: Candidate) {
    if (!scanBusy && !executing) selected = toggleSelection(selected, item);
  }
  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      status = 'Caminho de exemplo copiado.';
    } catch {
      status =
        'Não foi possível copiar. O caminho permanece disponível para seleção.';
    }
  }
  function startExecution() {
    if (!consent || !budget.picked.length || executing) return;
    confirmation = false;
    executing = true;
    reportAvailable = true;
    reportBytes = budget.logicalBytes;
    page = 'report';
    executionIndex = 0;
    outcomes = budget.picked.map((c) => ({
      id: c.id,
      title: c.title,
      outcome: 'pending',
    }));
    processItem();
  }
  function processItem() {
    if (!executing) return;
    if (executionIndex >= outcomes.length) {
      executing = false;
      return;
    }
    const index = executionIndex;
    outcomes = outcomes.map((o, i) =>
      i === index ? { ...o, outcome: 'processing' } : o,
    );
    executionTimer = setTimeout(() => {
      outcomes = outcomes.map((o, i) =>
        i === index ? { ...o, outcome: executeDemo(o.id, scenario, false) } : o,
      );
      executionIndex++;
      processItem();
    }, 620);
  }
  function cancelExecution() {
    // Complete the current demo event, then cancel every not-yet-started item.
    if (!executing) return;
    clearTimeout(executionTimer);
    outcomes = outcomes.map((o) =>
      o.outcome === 'processing'
        ? { ...o, outcome: executeDemo(o.id, scenario, false) }
        : o.outcome === 'pending'
          ? { ...o, outcome: 'cancelled' }
          : o,
    );
    executing = false;
  }
</script>

<svelte:head
  ><title
    >Inodune · {page === 'overview'
      ? 'Visão geral'
      : page === 'report'
        ? 'Relatório'
        : 'Seu espaço, sob seu controle'}</title
  ><link rel="icon" href={icon} /></svelte:head
>
<a class="skip-link" href="#main">Pular para conteúdo</a>
<div class="app-shell">
  <aside class="sidebar" aria-label="Navegação principal">
    <a
      class="brand"
      href="#overview"
      onclick={(e) => {
        e.preventDefault();
        void go('overview');
      }}
      ><img src={icon} alt="" /><span
        >Inodune<span class="brand-dot">.</span></span
      ></a
    >
    <div class="workspace-label">SEU ESPAÇO DE TRABALHO</div>
    <nav aria-label="Áreas do aplicativo">
      {#each nav as item (item.id)}<button
          class:active={page === item.id}
          aria-current={page === item.id ? 'page' : undefined}
          onclick={() => go(item.id)}
          ><item.icon size={19} /><span>{item.label}</span
          >{#if page === item.id}<span class="nav-marker"></span>{/if}</button
        >{/each}
      <div class="nav-divider"></div>
      <button
        class:active={page === 'protected'}
        aria-current={page === 'protected' ? 'page' : undefined}
        onclick={() => go('protected')}
        ><Shield size={19} /><span>Itens protegidos</span
        >{#if candidates.length}<span class="nav-count"
            >{protectedItems.length}</span
          >{/if}</button
      >
      <button
        class:active={page === 'history'}
        aria-current={page === 'history' ? 'page' : undefined}
        onclick={() => go('history')}
        ><History size={19} /><span>Histórico</span></button
      >
      <button
        class="mobile-settings"
        class:active={page === 'settings'}
        onclick={() => go('settings')}
        ><Settings size={18} /> Configurações</button
      >
    </nav>
    <div class="sidebar-bottom">
      <div class="local-note">
        <span class="status-dot"></span><span
          >Local por natureza<small>Sem conta. Sem telemetria.</small></span
        >
      </div>
      <button class:active={page === 'settings'} onclick={() => go('settings')}
        ><Settings size={18} /> Configurações</button
      >
      <div class="sidebar-version">
        <span>INODUNE</span><span>0.0.1 · Fase 0</span>
      </div>
    </div>
  </aside>
  <div class="main-shell">
    <header class="topbar">
      <div>
        <span class="device-mark"><HardDrive size={16} /></span> Ambiente de
        demonstração <span class="topbar-separator">/</span>
        <strong>Somente dados sintéticos</strong>
      </div>
      <div class="topbar-actions">
        <button
          class="icon-button"
          aria-label="Alternar tema"
          onclick={() => {
            theme = theme === 'dark' ? 'light' : 'dark';
            applyPreferences();
          }}
          >{#if theme === 'dark'}<Sun size={18} />{:else}<Moon
              size={18}
            />{/if}</button
        ><span class="private-label"><ShieldCheck size={15} /> 100% local</span>
      </div>
    </header>
    <main id="main" tabindex="-1">
      <div class="page-head">
        <div>
          <div class="eyebrow">
            {page === 'overview'
              ? 'VISÃO GERAL'
              : page === 'impact'
                ? 'SPACE BUDGET SIMULATOR'
                : page === 'projects'
                  ? 'PROJECT LENS'
                  : page === 'protected'
                    ? 'PROTECTION LEDGER'
                    : 'INODUNE'}
          </div>
          <h1 bind:this={heading} tabindex="-1">{pageNames[page]}</h1>
          <p>
            {page === 'overview'
              ? 'Liberte espaço. Preserve o essencial.'
              : page === 'protected'
                ? 'Veja o que preservamos e entenda cada motivo.'
                : page === 'projects'
                  ? 'Artefatos, custos e proteções no contexto de cada projeto.'
                  : page === 'impact'
                    ? 'Uma estimativa para revisar. Nenhuma operação será executada.'
                    : 'Decisões claras, com você no controle.'}
          </p>
        </div>
        {#if !['settings', 'catalog', 'impact', 'report'].includes(page)}<button
            class="button primary"
            onclick={startScan}
            disabled={scanBusy || executing || scopes.length === 0}
            ><RefreshCw size={16} />
            {scanState === 'idle'
              ? 'Analisar exemplo'
              : 'Analisar novamente'}</button
          >{/if}
      </div>
      <div class="demo-banner">
        <FlaskConical size={16} /><span
          ><strong>Protótipo navegável</strong> · Pastas e valores fictícios. Seus
          arquivos não são acessados.</span
        ><button
          onclick={() => {
            onboarding = 0;
          }}
          disabled={executing || scanBusy}
          >Conhecer o fluxo <ArrowUpRight size={14} /></button
        >
      </div>
      <div class="live-status" role="status" aria-live="polite">{status}</div>

      {#if scanBusy}
        <section
          class="scan-panel"
          aria-label="Análise de demonstração"
          role="status"
          aria-live="polite"
        >
          <div class="scan-symbol"><CircleDashed size={28} /></div>
          <div>
            <h2>
              {scanState === 'paused'
                ? 'Análise pausada'
                : 'Analisando exemplos locais'}
            </h2>
            <p>
              {candidates.length} itens emitidos pelo provedor de demonstração · {scanState ===
              'paused'
                ? 'nenhum próximo item será lido'
                : 'total ainda não informado'}
            </p>
            <span class="badge neutral">Nenhuma escrita em arquivos</span>
          </div>
          <div class="inline-actions">
            {#if scanState === 'paused'}<button
                class="button"
                onclick={resumeScan}><Play size={16} /> Continuar</button
              >{:else}<button class="button" onclick={pauseScan}
                ><Pause size={16} /> Pausar</button
              >{/if}<button class="button" onclick={cancelScan}
              ><X size={16} /> Cancelar</button
            >
          </div>
        </section>
      {:else if scanState === 'error'}
        <section class="notice error" role="alert">
          <CircleAlert size={22} />
          <div>
            <h2>Não foi possível concluir o exemplo</h2>
            <p>
              Permissão negada no cenário simulado. Nenhum arquivo foi alterado.
              Escolha outro cenário em Configurações.
            </p>
            <button class="button" onclick={() => go('settings')}
              >Rever cenário</button
            >
          </div>
        </section>
      {:else if scanState === 'cancelled'}
        <div class="notice">
          <CircleAlert size={20} />
          <div>
            <strong>Análise cancelada</strong>
            <p>
              {candidates.length} itens parciais disponíveis. Nenhuma seleção automática
              foi mantida.
            </p>
          </div>
        </div>
      {/if}

      {#if page === 'overview'}
        <section class="storage-panel" aria-labelledby="storage-title">
          <div class="volume-summary">
            <div
              class="ring"
              role="img"
              aria-label="Volume fictício: 512 GiB totais, 382 GiB ocupados e 130 GiB livres"
            >
              <div>
                <HardDrive size={19} /><strong>512 <span>GiB</span></strong
                ><small>volume de exemplo</small>
              </div>
            </div>
            <div class="storage-legend">
              <h2 id="storage-title">Um pouco mais de respiro.</h2>
              <p>O primeiro passo é entender o que ocupa espaço.</p>
              <div>
                <span><i class="legend-dot used"></i> Ocupado</span><strong
                  >382 GiB</strong
                >
              </div>
              <div>
                <span><i class="legend-dot free"></i> Livre</span><strong
                  >130 GiB</strong
                >
              </div>
              <small>Valores do corpus sintético, sem medição do disco.</small>
            </div>
          </div>
          <div class="potential-summary">
            <span class="eyebrow">POTENCIAL LÓGICO ESTIMADO</span>
            <div class="big-number">
              {scanState === 'idle' || scanState === 'error'
                ? '—'
                : formatBytes(potential)}
            </div>
            <p>
              {scanState === 'idle'
                ? 'Analise o exemplo para encontrar candidatos.'
                : `${available.length} candidatos · ${protectedItems.length} itens protegidos`}
            </p>
            <button
              class="button primary"
              onclick={() => go('review')}
              disabled={!available.length || scanBusy}
              >Revisar candidatos <ArrowRight size={17} /></button
            ><small
              ><ShieldCheck size={14} /> Nenhuma limpeza está habilitada.</small
            >
          </div>
        </section>
        <section class="category-grid" aria-label="Categorias">
          {#each categories as cat (cat.page)}
            {@const group = candidates.filter((c) => c.category === cat.label)}
            {@const eligible = group.filter((c) => c.risk !== 'protected')}
            <button class="category-card" onclick={() => go(cat.page)}
              ><div class="category-top">
                <span class="category-icon"><cat.icon size={22} /></span
                ><ArrowUpRight size={17} />
              </div>
              <h2>{cat.label}</h2>
              <div class="category-number">
                {!candidates.length
                  ? '—'
                  : formatBytes(
                      (eligible.length
                        ? eligible
                        : group.filter((c) => c.coverage_key !== null)
                      ).reduce((n, c) => n + c.logical_bytes, 0),
                    )}
              </div>
              <p>{cat.caption}</p>
              <span class="category-qualifier"
                >{eligible.length
                  ? 'Potencial lógico estimado'
                  : 'Inventário · não recuperável'}</span
              >
              <div class="category-footer">
                <span
                  >{group.length} itens {eligible.length
                    ? 'no exemplo'
                    : 'em inventário'}</span
                >{#if group.length}<span class="tiny-label"
                    >{eligible.length ? 'Revisar' : 'Protegido'}</span
                  >{/if}
              </div></button
            >
          {/each}
        </section>
        <section class="panel">
          <div class="section-head">
            <div>
              <h2>Pronto para olhar mais de perto?</h2>
              <p>Você escolhe o que entra na simulação.</p>
            </div>
            <button class="text-button" onclick={() => go('review')}
              >Ver todos os itens <ArrowRight size={16} /></button
            >
          </div>
          {#if scanState === 'idle'}<div class="empty-state">
              <Folder size={30} />
              <h3>Seu primeiro passo é uma análise.</h3>
              <p>
                Explore o fluxo com pastas de exemplo. Nada será lido do seu
                disco.
              </p>
              <button class="button" onclick={() => (onboarding = 1)}
                >Escolher escopos de exemplo</button
              >
            </div>{:else if scanBusy}<div
              class="skeleton-group"
              aria-label="Aguardando resultados completos"
            >
              <div class="skeleton"></div>
              <div class="skeleton"></div>
              <div class="skeleton"></div>
            </div>{:else}<ItemList
              items={available}
              {selected}
              ontoggle={toggle}
              ondetail={(item) => (detail = item)}
            />{/if}
        </section>
        <div class="trust-footer">
          <Shield size={17} /><span
            >Preservar também é cuidar do seu espaço.</span
          ><button class="text-button" onclick={() => go('protected')}
            >Conheça os itens protegidos <ArrowUpRight size={14} /></button
          >
        </div>
      {:else if page === 'projects'}
        <div class="section-head">
          <h2>Seus projetos de exemplo</h2>
          <span class="muted"
            >{projects.length} grupos · tamanhos sem contêineres sobrepostos</span
          >
        </div>
        <div class="project-grid">
          {#each projects as project (project)}{@const items =
              candidates.filter((c) => c.project === project)}<button
              class="project-card"
              onclick={async () => {
                await go('review');
                projectFilter = project;
              }}
              ><Folder size={24} />
              <h2>{project}</h2>
              <strong
                >{formatBytes(
                  items
                    .filter((c) => c.risk !== 'protected')
                    .reduce((n, c) => n + c.logical_bytes, 0),
                )}</strong
              >
              <p>potencial lógico estimado</p>
              <div class="category-footer">
                <span
                  >{items.length} itens · {items.filter(
                    (c) => c.risk === 'protected',
                  ).length} protegidos</span
                ><ChevronRight size={16} />
              </div></button
            >{:else}<div class="empty-state">
              <Layers size={28} />
              <h2>Nenhum projeto analisado</h2>
              <p>Inicie uma análise de exemplo para explorar o Project Lens.</p>
            </div>{/each}
        </div>
      {:else if ['development', 'git', 'ai', 'system', 'review', 'protected'].includes(page)}
        {#if page === 'protected' || page === 'git' || page === 'ai'}<div
            class="protection-note"
          >
            <Shield size={24} />
            <div>
              <h2>
                {page === 'protected'
                  ? 'Proteção é uma decisão explícita.'
                  : page === 'git'
                    ? 'Idade não é evidência de desuso.'
                    : 'Sua memória não é descartável.'}
              </h2>
              <p>
                Estado Terraform, código de worktrees e sessões de IA permanecem
                em inventário. Nenhum desses itens pode ser selecionado.
              </p>
            </div>
          </div>{/if}
        <section class="panel">
          <div class="section-head">
            <div>
              <h2>
                {projectFilter ||
                  (page === 'protected'
                    ? 'Inventário protegido'
                    : 'Revisão de itens')}
              </h2>
              <p>
                {filtered.length} itens exibidos · {selected.length} selecionados
                no total
              </p>
            </div>
            <span class="badge neutral"
              >{scanState === 'complete'
                ? 'Análise de exemplo concluída'
                : scanState === 'cancelled'
                  ? 'Resultado parcial'
                  : 'Dados não concluídos'}</span
            >
          </div>
          <div class="filters">
            <label class="search"
              ><Search size={17} /><input
                aria-label="Buscar item ou caminho"
                placeholder="Buscar item ou caminho…"
                bind:value={query}
              /></label
            ><label
              ><span class="sr-only">Filtrar por risco</span><select
                aria-label="Filtrar por risco"
                bind:value={riskFilter}
                ><option value="all">Todos os riscos</option><option value="low"
                  >Baixo risco</option
                ><option value="review">Revisar</option><option
                  value="protected">Protegido</option
                ></select
              ></label
            ><label
              ><span class="sr-only">Ordenar itens</span><select
                aria-label="Ordenar itens"
                bind:value={sort}
                ><option value="size">Maior tamanho</option><option value="name"
                  >Nome</option
                ></select
              ></label
            >
          </div>
          {#if scanBusy}<p class="inline-note">
              Seleção indisponível durante análise. Aguarde ou cancele.
            </p>{/if}
          <ItemList
            items={filtered}
            disabled={scanBusy || executing}
            {selected}
            ontoggle={toggle}
            ondetail={(item) => (detail = item)}
          />
        </section>
        {#if page !== 'protected'}<div class="selection-bar">
            <div>
              <strong>{selected.length} itens selecionados</strong><span
                >{formatBytes(budget.logicalBytes)} · estimativa lógica, sem recuperação
                medida</span
              >
            </div>
            <div class="inline-actions">
              <button
                class="button"
                onclick={() => (selected = [])}
                disabled={!selected.length || scanBusy || executing}
                >Desmarcar todos</button
              ><button
                class="button primary"
                onclick={() => go('impact')}
                disabled={!selected.length || scanBusy || executing}
                >Simular impacto <ArrowRight size={16} /></button
              >
            </div>
          </div>{/if}
      {:else if page === 'impact'}
        <div class="impact-grid">
          <section class="panel padded">
            <span class="eyebrow">SUA SELEÇÃO</span>
            <h2 class="impact-number">{formatBytes(budget.logicalBytes)}</h2>
            <p>{selected.length} itens · tamanho lógico estimado</p>
            <div class="compare-grid">
              <div>
                <span>Antes · exemplo</span><strong>130 GiB livres</strong>
              </div>
              <div>
                <span>Potencial físico</span><strong
                  >{policy === 'trash'
                    ? '0 B na mesma unidade'
                    : 'Não determinado'}</strong
                >
              </div>
              <div><span>Depois real</span><strong>Não medido</strong></div>
            </div>
            <div class="notice">
              <CircleAlert size={20} />
              <p>
                {policy === 'trash'
                  ? 'Mover para a Lixeira no mesmo volume não libera espaço físico. Você precisaria esvaziá-la em outra operação.'
                  : 'Exclusão permanente é irreversível. Bytes lógicos não garantem espaço físico por hardlinks, CoW, snapshots ou arquivos abertos.'}
              </p>
            </div>
            <h3>Custos que você deve considerar</h3>
            {#each budget.picked as item (item.id)}<div class="cost-row">
                <span>{item.title}</span><span>{item.rebuild_hint}</span>
              </div>{/each}
          </section>
          <section class="panel padded">
            <h2>Como seria o descarte?</h2>
            <p>
              Escolha apenas para explorar a confirmação. Nenhum arquivo será
              movido ou apagado.
            </p>
            <fieldset class="policy-options">
              <legend>Política da simulação</legend><label
                class:chosen={policy === 'trash'}
                ><input type="radio" bind:group={policy} value="trash" /><span
                  ><strong>Lixeira</strong><small
                    >Geralmente reversível · sem ganho imediato na mesma unidade</small
                  ></span
                ></label
              ><label class:chosen={policy === 'permanent'}
                ><input
                  type="radio"
                  bind:group={policy}
                  value="permanent"
                /><span
                  ><strong>Exclusão permanente</strong><small
                    >Irreversível em operação real · confirmação explícita</small
                  ></span
                ></label
              >
            </fieldset>
            <RiskBadge risk={budget.reviewCount ? 'review' : 'low'} />
            <p>
              {budget.reviewCount
                ? 'Inclui build com custo de reconstrução. Revise individualmente.'
                : 'Apenas exemplos de caches recriáveis selecionados.'}
            </p>
            <button
              class="button primary full"
              disabled={!selected.length || executing}
              onclick={() => {
                consent = false;
                confirmation = true;
              }}>Revisar confirmação <ArrowRight size={16} /></button
            ><button class="text-button full" onclick={() => go('review')}
              >Voltar à seleção</button
            >
          </section>
        </div>
      {:else if page === 'report' || page === 'history'}
        {#if reportAvailable}<section class="panel padded">
            <div class="section-head">
              <div>
                <span class="eyebrow"
                  >{executing
                    ? 'EM ANDAMENTO'
                    : 'RELATÓRIO DE DEMONSTRAÇÃO'}</span
                >
                <h2>
                  {executing
                    ? 'Processando exemplos, item a item.'
                    : outcomes.some((o) =>
                          ['failed', 'skipped_changed', 'cancelled'].includes(
                            o.outcome,
                          ),
                        )
                      ? 'Simulação com itens não concluídos.'
                      : 'Simulação concluída.'}
                </h2>
                <p>
                  Resultado somente desta sessão. Nenhum arquivo foi alterado.
                </p>
              </div>
              {#if executing}<button class="button" onclick={cancelExecution}
                  >Cancelar entre itens</button
                >{/if}
            </div>
            <div class="report-metrics">
              <div>
                <span>Tamanho lógico selecionado</span><strong
                  >{formatBytes(reportBytes)}</strong
                >
              </div>
              <div>
                <span>Espaço efetivamente recuperado</span><strong
                  >Não medido</strong
                >
              </div>
              <div><span>Operações reais</span><strong>Nenhuma</strong></div>
            </div>
            {#each outcomes as outcome (outcome.id)}<div class="outcome-row">
                <span>{outcome.title}</span><span
                  class:outcome-error={outcome.outcome === 'failed'}
                  class:outcome-warning={[
                    'cancelled',
                    'skipped_changed',
                  ].includes(outcome.outcome)}
                  >{#if outcome.outcome === 'simulated'}<Check
                      size={16}
                    />{:else if outcome.outcome === 'processing'}<CircleDashed
                      size={16}
                    />{:else}<CircleAlert size={16} />{/if}{outcomeLabel[
                    outcome.outcome
                  ]}</span
                >
              </div>{/each}
            <p class="inline-note">
              Lixeira e exclusão permanente não foram executadas. Não existe
              desfazer para este exemplo, pois nada foi modificado.
            </p>
            <button
              class="button"
              onclick={() => go('review')}
              disabled={executing}>Voltar à revisão</button
            >
          </section>{:else}<section class="panel empty-state">
            <History size={30} />
            <h2>Ainda não há relatórios.</h2>
            <p>
              O histórico de demonstração aparece após simular uma seleção e
              dura somente nesta sessão.
            </p>
            <button class="button" onclick={() => go('review')}
              >Ir para revisão</button
            >
          </section>{/if}
      {:else if page === 'settings'}
        <section class="panel padded settings-panel">
          <h2>Aparência e acessibilidade</h2>
          <label class="setting-row"
            ><span
              ><strong>Tema</strong><small
                >O modo automático acompanha o sistema.</small
              ></span
            ><select
              aria-label="Tema"
              bind:value={theme}
              onchange={applyPreferences}
              ><option value="system">Sistema</option><option value="light"
                >Claro</option
              ><option value="dark">Escuro</option></select
            ></label
          ><label class="setting-row"
            ><span
              ><strong>Reduzir animações</strong><small
                >A preferência do sistema também é respeitada.</small
              ></span
            ><input
              type="checkbox"
              bind:checked={reduced}
              onchange={applyPreferences}
            /></label
          >
          <div class="section-divider"></div>
          <h2>Escopos de demonstração</h2>
          <p>
            Os caminhos abaixo são fictícios. O seletor nativo de pastas será
            integrado ao scanner na Fase 1.
          </p>
          {#each demo.scopes as scope (scope.id)}<label class="scope-option"
              ><input
                type="checkbox"
                checked={scopes.includes(scope.id)}
                onchange={() => chooseScope(scope.id)}
                disabled={scanBusy || executing}
              /><span
                ><strong>{scope.label}</strong><code>{scope.display_path}</code
                ></span
              ></label
            >{/each}<label class="setting-row"
            ><span
              ><strong>Cenário de demonstração</strong><small
                >Reproduza estados sem usar arquivos reais.</small
              ></span
            ><select
              aria-label="Cenário de demonstração"
              bind:value={scenario}
              disabled={scanBusy || executing}
              ><option value="normal">Resultados normais</option><option
                value="partial">Falha e item alterado na execução</option
              ><option value="empty">Nenhum resultado</option><option
                value="error">Erro de permissão no scan</option
              ></select
            ></label
          ><button
            class="button primary"
            onclick={startScan}
            disabled={scanBusy || executing || !scopes.length}
            >Analisar cenário</button
          >
          <div class="section-divider"></div>
          <button class="text-button" onclick={() => go('catalog')}
            ><SlidersHorizontal size={17} /> Abrir catálogo de componentes e estados
            <ArrowUpRight size={14} /></button
          >
        </section>
      {:else if page === 'catalog'}
        <section class="panel padded">
          <h2>Componentes em contexto</h2>
          <div class="inline-actions">
            <button
              class="button primary"
              onclick={() => (status = 'Ação de exemplo reconhecida.')}
              >Ação primária</button
            ><button
              class="button"
              onclick={() => (status = 'Ação secundária reconhecida.')}
              >Secundária</button
            ><button class="button" disabled>Indisponível</button><RiskBadge
              risk="low"
            /><RiskBadge risk="review" /><RiskBadge risk="protected" />
          </div>
          <div class="section-divider"></div>
          <h3>Carregamento indeterminado</h3>
          <p>Nenhuma porcentagem sem total conhecido.</p>
          <div class="skeleton-group">
            <div class="skeleton"></div>
            <div class="skeleton"></div>
          </div>
          <div class="notice error">
            <CircleAlert size={20} />
            <p>Permissão negada. Este item não foi processado.</p>
          </div>
          <div class="empty-state">
            <Folder size={24} />
            <h3>Nenhum resultado</h3>
            <p>Escolha outro escopo para continuar.</p>
          </div>
          <button class="button" onclick={() => go('settings')}
            >Experimentar cenários completos</button
          >
        </section>
      {/if}
      <footer class="page-footer">
        <span>Feito para abrir espaço. Projetado para preservar.</span><span
          >Prévia de produto · dados sintéticos</span
        >
      </footer>
    </main>
  </div>
</div>

{#if onboarding !== null}
  <Modal
    title={onboarding === 0
      ? 'Seu espaço, sob seu controle.'
      : onboarding === 1
        ? 'Onde você quer olhar?'
        : 'Tudo começa pela leitura.'}
    onclose={() => (onboarding = null)}
    wide
  >
    <div class="onboarding-brand"><img src={logo} alt="Inodune" /></div>
    <div
      class="onboarding-steps"
      aria-label={'Etapa ' + (onboarding + 1) + ' de 3'}
    >
      {#each [0, 1, 2] as step (step)}<span class:current={step <= onboarding}
        ></span>{/each}<small>{onboarding + 1} de 3</small>
    </div>
    {#if onboarding === 0}<p class="onboarding-intro">
        Mais espaço para o que importa.<br />Sem abrir mão do que é seu.
      </p>
      <p>
        O Inodune ajuda você a entender caches e artefatos, revisar cada item e
        preservar o essencial.
      </p>
      <div class="onboarding-promises">
        <span><ShieldCheck size={20} /> Sem remoção automática</span><span
          ><HardDrive size={20} /> Processamento local</span
        ><span><Shield size={20} /> Sessões e estado protegidos</span>
      </div>
      <div class="notice">
        <FlaskConical size={20} />
        <p>
          Você está em um protótipo. Todos os caminhos e valores são exemplos;
          nenhum acesso aos seus arquivos será solicitado.
        </p>
      </div>
    {:else if onboarding === 1}<p>
        Escolha quais exemplos entram na análise. Nenhuma pasta real será
        aberta.
      </p>
      {#each demo.scopes as scope (scope.id)}<label class="scope-option"
          ><input
            type="checkbox"
            checked={scopes.includes(scope.id)}
            onchange={() => chooseScope(scope.id)}
          /><span
            ><strong>{scope.label}</strong><code>{scope.display_path}</code
            ></span
          ></label
        >{/each}
    {:else}<div class="onboarding-scan"><Search size={40} /></div>
      <p>
        O provedor de demonstração emitirá itens e estados de análise. Você
        poderá pausar, cancelar e revisar tudo antes de simular uma decisão.
      </p>
      <label class="setting-row"
        ><span>Cenário</span><select
          aria-label="Cenário inicial"
          bind:value={scenario}
          ><option value="normal">Resultados normais</option><option
            value="partial">Execução parcial</option
          ><option value="empty">Sem resultados</option><option value="error"
            >Erro de permissão</option
          ></select
        ></label
      >{/if}
    <div class="dialog-actions">
      <button class="text-button" onclick={() => (onboarding = null)}
        >Explorar sem analisar</button
      >
      <div class="inline-actions">
        {#if onboarding > 0}<button
            class="button"
            onclick={() => (onboarding = onboarding! - 1)}>Voltar</button
          >{/if}<button
          class="button primary"
          disabled={!scopes.length && onboarding > 0}
          onclick={() => {
            if (onboarding === 2) startScan();
            else onboarding = onboarding! + 1;
          }}
          >{onboarding === 2
            ? 'Iniciar análise de exemplo'
            : 'Continuar'}<ArrowRight size={16} /></button
        >
      </div>
    </div>
  </Modal>
{/if}
{#if detail}
  <Modal
    title="Por que este item apareceu?"
    onclose={() => {
      detail = null;
      status = '';
    }}
  >
    <RiskBadge risk={detail.risk} />
    <h3 class="detail-title">{detail.title}</h3>
    <code class="detail-path">{detail.display_path}</code><button
      class="text-button"
      onclick={() => copyPath(detail!.display_path)}
      ><Copy size={15} /> Copiar caminho de exemplo</button
    >
    <p role="status">{status}</p>
    <dl class="explanation">
      <dt>Por que apareceu</dt>
      <dd>{detail.reason}</dd>
      <dt>O que pode mudar</dt>
      <dd>{detail.impact}</dd>
      <dt>Como recriar</dt>
      <dd>{detail.rebuild_hint}</dd>
      <dt>O que será preservado</dt>
      <dd>{detail.preserved}</dd>
      <dt>Última atividade</dt>
      <dd>{detail.activity}</dd>
      <dt>Tamanho lógico / alocado</dt>
      <dd>{formatBytes(detail.logical_bytes)} / não medido</dd>
    </dl>
    <details>
      <summary>Detector e evidências técnicas</summary><code
        >{detail.detector_id} @ {detail.detector_version}</code
      >
      <ul>
        {#each detail.evidence as evidence (evidence)}<li>{evidence}</li>{/each}
      </ul>
      <p>
        Identidade de filesystem não coletada. Ação suportada: nenhuma. Fixture
        sintética, não prova de segurança de um detector.
      </p>
    </details>
    <div class="dialog-actions">
      <button class="button full" onclick={() => (detail = null)}
        >Voltar à revisão</button
      >
    </div>
  </Modal>
{/if}
{#if confirmation}
  <Modal
    title={policy === 'trash'
      ? 'Simular envio para a Lixeira?'
      : 'Simular exclusão permanente?'}
    onclose={() => (confirmation = false)}
  >
    <span class="badge neutral"
      ><FlaskConical size={14} /> Demonstração, sem operações reais</span
    >
    <p>
      {selected.length} itens · {formatBytes(budget.logicalBytes)} lógicos estimados.
    </p>
    <ul class="confirmation-list">
      {#each budget.picked as item (item.id)}<li>
          <strong>{item.title}</strong><code>{item.display_path}</code
          ><RiskBadge risk={item.risk} />
        </li>{/each}
    </ul>
    <div class="notice" class:error={policy === 'permanent'}>
      <CircleAlert size={20} />
      <p>
        {policy === 'trash'
          ? 'Na mesma unidade, mover para a Lixeira não libera espaço. Uma operação real seria em geral reversível.'
          : 'Uma exclusão permanente real não pode ser desfeita pelo Inodune. Este protótipo não exclui arquivos.'}
      </p>
    </div>
    <label class="consent"
      ><input type="checkbox" bind:checked={consent} /><span
        >Revisei os itens e entendo que esta é uma simulação sem alterações no
        disco.</span
      ></label
    >
    <div class="dialog-actions">
      <button class="button" onclick={() => (confirmation = false)}
        >Voltar</button
      ><button
        class="button primary"
        disabled={!consent}
        onclick={startExecution}>Confirmar simulação</button
      >
    </div>
  </Modal>
{/if}
