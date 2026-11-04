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
    Copy,
    Sun,
    Moon,
    ShieldCheck,
    ChevronRight,
    ChevronLeft,
    FlaskConical,
    CircleDashed,
    Package,
  } from '@lucide/svelte';
  import icon from '../../../assets/brand/inodune-icon.svg';
  import InstalledApps from './lib/InstalledApps.svelte';
  import BrandMark from './lib/BrandMark.svelte';
  import RiskBadge from './lib/RiskBadge.svelte';
  import ItemList from './lib/ItemList.svelte';
  import Modal from './lib/Modal.svelte';
  import NativeStorage from './lib/NativeStorage.svelte';
  import HistoryPanel from './lib/HistoryPanel.svelte';
  import FileOutcomes from './lib/FileOutcomes.svelte';
  import ScopeControls from './lib/ScopeControls.svelte';
  import ScanExperience from './lib/ScanExperience.svelte';
  import FeedbackSettings from './lib/FeedbackSettings.svelte';
  import FontCredits from './lib/FontCredits.svelte';
  import CoverageNotice from './lib/CoverageNotice.svelte';
  import OverviewReview from './lib/OverviewReview.svelte';
  import {
    countLabel,
    actionLabel,
    displayPath,
    tidyReason,
    warningText,
  } from './lib/present';
  import { SoundFeedback } from './lib/sound-feedback';
  import { formatNumber, language, t, type Locale } from './lib/i18n.svelte';
  import {
    native,
    isDesktop,
    type NativeScope,
    type NativeResult,
    type ScopeKind,
    type CleanupPlan,
    type CleanupExecutionReport,
    type CleanupPoll,
    type ScanActivity,
  } from './lib/backend';
  import {
    demo,
    demoAvailable,
    formatBytes,
    formatSignedBytes,
    selectOverviewDefaults,
    applyScanResult,
    isLocallyEligible,
    toggleSelection,
    trySimulateBudget,
    validateScanResult,
    measuredSum,
    cleanupBatches,
    cleanupBatchWarning,
    protectedOnlyLabel,
    executeDemo,
    outcomeLabel,
    type Candidate,
    type Scenario,
    type ScanState,
    type Policy,
    type Outcome,
  } from './lib/model';

  type Page =
    | 'apps'
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
  // Headings name the area; the subtitle says what it holds.
  const pageNames: Record<Page, string> = {
    apps: 'Programas & Jogos',
    overview: 'Visão geral',
    projects: 'Projetos',
    development: 'Desenvolvimento',
    git: 'Git Worktrees',
    ai: 'IA & Agentes',
    system: 'Sistema',
    review: 'Todos os itens',
    protected: 'Itens protegidos',
    history: 'Histórico',
    settings: 'Configurações',
    catalog: 'Catálogo de componentes',
    impact: 'Simulador de impacto',
    report: 'Relatório da limpeza',
  };
  const pageSubtitles: Record<Page, string> = {
    apps: 'Encontre, filtre e remova programas com um plano revisável.',
    overview: 'Liberte espaço. Preserve o essencial.',
    projects: 'Artefatos, custos e proteções agrupados por projeto.',
    development:
      'Builds, dependências e caches de ferramentas de desenvolvimento.',
    git: 'Worktrees e repositórios ficam em inventário: código e branches são preservados.',
    ai: 'Sessões, memória e modelos de agentes ficam protegidos.',
    system: 'Miniaturas, Lixeira e caches do seu usuário.',
    review: 'Todos os itens encontrados, com o risco e o motivo de cada um.',
    protected: 'Veja o que preservamos e entenda cada motivo.',
    history: 'Relatórios das limpezas anteriores, salvos só neste computador.',
    settings: 'Idioma, aparência e como a análise funciona.',
    catalog: 'Componentes e estados do sistema visual.',
    impact: 'Uma estimativa para revisar. Nenhuma operação será executada.',
    report: 'O resultado de cada item, com o que foi medido e o que não foi.',
  };
  let page = $state<Page>('overview');
  // Release builds have no demo corpus, so they always start in local mode.
  const demoMode = __INODUNE_DEMO__ && !isDesktop && demoAvailable;
  let onboarding = $state<number | null>(demoMode ? 0 : null);
  let source = $state<'local' | 'demo'>(demoMode ? 'demo' : 'local');
  let scopes = $state<string[]>(demoMode ? demo.scopes.map((s) => s.id) : []);
  let scanMode = $state<'automatic' | 'manual'>('automatic');
  let nativeScopes = $state<NativeScope[]>([]);
  let nativeResult = $state<NativeResult | null>(null);
  let nativeJob = $state<string | null>(null);
  let nativeProgress = $state<ScanActivity>({
    visited: 0,
    candidates: 0,
    stage: '',
  });
  let nativeError = $state('');
  // Folder approval refusals are guidance, not failures: amber, not red.
  let scopeNotice = $state('');
  let cleanupError = $state('');
  let cleanupBatchIds = $state<string[]>([]);
  let cleanupPlan = $state<CleanupPlan | null>(null);
  // Action of the plan being executed; kept after the plan object is cleared.
  let cleanupAction = $state<CleanupPlan['action'] | null>(null);
  let cleanupReport = $state<CleanupExecutionReport | null>(null);
  let cleanupProgress = $state<CleanupPoll['progress']>({
    completed_files: 0,
    total_files: 0,
    completed_items: 0,
    total_items: 0,
  });
  let cleanupJob = $state<string | null>(null);
  let preparingCleanup = $state(false);
  let localScanStale = $state(false);
  let cleanupPollFailures = $state(0);
  let cleanupStateUnknown = $state(false);
  let choosingScope = $state(false);
  let nativeTimer: ReturnType<typeof setTimeout>;
  let cleanupTimer: ReturnType<typeof setTimeout>;
  const scopeOptions = $derived(
    source === 'local' ? nativeScopes : demo.scopes,
  );
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
  let homeConfirm = $state(false);
  let consent = $state(false);
  let policy = $state<Policy>('trash');
  let theme = $state('system');
  let reduced = $state(false);
  let soundEnabled = $state(false);
  let soundMessage = $state('');
  const sound = new SoundFeedback();
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

  // A malformed selection or inventory becomes an explicit error (no estimate,
  // destructive actions blocked) instead of an exception that blanks the UI.
  const budgetResult = $derived(
    trySimulateBudget(candidates, selected, policy),
  );
  const budgetError = $derived(budgetResult.ok ? '' : budgetResult.error);
  const budgetBytesLabel = $derived(
    budgetResult.ok
      ? formatBytes(budgetResult.logicalBytes)
      : t('estimativa indisponível'),
  );
  const budget = $derived(
    budgetResult.ok
      ? budgetResult
      : {
          picked: [] as Candidate[],
          logicalBytes: 0,
          physicalPotential: null,
          measuredFreedBytes: null,
          reviewCount: 0,
        },
  );
  const protectedItems = $derived(
    candidates.filter((c) => c.risk === 'protected'),
  );
  const available = $derived(candidates.filter((c) => c.risk !== 'protected'));
  const locallyEligible = $derived(candidates.filter(isLocallyEligible));
  // Totals only add sizes that can be summed (known, non-overlapping coverage).
  const potentialInfo = $derived(measuredSum(available));
  const potential = $derived(potentialInfo.bytes);
  const batchWarning = $derived(cleanupBatchWarning(selected.length));
  // Report: counts per outcome, problems first, long runs of successes folded.
  const reportFiles = $derived(
    cleanupReport ? cleanupReport.items.flatMap((item) => item.files) : [],
  );
  const reportSummary = $derived.by(() => {
    const kinds = [
      { key: 'trashed', label: 'Na Lixeira', tone: 'done' },
      { key: 'removed', label: 'Removidos', tone: 'done' },
      {
        key: 'skipped_changed',
        label: 'Alterados e preservados',
        tone: 'warn',
      },
      { key: 'quarantined', label: 'Em quarentena', tone: 'warn' },
      { key: 'failed', label: 'Falharam', tone: 'error' },
      { key: 'cancelled', label: 'Cancelados', tone: 'neutral' },
    ];
    return kinds
      .map((kind) => {
        const files = reportFiles.filter((file) => file.status === kind.key);
        return {
          ...kind,
          count: files.length,
          bytes: files.reduce((n, file) => n + file.logical_bytes, 0),
        };
      })
      .filter((kind) => kind.count > 0);
  });
  const reportItems = $derived.by(() => {
    if (!cleanupReport) return [];
    const many = reportFiles.length > 12;
    return cleanupReport.items
      .map((item) => {
        const candidate = candidates.find((c) => c.id === item.candidate_id);
        const clean =
          item.complete &&
          item.files.every(
            (file) => file.status === 'trashed' || file.status === 'removed',
          );
        return {
          item,
          title: candidate?.title ?? item.candidate_id,
          bytes: item.files.reduce((n, file) => n + file.logical_bytes, 0),
          clean,
          collapse: many && clean,
        };
      })
      .sort((a, b) => Number(a.clean) - Number(b.clean));
  });
  // Rebuild costs per detector: five "Bytecode Python" rows become one line.
  const costGroups = $derived.by(() => {
    const groups: {
      key: string;
      title: string;
      hint: string;
      count: number;
      bytes: number;
      paths: string[];
    }[] = [];
    for (const item of budget.picked) {
      const key = `${item.detector_id}::${item.title}::${item.rebuild_hint}`;
      const found = groups.find((g) => g.key === key);
      if (found) {
        found.count += 1;
        found.bytes += item.logical_bytes;
        found.paths.push(item.display_path);
      } else
        groups.push({
          key,
          title: item.title,
          hint: item.rebuild_hint,
          count: 1,
          bytes: item.logical_bytes,
          paths: [item.display_path],
        });
    }
    return groups.sort((a, b) => b.bytes - a.bytes);
  });
  const projects = $derived([...new Set(candidates.map((c) => c.project))]);
  // Bytes per risk for composition bars: summable sizes only, each once.
  const riskMix = (items: Candidate[]) =>
    (
      [
        { key: 'low', label: 'Baixo risco' },
        { key: 'review', label: 'Revisar' },
        { key: 'protected', label: 'Protegido' },
      ] as const
    )
      .map((part) => ({
        ...part,
        bytes: measuredSum(items.filter((c) => c.risk === part.key)).bytes,
      }))
      .filter((part) => part.bytes > 0);
  // Projects ordered by what could be cleaned; protected bytes shown apart.
  const projectSummaries = $derived(
    projects
      .map((project) => {
        const items = candidates.filter((c) => c.project === project);
        const eligible = measuredSum(
          items.filter((c) => c.risk !== 'protected'),
        );
        return {
          project,
          items,
          eligibleBytes: eligible.bytes,
          eligibleUnknown: eligible.unknown,
          eligibleCount: items.filter((c) => c.risk !== 'protected').length,
          protectedCount: items.filter((c) => c.risk === 'protected').length,
          mix: riskMix(items),
        };
      })
      // Comparable estimates first; projects with no summable size go last.
      .sort(
        (a, b) =>
          Number(a.eligibleCount > 0 && !a.eligibleBytes) -
            Number(b.eligibleCount > 0 && !b.eligibleBytes) ||
          b.eligibleBytes - a.eligibleBytes ||
          a.project.localeCompare(b.project, 'pt-BR'),
      ),
  );
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
  // Sidebar status: what the last analysis established, in one line.
  const scanSummary = $derived.by(() => {
    if (executing)
      return {
        tone: 'busy',
        title: 'Limpeza em andamento',
        detail: t('Acompanhe no relatório.'),
      };
    if (scanState === 'paused')
      return {
        tone: 'idle',
        title: 'Análise pausada',
        detail: t('Nada foi alterado.'),
      };
    if (scanState === 'scanning')
      return {
        tone: 'busy',
        title: 'Analisando…',
        detail: `${formatNumber(source === 'local' ? nativeProgress.visited : candidates.length)} ${t('entradas inspecionadas')}`,
      };
    if (scanState === 'error')
      return {
        tone: 'warning',
        title: 'A análise não terminou',
        detail: t('Nada foi alterado.'),
      };
    if (scanState === 'complete' && source === 'local' && localScanStale)
      return {
        tone: 'warning',
        title: 'Análise desatualizada',
        detail: t('Analise novamente antes de uma nova limpeza.'),
      };
    if (
      scanState === 'complete' &&
      source === 'local' &&
      nativeResult &&
      (!nativeResult.complete || nativeResult.issues.length > 0)
    )
      return {
        tone: 'ok',
        title: 'Análise parcial',
        detail: `${countLabel(available.length, 'candidato', 'candidatos')} · ${t('algumas áreas ficaram fora')}`,
      };
    if (scanState === 'complete' || scanState === 'cancelled')
      return {
        tone: scanState === 'complete' ? 'ok' : 'idle',
        title:
          scanState === 'complete' ? 'Análise concluída' : 'Análise cancelada',
        detail: `${countLabel(available.length, 'candidato', 'candidatos')} · ${countLabel(protectedItems.length, 'protegido', 'protegidos')}`,
      };
    return {
      tone: 'idle',
      title: 'Nenhuma análise ainda',
      detail: t('Somente leitura até você confirmar.'),
    };
  });
  let overviewReviewOpen = $state(false);
  let overviewExpanded = $state<string[]>([]);
  let navElement = $state<HTMLElement>();
  let navAtStart = $state(true);
  let navAtEnd = $state(true);
  // The strip shows an arrow and a fade only on the side with more areas.
  function updateNavEdges() {
    const el = navElement;
    if (!el) return;
    navAtStart = el.scrollLeft <= 2;
    navAtEnd = el.scrollLeft + el.clientWidth >= el.scrollWidth - 2;
  }
  function scrollNav(direction: 1 | -1) {
    navElement?.scrollBy({
      left: direction * navElement.clientWidth * 0.7,
      behavior: reduced ? 'auto' : 'smooth',
    });
  }
  $effect(() => {
    if (!navElement) return;
    updateNavEdges();
    const observer = new ResizeObserver(updateNavEdges);
    observer.observe(navElement);
    return () => observer.disconnect();
  });
  // Narrow layouts scroll the navigation strip; keep the current area visible.
  $effect(() => {
    void page;
    const active = navElement?.querySelector<HTMLElement>(
      '[aria-current="page"]',
    );
    if (active && navElement && navElement.scrollWidth > navElement.clientWidth)
      active.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    updateNavEdges();
  });
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
      soundEnabled = localStorage.getItem('inodune-sound') === 'on';
      const savedLocale = localStorage.getItem('inodune-locale');
      if (savedLocale === 'pt-BR' || savedLocale === 'en-US')
        language.value = savedLocale;
    } catch {
      /* Preferences are optional; no user files are accessed. */
    }
    applyPreferences();
    sound.setEnabled(soundEnabled);
    const silenceInBackground = () => {
      if (document.hidden) sound.dispose();
    };
    document.addEventListener('visibilitychange', silenceInBackground);
    if (isDesktop)
      void native
        .listScopes()
        .then((known) => {
          nativeScopes = known;
          if (source === 'local') scopes = known.map((s) => s.id);
        })
        .catch((e) => {
          nativeError = String(e);
        });
    return () => {
      clearTimeout(scanTimer);
      clearTimeout(executionTimer);
      clearTimeout(nativeTimer);
      clearTimeout(cleanupTimer);
      document.removeEventListener('visibilitychange', silenceInBackground);
      sound.dispose();
    };
  });
  $effect(() => {
    document.documentElement.lang = language.value;
  });
  function setLanguage(locale: Locale) {
    language.value = locale;
    try {
      localStorage.setItem('inodune-locale', locale);
    } catch {
      /* Keep the current locale for this session. */
    }
  }
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
  async function setSoundEnabled(enabled: boolean) {
    soundEnabled = enabled;
    sound.setEnabled(enabled);
    soundMessage = '';
    try {
      localStorage.setItem('inodune-sound', enabled ? 'on' : 'off');
    } catch {
      soundMessage =
        language.value === 'en-US'
          ? 'Preference applies to this session; storage is unavailable.'
          : 'A preferência vale nesta sessão; não foi possível salvá-la.';
    }
    if (enabled && !(await sound.unlock()))
      soundMessage =
        language.value === 'en-US'
          ? 'Audio is unavailable. All feedback remains visible.'
          : 'Áudio indisponível. Todos os avisos continuam visíveis.';
  }
  async function previewSound() {
    soundMessage = '';
    if (!(await sound.unlock()) || !sound.play('preview'))
      soundMessage =
        language.value === 'en-US'
          ? 'Audio unavailable or temporarily paused. Try again.'
          : 'Áudio indisponível ou temporariamente pausado. Tente novamente.';
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
  function handleScopesChanged(nextScopes: NativeScope[]) {
    if (scanBusy || executing || preparingCleanup) return;
    nativeScopes = nextScopes;
    const validIds = new Set(nextScopes.map((scope) => scope.id));
    scopes = scopes.filter((id) => validIds.has(id));
    selected = [];
    candidates = [];
    nativeResult = null;
    nativeJob = null;
    nativeProgress = { visited: 0, candidates: 0, stage: '' };
    cleanupPlan = null;
    consent = false;
    cleanupError = '';
    nativeError = '';
    localScanStale = false;
    clearTimeout(nativeTimer);
    clearTimeout(scanTimer);
    scanState = 'idle';
  }
  async function startScan() {
    // Called from a user gesture. No audio context is created while disabled.
    void sound.unlock();
    clearTimeout(scanTimer);
    localScanStale = false;
    candidates = [];
    selected = [];
    outcomes = [];
    reportAvailable = false;
    cleanupError = '';
    cleanupPlan = null;
    cleanupReport = null;
    cleanupJob = null;
    cleanupProgress = {
      completed_files: 0,
      total_files: 0,
      completed_items: 0,
      total_items: 0,
    };
    cleanupPollFailures = 0;
    cleanupStateUnknown = false;
    nativeError = '';
    nativeResult = null;
    nativeJob = null;
    nativeProgress = { visited: 0, candidates: 0, stage: 'Iniciando' };
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
    if (source === 'local') {
      try {
        nativeJob =
          scanMode === 'automatic'
            ? await native.beginAutomaticScan()
            : await native.beginScan(scopes);
        await pollNative();
      } catch (e) {
        nativeError = String(e);
        scanState = 'error';
        sound.play('attention');
      }
    } else scanTimer = setTimeout(emitScan, 420);
    await tick();
    heading?.focus();
  }
  function emitScan() {
    if (scanState !== 'scanning') return;
    if (scenario === 'error') {
      scanState = 'error';
      sound.play('attention');
      return;
    }
    const next = scanQueue.shift();
    if (next) {
      candidates = [...candidates, next];
      scanTimer = setTimeout(emitScan, 220);
    } else {
      scanState = 'complete';
      selected = selectOverviewDefaults(candidates);
      sound.play('ready');
    }
  }
  async function addNativeScope(kind: ScopeKind) {
    choosingScope = true;
    nativeError = '';
    scopeNotice = '';
    try {
      const scope = await native.chooseScope(kind, language.value);
      if (scope) {
        nativeScopes = await native.listScopes();
        if (!scopes.includes(scope.id)) scopes = [...scopes, scope.id];
      }
    } catch (e) {
      scopeNotice = String(e);
    } finally {
      choosingScope = false;
    }
  }
  function changeSource() {
    clearTimeout(nativeTimer);
    clearTimeout(scanTimer);
    clearTimeout(cleanupTimer);
    localScanStale = false;
    candidates = [];
    selected = [];
    nativeResult = null;
    nativeJob = null;
    reportAvailable = false;
    cleanupPlan = null;
    cleanupReport = null;
    cleanupJob = null;
    cleanupError = '';
    cleanupPollFailures = 0;
    cleanupStateUnknown = false;
    outcomes = [];
    scanState = 'idle';
    scopes =
      source === 'demo'
        ? demo.scopes.map((s) => s.id)
        : nativeScopes.map((s) => s.id);
  }
  async function pollNative() {
    if (!nativeJob) return;
    const jobId = nativeJob;
    try {
      const result = await native.pollScan(jobId);
      if (nativeJob !== jobId) return;
      nativeProgress = result.progress;
      if (result.status === 'error') {
        nativeError = result.error ?? 'Falha de análise';
        scanState = 'error';
        nativeJob = null;
        sound.play('attention');
        return;
      }
      if (result.result) {
        const invalid = validateScanResult(result.result);
        if (invalid) {
          // Never apply a result the UI cannot show safely; nothing was changed.
          nativeError = `${t('O resultado da análise chegou em formato inesperado')} (${t(invalid)}). ${t('Nenhum item foi aplicado e nada foi alterado. Analise novamente.')}`;
          scanState = 'error';
          nativeJob = null;
          sound.play('attention');
          return;
        }
        nativeResult = result.result;
        const next = applyScanResult(result.result, scanMode === 'automatic');
        candidates = next.candidates;
        if (scanMode === 'automatic') {
          try {
            const discovered = await native.listScopes();
            if (nativeJob !== jobId) return;
            nativeScopes = discovered;
            scopes = discovered.map((scope) => scope.id);
          } catch (e) {
            nativeError = `A análise terminou, mas não foi possível atualizar a lista das áreas descobertas: ${String(e)}`;
          }
        }
        scanState = result.result.cancelled ? 'cancelled' : 'complete';
        nativeJob = null;
        if (!result.result.cancelled)
          sound.play(result.result.complete ? 'ready' : 'attention');
        // Low risk only. Automatic scans are rarely "complete" (system areas are
        // skipped), yet their plans are accepted; manual partial scans are not.
        selected = next.selected;
        return;
      }
      nativeTimer = setTimeout(() => void pollNative(), 150);
    } catch (e) {
      nativeError = String(e);
      scanState = 'error';
      sound.play('attention');
    }
  }
  async function pauseScan() {
    if (source === 'local' && nativeJob) {
      const jobId = nativeJob;
      try {
        await native.controlScan(jobId, 'pause');
        if (nativeJob === jobId && scanState === 'scanning')
          scanState = 'paused';
      } catch (e) {
        nativeError = String(e);
      }
      return;
    }

    clearTimeout(scanTimer);
    scanState = 'paused';
  }
  async function resumeScan() {
    void sound.unlock();
    if (source === 'local' && nativeJob) {
      const jobId = nativeJob;
      try {
        await native.controlScan(jobId, 'resume');
        if (nativeJob === jobId && scanState === 'paused')
          scanState = 'scanning';
      } catch (e) {
        nativeError = String(e);
      }
      return;
    }
    scanState = 'scanning';
    scanTimer = setTimeout(emitScan, 220);
  }
  async function cancelScan() {
    if (source === 'local' && nativeJob) {
      try {
        await native.controlScan(nativeJob, 'cancel');
        selected = [];
      } catch (e) {
        nativeError = String(e);
      }
      return;
    }
    clearTimeout(scanTimer);
    scanState = 'cancelled';
    selected = [];
  }
  async function askClean(ids: string[]) {
    if (
      !ids.length ||
      budgetError ||
      scanBusy ||
      executing ||
      preparingCleanup ||
      localScanStale
    )
      return;
    cleanupError = '';
    homeIds = ids;
    homeConfirm = true;
  }
  let homeIds = $state<string[]>([]);
  const homeItems = $derived(
    candidates.filter((item) => homeIds.includes(item.id)),
  );
  const homeBytes = $derived(
    homeItems.reduce((n, item) => n + item.logical_bytes, 0),
  );
  const isSelectable = (item: Candidate) =>
    source === 'local' ? isLocallyEligible(item) : item.risk !== 'protected';
  function setMany(ids: string[], checked: boolean) {
    if (scanBusy || executing || preparingCleanup || localScanStale) return;
    const allowed = ids.filter((id) => {
      const item = candidates.find((c) => c.id === id);
      return item && isSelectable(item);
    });
    selected = checked
      ? [...new Set([...selected, ...allowed])]
      : selected.filter((id) => !allowed.includes(id));
    status = '';
  }
  const selectedBytesOf = (items: Candidate[]) =>
    items
      .filter((item) => selected.includes(item.id))
      .reduce((n, item) => n + item.logical_bytes, 0);
  async function confirmOverviewClean() {
    homeConfirm = false;
    selected = homeIds;
    if (source === 'local') {
      await prepareLocalCleanup();
      if (!cleanupPlan) return;
      consent = true;
      await beginLocalCleanup();
      return;
    }
    outcomes = selected.map((id) => {
      const item = candidates.find((candidate) => candidate.id === id);
      return { id, title: item?.title ?? id, outcome: 'pending' as Outcome };
    });
    page = 'report';
    executing = true;
    executionIndex = 0;
    reportAvailable = true;
    processItem();
  }
  // WCAG 2.4.11: a row that receives focus must not end up under the sticky
  // selection bar (scroll-padding alone is not honoured by every engine).
  function keepFocusAboveBar(event: FocusEvent) {
    const bar = document.querySelector<HTMLElement>('.selection-bar');
    const target = event.target as HTMLElement | null;
    if (!bar || !target || bar.contains(target)) return;
    if (getComputedStyle(bar).position !== 'sticky') return;
    const overlap =
      target.getBoundingClientRect().bottom -
      bar.getBoundingClientRect().top +
      12;
    if (overlap > 0) window.scrollBy({ top: overlap, behavior: 'auto' });
  }
  function toggle(item: Candidate) {
    if (source === 'local' && !isLocallyEligible(item)) return;
    if (!scanBusy && !executing && !preparingCleanup && !localScanStale) {
      selected = toggleSelection(selected, item);
      status = '';
    }
  }
  function selectAllDeletable() {
    if (scanBusy || executing || preparingCleanup || localScanStale) return;
    selected = locallyEligible.map((item) => item.id);
    status = '';
  }
  async function cleanSafe() {
    if (
      source !== 'local' ||
      !nativeResult?.scan_id ||
      budgetError ||
      scanBusy ||
      executing ||
      preparingCleanup ||
      localScanStale
    )
      return;
    preparingCleanup = true;
    cleanupError = '';
    try {
      cleanupPlan = await native.prepareSafeCleanup(
        nativeResult.scan_id,
        policy,
        language.value,
      );
      cleanupBatchIds = cleanupPlan.items.map((item) => item.candidate_id);
      consent = true;
      await beginLocalCleanup();
    } catch (e) {
      cleanupError = String(e);
      preparingCleanup = false;
    }
  }
  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      status =
        source === 'local' ? 'Caminho copiado.' : 'Caminho de exemplo copiado.';
    } catch {
      status =
        'Não foi possível copiar. O caminho permanece disponível para seleção.';
    }
  }
  async function startExecution() {
    if (!consent || !budgetResult.ok || !budget.picked.length || executing)
      return;
    void sound.unlock();
    if (source === 'local') {
      await beginLocalCleanup();
      return;
    }
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
    await tick();
    heading?.focus();
  }
  async function prepareLocalCleanup() {
    if (
      source !== 'local' ||
      !nativeResult ||
      nativeResult.cancelled ||
      (!nativeResult.complete && scanMode !== 'automatic') ||
      !nativeResult.scan_id ||
      !selected.length ||
      !budgetResult.ok ||
      selected.some((id) => !locallyEligible.some((item) => item.id === id)) ||
      preparingCleanup ||
      executing
    )
      return;
    preparingCleanup = true;
    cleanupError = '';
    cleanupPlan = null;
    cleanupReport = null;
    cleanupProgress = {
      completed_files: 0,
      total_files: 0,
      completed_items: 0,
      total_items: 0,
    };
    try {
      const batch = cleanupBatches(selected)[0] ?? [];
      if (!batch.length) return;
      cleanupPlan = await native.prepareCleanup(
        nativeResult.scan_id,
        batch,
        policy,
        language.value,
      );
      cleanupBatchIds = batch;
      consent = false;
      confirmation = true;
    } catch (e) {
      cleanupError = String(e);
    } finally {
      preparingCleanup = false;
    }
  }
  async function beginLocalCleanup() {
    const plan = cleanupPlan;
    if (!plan || !consent || executing || source !== 'local') return;
    confirmation = false;
    preparingCleanup = true;
    cleanupError = '';
    try {
      const jobId = await native.beginCleanup(plan.id, plan.hash);
      cleanupJob = jobId;
      cleanupAction = plan.action;
      localScanStale = true;
      cleanupPollFailures = 0;
      cleanupStateUnknown = false;
      cleanupReport = null;
      reportAvailable = true;
      executing = true;
      page = 'report';
      await tick();
      heading?.focus();
      void pollCleanup();
    } catch (e) {
      cleanupError = String(e);
      cleanupPlan = null;
      consent = false;
    } finally {
      preparingCleanup = false;
    }
  }
  async function pollCleanup() {
    if (!cleanupJob || !executing) return;
    const jobId = cleanupJob;
    try {
      const result = await native.pollCleanup(jobId);
      if (cleanupJob !== jobId) return;
      cleanupPollFailures = 0;
      cleanupStateUnknown = false;
      cleanupError = '';
      cleanupProgress = result.progress;
      if (result.status === 'error') {
        cleanupError = result.error ?? 'A execução foi interrompida.';
        executing = false;
        sound.play('attention');
        return;
      }
      if (result.status === 'done') {
        cleanupReport = result.report;
        if (!result.report)
          cleanupError =
            'O executor encerrou sem fornecer um relatório de execução.';
        executing = false;
        if (cleanupBatchIds.length) {
          const done = new Set(cleanupBatchIds);
          selected = selected.filter((id) => !done.has(id));
          cleanupBatchIds = [];
        }
        cleanupJob = null;
        cleanupPlan = null;
        sound.play(
          result.report?.complete && !result.report.warnings.length
            ? 'ready'
            : 'attention',
        );
        return;
      }
      cleanupTimer = setTimeout(() => void pollCleanup(), 180);
    } catch {
      cleanupPollFailures += 1;
      cleanupError = `Não foi possível consultar o executor (${cleanupPollFailures}/3).`;
      if (cleanupPollFailures < 3) {
        cleanupTimer = setTimeout(
          () => void pollCleanup(),
          200 * cleanupPollFailures,
        );
      } else {
        cleanupStateUnknown = true;
      }
    }
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
    if (source === 'local' && cleanupJob) {
      void native.cancelCleanup(cleanupJob).catch((e) => {
        cleanupError = String(e);
      });
      return;
    }
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
  ><title>Inodune · {t(pageNames[page])}</title><link
    rel="icon"
    href={icon}
  /></svelte:head
>
<a class="skip-link" href="#main">{t('Pular para conteúdo')}</a>
<div class="app-shell">
  <aside class="sidebar" aria-label={t('Navegação principal')}>
    <a
      class="brand"
      href="#overview"
      onclick={(e) => {
        e.preventDefault();
        void go('overview');
      }}
      ><BrandMark size={34} /><span
        >{t('Inodune')}<span class="brand-dot">.</span></span
      ></a
    >
    <div class="nav-strip" class:at-start={navAtStart} class:at-end={navAtEnd}>
      <!-- Pointer shortcut only: keyboard users reach every area with Tab. -->
      <button
        class="nav-scroll prev"
        tabindex="-1"
        aria-hidden="true"
        onclick={() => scrollNav(-1)}><ChevronLeft size={16} /></button
      >
      <nav
        aria-label={t('Áreas do aplicativo')}
        bind:this={navElement}
        onscroll={updateNavEdges}
      >
        {#each nav as item (item.id)}<button
            class:active={page === item.id}
            aria-current={page === item.id ? 'page' : undefined}
            onclick={() => go(item.id)}
            ><item.icon size={19} /><span>{t(item.label)}</span
            >{#if page === item.id}<span class="nav-marker"></span>{/if}</button
          >{/each}
        <div class="nav-divider" role="separator"></div>
        <button
          class:active={page === 'apps'}
          aria-current={page === 'apps' ? 'page' : undefined}
          onclick={() => go('apps')}
          ><Package size={19} /><span>{t('Programas & Jogos')}</span
          >{#if page === 'apps'}<span class="nav-marker"></span>{/if}</button
        >
        <button
          class:active={page === 'protected'}
          aria-current={page === 'protected' ? 'page' : undefined}
          onclick={() => go('protected')}
          ><Shield size={19} /><span>{t('Itens protegidos')}</span
          >{#if candidates.length}<span class="sr-only">: </span><span
              class="nav-count">{formatNumber(protectedItems.length)}</span
            >{/if}</button
        >
        <button
          class:active={page === 'history'}
          aria-current={page === 'history' ? 'page' : undefined}
          onclick={() => go('history')}
          ><History size={19} /><span>{t('Histórico')}</span></button
        >
        <button
          class="mobile-settings"
          class:active={page === 'settings'}
          onclick={() => go('settings')}
          ><Settings size={18} /> {t('Configurações')}</button
        >
      </nav>
      <button
        class="nav-scroll next"
        tabindex="-1"
        aria-hidden="true"
        onclick={() => scrollNav(1)}><ChevronRight size={16} /></button
      >
    </div>
    <div class="sidebar-bottom">
      <div class="scan-state" data-state={scanSummary.tone}>
        <span class="status-dot" aria-hidden="true"></span><span
          ><strong>{t(scanSummary.title)}</strong><small
            >{scanSummary.detail}</small
          ></span
        >
      </div>
      <button
        class:active={page === 'settings'}
        aria-current={page === 'settings' ? 'page' : undefined}
        onclick={() => go('settings')}
        ><Settings size={18} /> {t('Configurações')}</button
      >
    </div>
  </aside>
  <div class="main-shell">
    <header class="topbar">
      <div>
        <span class="device-mark"><HardDrive size={16} /></span>
        <span class="device-name"
          >{t(
            source === 'local' ? 'Meu computador' : 'Ambiente de demonstração',
          )}</span
        >
        <span class="topbar-separator" aria-hidden="true">/</span>
        <strong>{t(pageNames[page])}</strong>
      </div>
      <div class="topbar-actions">
        {#if reportAvailable && page !== 'report'}
          <button class="text-button" onclick={() => go('report')}>
            {t(executing ? 'Acompanhar limpeza' : 'Ver último relatório')}
          </button>
        {/if}
        <button
          class="icon-button"
          aria-label={t('Alternar tema')}
          onclick={() => {
            theme = theme === 'dark' ? 'light' : 'dark';
            applyPreferences();
          }}
          >{#if theme === 'dark'}<Sun size={18} />{:else}<Moon
              size={18}
            />{/if}</button
        ><span
          class="private-label"
          title={t('Sem conta. Sem telemetria. Nada sai deste computador.')}
          ><ShieldCheck size={15} />
          {t('100% local')}<span class="sr-only"
            >{t('Sem conta. Sem telemetria.')}</span
          ></span
        >
      </div>
    </header>
    <main id="main" tabindex="-1">
      <div class="page-head">
        <div>
          <h1 bind:this={heading} tabindex="-1">{t(pageNames[page])}</h1>
          <p>{t(pageSubtitles[page])}</p>
        </div>
        {#if !['apps', 'settings', 'catalog', 'impact', 'report'].includes(page)}<button
            class="button primary"
            onclick={startScan}
            disabled={scanBusy ||
              executing ||
              preparingCleanup ||
              (source === 'local' &&
                scanMode === 'manual' &&
                scopes.length === 0)}
            ><RefreshCw size={16} />
            {t(
              scanState === 'idle'
                ? source === 'local'
                  ? scanMode === 'automatic'
                    ? 'Analisar computador'
                    : 'Analisar pastas'
                  : 'Analisar exemplo'
                : 'Analisar novamente',
            )}</button
          >{/if}
      </div>
      {#if page !== 'apps'}
        {#if __INODUNE_DEMO__ && source === 'demo'}<div class="demo-banner">
            <FlaskConical size={16} /><span
              ><strong>{t('Protótipo navegável')}</strong>
              · {t(
                'Pastas e valores fictícios. Seus arquivos não são acessados.',
              )}</span
            ><button
              onclick={() => {
                onboarding = 0;
              }}
              disabled={executing || scanBusy || preparingCleanup}
              >{t('Conhecer o fluxo')} <ArrowUpRight size={14} /></button
            >
          </div>{/if}
        <div class="live-status" role="status" aria-live="polite">
          {t(status)}
        </div>

        {#if scanBusy}
          <ScanExperience
            paused={scanState === 'paused'}
            title={t(
              scanState === 'paused'
                ? 'Análise pausada'
                : source === 'local'
                  ? scanMode === 'automatic'
                    ? 'Analisando volumes locais'
                    : 'Analisando pastas aprovadas'
                  : 'Analisando exemplos locais',
            )}
            progress={source === 'local'
              ? nativeProgress
              : {
                  visited: candidates.length,
                  candidates: candidates.length,
                  stage: 'Demonstração',
                }}
            findings={source === 'local'
              ? (nativeProgress.recent_findings ?? [])
              : candidates.slice(-4)}
            demo={source === 'demo'}
            {reduced}
            {soundEnabled}
            onpause={() => void pauseScan()}
            onresume={() => void resumeScan()}
            oncancel={() => void cancelScan()}
            onreduce={() => {
              reduced = !reduced;
              applyPreferences();
            }}
            onsound={() => void setSoundEnabled(!soundEnabled)}
          />
        {:else if scanState === 'error'}
          <section class="notice error" role="alert">
            <CircleAlert size={22} />
            <div>
              <h2>
                {t(
                  source === 'local'
                    ? 'Não foi possível concluir a análise'
                    : 'Não foi possível concluir o exemplo',
                )}
              </h2>
              <p>
                {source === 'local'
                  ? nativeError
                  : t(
                      'Permissão negada neste exemplo. Nenhum arquivo foi alterado. Escolha outro cenário em Configurações.',
                    )}
              </p>
              <button class="button" onclick={() => go('settings')}
                >{t('Rever cenário')}</button
              >
            </div>
          </section>
        {:else if scanState === 'cancelled'}
          <div class="notice">
            <CircleAlert size={20} />
            <div>
              <strong>{t('Análise cancelada')}</strong>
              <p>
                {countLabel(
                  candidates.length,
                  'item parcial disponível.',
                  'itens parciais disponíveis.',
                )}
                {t('Nenhuma seleção automática foi mantida.')}
              </p>
            </div>
          </div>
        {/if}

        {#if nativeError && scanState !== 'error'}<div
            class="notice error"
            role="alert"
          >
            <CircleAlert size={20} />
            <p>{t(nativeError)}</p>
          </div>{/if}
        {#if budgetError && !scanBusy}<div class="notice error" role="alert">
            <CircleAlert size={20} />
            <p>
              {t(
                'Não foi possível estimar a seleção. Nenhuma ação foi executada e a limpeza fica bloqueada até uma nova análise.',
              )}
              <small>{t(budgetError)}</small>
            </p>
          </div>{/if}
      {/if}
      {#if page === 'apps'}
        <InstalledApps />
      {:else if page === 'overview' && !scanBusy}
        {#if cleanupError}<div class="notice error" role="alert">
            <CircleAlert size={20} />
            <p>{t(cleanupError)}</p>
          </div>{/if}
        {#if source === 'local'}<NativeStorage
            automatic={scanMode === 'automatic'}
            volumes={nativeResult?.volumes ?? []}
            {potential}
            count={available.length}
            protectedCount={protectedItems.length}
            busy={scanBusy}
            scanStatus={scanState}
            onclean={() => askClean(selected)}
            selectedCount={selected.length}
            selectedLabel={budgetBytesLabel}
            {policy}
            partial={!!nativeResult &&
              (!nativeResult.complete || nativeResult.issues.length > 0)}
            potentialUncertain={potentialInfo.unknown > 0}
            onprotected={() => go('protected')}
            cleanLabel={selected.length
              ? `${t('Limpar')} ${countLabel(selected.length, 'item', 'itens')} · ${budgetBytesLabel}`
              : t('Limpar')}
          />
          {#if !scanBusy && nativeResult && (!nativeResult.complete || nativeResult.issues.length)}<CoverageNotice
              result={nativeResult}
            />{/if}{:else if __INODUNE_DEMO__}
          <section class="storage-panel" aria-labelledby="storage-title">
            <div class="volume-summary">
              <div
                class="ring"
                role="img"
                aria-label={t(
                  'Volume fictício: 512 GiB totais, 382 GiB ocupados e 130 GiB livres',
                )}
              >
                <div>
                  <HardDrive size={19} /><strong
                    >512 <span>{t('GiB')}</span></strong
                  ><small>{t('volume de exemplo')}</small>
                </div>
              </div>
              <div class="storage-legend">
                <h2 id="storage-title">{t('Um pouco mais de respiro.')}</h2>
                <p>{t('O primeiro passo é entender o que ocupa espaço.')}</p>
                <div>
                  <span><i class="legend-dot used"></i> {t('Ocupado')}</span
                  ><strong>{t('382 GiB')}</strong>
                </div>
                <div>
                  <span><i class="legend-dot free"></i> {t('Livre')}</span
                  ><strong>{t('130 GiB')}</strong>
                </div>
                <small
                  >{t(
                    'Valores do corpus sintético, sem medição do disco.',
                  )}</small
                >
              </div>
            </div>
            <div class="potential-summary">
              <span class="eyebrow">{t('POTENCIAL LÓGICO ESTIMADO')}</span>
              <div class="big-number">
                {scanState === 'idle' || scanState === 'error'
                  ? '—'
                  : formatBytes(potential)}
              </div>
              <p>
                {scanState === 'idle'
                  ? t('Analise o exemplo para encontrar candidatos.')
                  : `${formatNumber(available.length)} ${t('candidatos')} · ${formatNumber(protectedItems.length)} ${t('itens protegidos')}`}
              </p>
              <button
                class="button primary"
                onclick={() => go('review')}
                disabled={!available.length || scanBusy || preparingCleanup}
                >{t('Revisar candidatos')} <ArrowRight size={17} /></button
              ><small
                ><ShieldCheck size={14} />
                {t('Limpeza real indisponível na demonstração.')}</small
              >
            </div>
          </section>
        {/if}
        <section class="category-grid" aria-label={t('Categorias')}>
          {#each categories as cat (cat.page)}
            {@const group = candidates.filter((c) => c.category === cat.label)}
            {@const eligible = group.filter((c) => c.risk !== 'protected')}
            {@const measured = eligible.length
              ? eligible
              : group.filter((c) => c.coverage_key !== null)}
            {@const uncertain =
              !eligible.length && group.some((c) => c.coverage_key === null)}
            {@const protectedOnly = group.length > 0 && eligible.length === 0}
            {@const chosen = eligible.filter((item) =>
              selected.includes(item.id),
            )}
            {@const groupMeasure = measuredSum(group)}
            {@const groupBytes = groupMeasure.bytes}
            {@const mix = riskMix(group)}
            <article class="category-card">
              <div class="category-top">
                <span class="category-icon"><cat.icon size={20} /></span>
                <h2>
                  <button
                    class="card-link"
                    aria-label={`${t('Abrir')} ${t(cat.label)}`}
                    onclick={() => go(cat.page)}
                    >{t(cat.label)}<ArrowUpRight size={16} /></button
                  >
                </h2>
              </div>
              <div
                class="category-number"
                class:protected-summary={protectedOnly}
              >
                {protectedOnly
                  ? groupBytes
                    ? (groupMeasure.unknown ? '≥ ' : '') +
                      formatBytes(groupBytes)
                    : t('Não medido')
                  : t(
                      !candidates.length || !group.length
                        ? '—'
                        : !measured.length && group.length
                          ? 'Não medido'
                          : (uncertain ? '≥ ' : '') +
                            formatBytes(
                              measured.reduce((n, c) => n + c.logical_bytes, 0),
                            ),
                    )}
              </div>
              <span class="category-qualifier"
                >{protectedOnly
                  ? groupBytes
                    ? protectedOnlyLabel(
                        group.length,
                        (groupMeasure.unknown ? '≥ ' : '') +
                          formatBytes(groupBytes),
                      )
                    : `${countLabel(group.length, 'item protegido', 'itens protegidos')} · ${t('tamanho não somado')}`
                  : candidates.length && eligible.length
                    ? `${t('estimado')} · ${countLabel(eligible.length, 'candidato', 'candidatos')}`
                    : t(
                        candidates.length && group.length
                          ? 'Somente consulta'
                          : cat.caption,
                      )}</span
              >
              <!-- Empty means no items; unknown sizes are listed, never zero. -->
              {#if group.length && (groupBytes > 0 || !protectedOnly)}<div
                  class="category-mix"
                >
                  {#if groupBytes > 0}<div class="mix-bar" aria-hidden="true">
                      {#each mix as part (part.key)}<span
                          class={`mix-${part.key}`}
                          style:flex-grow={part.bytes}
                        ></span>{/each}
                    </div>{/if}
                  <ul class="mix-legend">
                    {#each mix as part (part.key)}<li>
                        <i class={`mix-${part.key}`}></i>{t(part.label)}
                        <strong>{formatBytes(part.bytes)}</strong>
                      </li>{/each}
                    {#if groupMeasure.unknown}<li class="mix-unknown">
                        {countLabel(
                          groupMeasure.unknown,
                          'item sem tamanho somado',
                          'itens sem tamanho somado',
                        )}
                      </li>{/if}
                  </ul>
                </div>{:else if candidates.length && !group.length}<p
                  class="category-empty"
                >
                  {t('Nada encontrado nesta categoria.')}
                </p>{:else if !candidates.length}<p class="category-empty">
                  {t(cat.caption)}
                </p>{/if}
              <div class="category-footer">
                <span
                  >{#if eligible.length}{formatNumber(chosen.length)}
                    {t('de')}
                    {countLabel(
                      group.length,
                      'item selecionado',
                      'itens selecionados',
                    )}<small>{formatBytes(selectedBytesOf(eligible))}</small
                    >{:else if group.length}{countLabel(
                      group.length,
                      'item',
                      'itens',
                    )}
                    {t('em inventário')}{/if}</span
                >{#if eligible.length}<button
                    class="button"
                    type="button"
                    aria-label={chosen.length
                      ? `${t('Limpar')} ${countLabel(chosen.length, 'item', 'itens')} ${t('de')} ${t(cat.label)}`
                      : `${t('Nada selecionado em')} ${t(cat.label)}`}
                    disabled={!chosen.length ||
                      scanBusy ||
                      executing ||
                      preparingCleanup ||
                      localScanStale}
                    onclick={() => askClean(chosen.map((item) => item.id))}
                    >{chosen.length
                      ? `${t('Limpar')} ${formatNumber(chosen.length)}`
                      : t('Limpar')}</button
                  >{:else if group.length}<span class="tiny-label"
                    ><Shield size={13} /> {t('Protegido')}</span
                  >{/if}
              </div>
            </article>
          {/each}
        </section>
        <section class="panel">
          <div class="section-head">
            <div>
              <h2>{t('Pronto para olhar mais de perto?')}</h2>
              <p>
                {t(
                  selected.length
                    ? 'Itens de baixo risco já vêm marcados. Itens para revisar só entram se você marcar.'
                    : 'Nenhum item marcado. Marque o que deseja limpar.',
                )}
              </p>
            </div>
            <button class="text-button" onclick={() => go('review')}
              >{t('Abrir a lista completa')} <ArrowRight size={16} /></button
            >
          </div>
          {#if candidates.length}<details
              class="review-drawer"
              bind:open={overviewReviewOpen}
            >
              <summary
                >{`${t('Revisar itens')} · ${formatNumber(selected.length)} ${t('de')} ${countLabel(available.length, 'item selecionado', 'itens selecionados')}`}</summary
              >
              <!-- Rendered only while open: thousands of hidden rows made the
                   overview slow, and WebKit slowest. -->
              {#if overviewReviewOpen}<OverviewReview
                  {candidates}
                  {selected}
                  {isSelectable}
                  disabled={scanBusy ||
                    executing ||
                    preparingCleanup ||
                    localScanStale}
                  onsetmany={setMany}
                  ondetail={(item) => (detail = item)}
                  bind:expanded={overviewExpanded}
                />{/if}
            </details>{/if}
          {#if scanState === 'idle'}<div class="empty-state">
              <Folder size={30} />
              <h3>{t('Seu primeiro passo é uma análise.')}</h3>
              <p>
                {t(
                  source === 'local'
                    ? scanMode === 'automatic'
                      ? 'Analise automaticamente os volumes locais acessíveis. Áreas protegidas e limitações serão relatadas; nada será removido.'
                      : 'Escolha os escopos avançados que deseja analisar.'
                    : 'Explore o fluxo com pastas de exemplo. Nada será lido do seu disco.',
                )}
              </p>
              <button
                class="button"
                onclick={() =>
                  source === 'local' && scanMode === 'automatic'
                    ? startScan()
                    : (onboarding = 1)}
                >{t(
                  source === 'local'
                    ? scanMode === 'automatic'
                      ? 'Analisar computador'
                      : 'Escolher pastas'
                    : 'Escolher escopos de exemplo',
                )}</button
              >
            </div>{:else if scanBusy}<div
              class="skeleton-group"
              aria-label={t('Aguardando resultados completos')}
            >
              <div class="skeleton"></div>
              <div class="skeleton"></div>
              <div class="skeleton"></div>
            </div>{:else if candidates.length === 0}<div class="empty-state">
              <Folder size={28} />
              <h3>{t('Nenhum item nesta seleção')}</h3>
              <p>
                {t('Ajuste os filtros ou escolha outro escopo de exemplo.')}
              </p>
            </div>{/if}
        </section>
      {:else if page === 'projects' && !scanBusy}
        <div class="section-head project-head">
          <h2>
            {t(
              source === 'local' ? 'Seus projetos' : 'Seus projetos de exemplo',
            )}
          </h2>
          <span class="muted"
            >{countLabel(projects.length, 'grupo', 'grupos')} · {t(
              'ordenados pelo que pode ser limpo',
            )}</span
          >
        </div>
        <div class="project-grid">
          {#each projectSummaries as summary (summary.project)}<button
              class="project-card"
              onclick={async () => {
                await go('review');
                projectFilter = summary.project;
              }}
              ><span class="project-title"
                ><Folder size={20} />
                <h2>{summary.project}</h2></span
              >
              <strong
                >{summary.eligibleUnknown && summary.eligibleBytes
                  ? '≥ '
                  : ''}{summary.eligibleCount &&
                !summary.eligibleBytes &&
                summary.eligibleUnknown
                  ? t('Não medido')
                  : formatBytes(summary.eligibleBytes)}</strong
              >
              <p>
                {summary.eligibleCount
                  ? `${t('estimado')} · ${countLabel(summary.eligibleCount, 'candidato', 'candidatos')}`
                  : t('Somente itens protegidos')}
              </p>
              {#if summary.mix.length}<span class="mix-bar" aria-hidden="true"
                  >{#each summary.mix as part (part.key)}<span
                      class={`mix-${part.key}`}
                      style:flex-grow={part.bytes}
                    ></span>{/each}</span
                ><span class="project-mix"
                  >{#each summary.mix as part (part.key)}<span
                      ><i class={`mix-${part.key}`}></i>{t(part.label)}
                      <strong>{formatBytes(part.bytes)}</strong></span
                    >{/each}{#if summary.eligibleUnknown}<span
                      class="mix-unknown"
                      >{countLabel(
                        summary.eligibleUnknown,
                        'item sem tamanho somado',
                        'itens sem tamanho somado',
                      )}</span
                    >{/if}</span
                >{/if}
              <span class="category-footer"
                ><span
                  >{countLabel(summary.items.length, 'item', 'itens')} · {countLabel(
                    summary.protectedCount,
                    'protegido',
                    'protegidos',
                  )}</span
                ><span class="project-cta"
                  >{t('Revisar projeto')} <ChevronRight size={16} /></span
                ></span
              ></button
            >{:else}<div class="empty-state">
              <Layers size={28} />
              <h2>{t('Nenhum projeto analisado')}</h2>
              <p>
                {t(
                  source === 'local'
                    ? 'Inicie uma análise automática para encontrar seus projetos.'
                    : 'Inicie uma análise de exemplo para explorar o Project Lens.',
                )}
              </p>
            </div>{/each}
        </div>
      {:else if !scanBusy && ['development', 'git', 'ai', 'system', 'review', 'protected'].includes(page)}
        {#if page === 'protected' || page === 'git' || page === 'ai'}<div
            class="protection-note"
          >
            <Shield size={24} />
            <div>
              <h2>
                {t(
                  page === 'protected'
                    ? 'Proteção é uma decisão explícita.'
                    : page === 'git'
                      ? 'Idade não é evidência de desuso.'
                      : 'Sua memória não é descartável.',
                )}
              </h2>
              <p>
                {t(
                  'Estado Terraform, código de worktrees e sessões de IA permanecem em inventário. Nenhum desses itens pode ser selecionado.',
                )}
              </p>
            </div>
          </div>{/if}
        <section class="panel" onfocusin={keepFocusAboveBar}>
          <div class="section-head">
            <div>
              <h2>
                {projectFilter
                  ? projectFilter
                  : t(
                      page === 'protected'
                        ? 'Inventário protegido'
                        : 'Revisão de itens',
                    )}
              </h2>
              <p>
                {formatNumber(filtered.length)}
                {t('de')}
                {countLabel(
                  candidates.filter(
                    (c) => page !== 'protected' || c.risk === 'protected',
                  ).length,
                  'item exibido',
                  'itens exibidos',
                )}{#if page !== 'protected'}
                  · {countLabel(selected.length, 'selecionado', 'selecionados')}
                  {t('no total')}{/if}
              </p>
            </div>
            <span class="badge neutral"
              >{t(
                scanState === 'complete'
                  ? source === 'local'
                    ? 'Análise encerrada'
                    : 'Análise de exemplo concluída'
                  : scanState === 'cancelled'
                    ? 'Resultado parcial'
                    : 'Dados não concluídos',
              )}</span
            >
          </div>
          <div class="filters">
            <label class="search"
              ><Search size={17} /><input
                aria-label={t('Buscar item ou caminho')}
                placeholder={t('Buscar item ou caminho…')}
                bind:value={query}
              /></label
            >{#if page !== 'protected'}<label
                ><span class="sr-only">{t('Filtrar por risco')}</span><select
                  aria-label={t('Filtrar por risco')}
                  bind:value={riskFilter}
                  ><option value="all">{t('Todos os riscos')}</option><option
                    value="low">{t('Baixo risco')}</option
                  ><option value="review">{t('Revisar')}</option><option
                    value="protected">{t('Protegido')}</option
                  ></select
                ></label
              >{/if}<label
              ><span class="sr-only">{t('Ordenar itens')}</span><select
                aria-label={t('Ordenar itens')}
                bind:value={sort}
                ><option value="size">{t('Maior tamanho')}</option><option
                  value="name">{t('Nome')}</option
                ></select
              ></label
            >
          </div>
          {#if scanBusy}<p class="inline-note">
              {t('Seleção indisponível durante análise. Aguarde ou cancele.')}
            </p>{/if}
          {#if source === 'local' && localScanStale}<p
              class="notice"
              role="status"
            >
              <CircleAlert size={18} /><span
                >{t(
                  'Esta análise antecede a última tentativa de limpeza. Analise novamente antes de selecionar novos itens.',
                )}</span
              >
            </p>{/if}
          <ItemList
            items={filtered}
            disabled={scanBusy ||
              executing ||
              preparingCleanup ||
              localScanStale}
            localMode={source === 'local'}
            {selected}
            ontoggle={toggle}
            onsetmany={setMany}
            ondetail={(item) => (detail = item)}
          />
        </section>
        {#if page !== 'protected'}<div class="selection-bar">
            <div>
              <strong
                >{countLabel(
                  selected.length,
                  'item selecionado',
                  'itens selecionados',
                )}</strong
              ><span
                >{budgetBytesLabel} · {t(
                  'estimativa, sem recuperação medida',
                )}</span
              >
            </div>
            {#if batchWarning}<p class="inline-note" role="status">
                {batchWarning}
              </p>{/if}
            {#if source === 'local'}<p
                id="safe-batch-hint"
                class="safe-batch-hint"
              >
                {t(
                  policy === 'trash'
                    ? 'Lote seguro: caches de baixo risco aceitos pelo executor, independente da seleção atual. Destino: Lixeira. A confirmação do sistema vem a seguir.'
                    : 'Lote seguro: caches de baixo risco aceitos pelo executor, independente da seleção atual. Destino: exclusão permanente. A confirmação do sistema vem a seguir.',
                )}
              </p>{/if}
            <div class="inline-actions">
              <button
                class="text-button"
                onclick={() => (selected = [])}
                disabled={!selected.length ||
                  scanBusy ||
                  executing ||
                  preparingCleanup ||
                  localScanStale}>{t('Desmarcar todos')}</button
              ><button
                class="text-button"
                onclick={selectAllDeletable}
                disabled={!locallyEligible.length ||
                  scanBusy ||
                  executing ||
                  preparingCleanup ||
                  localScanStale}>{t('Selecionar todos')}</button
              >{#if source === 'local'}<button
                  class="button"
                  aria-describedby="safe-batch-hint"
                  title={t(
                    'Independe da seleção atual; a confirmação do sistema vem a seguir.',
                  )}
                  onclick={() => void cleanSafe()}
                  disabled={scanBusy ||
                    executing ||
                    preparingCleanup ||
                    localScanStale}>{t('Limpar lote seguro da análise')}</button
                >{/if}<button
                class="button primary"
                onclick={() => go('impact')}
                disabled={!selected.length ||
                  scanBusy ||
                  executing ||
                  preparingCleanup ||
                  localScanStale}
                >{t('Revisar impacto')}
                <ArrowRight size={16} /></button
              >
            </div>
          </div>{/if}
      {:else if page === 'impact'}
        <div class="impact-grid">
          <section class="panel padded impact-selection">
            <span class="eyebrow">{t('SUA SELEÇÃO')}</span>
            <h2 class="impact-number">{budgetBytesLabel}</h2>
            <p>
              {countLabel(selected.length, 'item', 'itens')} · {t(
                'tamanho estimado',
              )}
            </p>
            <div class="compare-grid">
              <div>
                <span
                  >{t(
                    source === 'local'
                      ? 'Antes · última análise'
                      : 'Antes · exemplo',
                  )}</span
                ><strong
                  >{source === 'demo'
                    ? t('130 GiB livres')
                    : nativeResult?.volumes.length === 1
                      ? `${formatBytes(nativeResult.volumes[0].available_bytes)} ${t('disponíveis')}`
                      : t('Medição por volume')}</strong
                >
              </div>
              <div>
                <span>{t('Potencial físico')}</span><strong
                  >{t(
                    policy === 'trash'
                      ? '0 B na mesma unidade'
                      : 'Não determinado',
                  )}</strong
                >
              </div>
              <div>
                <span>{t('Depois real')}</span><strong>{t('Não medido')}</strong
                >
              </div>
            </div>
            <div class="notice">
              <CircleAlert size={20} />
              <p>
                {t(
                  policy === 'trash'
                    ? 'Mover para a Lixeira no mesmo volume não libera espaço físico. Você precisaria esvaziá-la em outra operação.'
                    : 'Exclusão permanente é irreversível. Bytes lógicos não garantem espaço físico por hardlinks, CoW, snapshots ou arquivos abertos.',
                )}
              </p>
            </div>
            <h3>{t('Custos que você deve considerar')}</h3>
            {#each costGroups as cost (cost.key)}<div class="cost-row">
                <span
                  ><strong>{t(cost.title)}</strong>{#if cost.count > 1}<small
                      >× {formatNumber(cost.count)}</small
                    >{/if}<small>{formatBytes(cost.bytes)}</small
                  >{#if cost.paths.length > 1}<details class="cost-paths">
                      <summary
                        >{t('Ver')}
                        {countLabel(
                          cost.paths.length,
                          'caminho',
                          'caminhos',
                        )}</summary
                      >
                      <ul>
                        {#each cost.paths as path, index (index)}<li>
                            <code>{displayPath(path)}</code>
                          </li>{/each}
                      </ul>
                    </details>{:else}<code class="cost-path"
                      >{displayPath(cost.paths[0])}</code
                    >{/if}</span
                ><span>{t(cost.hint)}</span>
              </div>{:else}<p class="inline-note">
                {t('Nenhum item selecionado.')}
              </p>{/each}
          </section>
          <section class="panel padded impact-action">
            <h2>
              {source === 'local'
                ? t('Preparar limpeza')
                : t('Como seria o descarte?')}
            </h2>
            <p>
              {t(
                source === 'local'
                  ? 'A seleção gera um plano explícito. Nenhum arquivo será alterado antes da confirmação do sistema.'
                  : 'Escolha apenas para explorar a confirmação. Nenhum arquivo será movido ou apagado.',
              )}
            </p>
            <fieldset class="policy-options">
              <legend
                >{t(
                  source === 'local'
                    ? 'Ação para os arquivos'
                    : 'Política do exemplo',
                )}</legend
              ><label class:chosen={policy === 'trash'}
                ><input
                  type="radio"
                  bind:group={policy}
                  value="trash"
                  disabled={preparingCleanup}
                /><span
                  ><strong>{t('Lixeira')}</strong><small
                    >{t(
                      'Geralmente reversível · sem ganho imediato na mesma unidade',
                    )}</small
                  ></span
                ></label
              ><label class:chosen={policy === 'permanent'}
                ><input
                  type="radio"
                  bind:group={policy}
                  value="permanent"
                  disabled={preparingCleanup}
                /><span
                  ><strong>{t('Exclusão permanente')}</strong><small
                    >{t(
                      'Irreversível em operação real · confirmação explícita',
                    )}</small
                  ></span
                ></label
              >
            </fieldset>
            <RiskBadge risk={budget.reviewCount ? 'review' : 'low'} />
            <p>
              {t(
                budget.reviewCount
                  ? 'Inclui cache com custo de recriação. Revise individualmente.'
                  : source === 'local'
                    ? 'Somente caches de baixo risco com ação de limpeza suportada podem ser executados.'
                    : 'Apenas exemplos de caches recriáveis selecionados.',
              )}
            </p>
            {#if batchWarning}<p class="inline-note" role="status">
                {batchWarning}
              </p>{/if}
            <button
              class="button primary full"
              disabled={!selected.length ||
                executing ||
                preparingCleanup ||
                localScanStale ||
                (source === 'local' &&
                  (!nativeResult ||
                    nativeResult.cancelled ||
                    (!nativeResult.complete && scanMode !== 'automatic') ||
                    selected.some(
                      (id) => !locallyEligible.some((item) => item.id === id),
                    )))}
              onclick={() => {
                consent = false;
                if (source === 'local') void prepareLocalCleanup();
                else confirmation = true;
              }}
              >{t(
                preparingCleanup
                  ? 'Preparando plano…'
                  : source === 'local'
                    ? 'Preparar limpeza segura'
                    : 'Revisar confirmação',
              )}
              <ArrowRight size={16} /></button
            >
            {#if cleanupError}<div class="notice error" role="alert">
                <CircleAlert size={20} />
                <p>{t(cleanupError)}</p>
              </div>{/if}
            {#if source === 'local'}<p class="inline-note">
                {t(
                  'A confirmação do sistema será exibida pelo backend antes de qualquer alteração.',
                )}
              </p>{/if}<button
              class="text-button full"
              onclick={() => go('review')}>{t('Voltar à seleção')}</button
            >
          </section>
        </div>
      {:else if page === 'history' && source === 'local'}
        <HistoryPanel
          refreshKey={cleanupReport
            ? `done-${cleanupReport.plan_id}`
            : (cleanupPlan?.id ?? '')}
        />
      {:else if page === 'report' || (page === 'history' && source === 'demo')}
        {#if reportAvailable}<section class="panel padded">
            {#if source === 'local'}
              <div class="section-head">
                <div>
                  <span class="eyebrow"
                    >{t(
                      executing ? 'EM ANDAMENTO' : 'RELATÓRIO DE LIMPEZA',
                    )}</span
                  >
                  <h2>
                    {t(
                      executing
                        ? 'Executando limpeza aprovada.'
                        : cleanupReport?.complete
                          ? 'Limpeza concluída.'
                          : cleanupReport?.cancelled
                            ? 'Limpeza cancelada.'
                            : 'Limpeza encerrada com itens pendentes.',
                    )}
                  </h2>
                  <p>
                    {t('Ação:')}
                    {actionLabel(cleanupReport?.action ?? cleanupAction)}. {t(
                      'Relatório do executor local com cada arquivo processado.',
                    )}
                  </p>
                </div>
                {#if executing}<button class="button" onclick={cancelExecution}
                    >{t('Cancelar limpeza')}</button
                  >{/if}
              </div>
              {#if cleanupError}<div class="notice error" role="alert">
                  <CircleAlert size={20} />
                  <p>{t(cleanupError)}</p>
                </div>{/if}
              {#if cleanupStateUnknown}<div class="notice error" role="alert">
                  <CircleAlert size={20} />
                  <p>
                    {t(
                      'O estado da limpeza ainda não foi confirmado. A origem, os escopos e novas análises permanecem bloqueados até a consulta responder.',
                    )}
                  </p>
                </div>
                <button
                  class="button"
                  onclick={() => {
                    cleanupPollFailures = 0;
                    cleanupStateUnknown = false;
                    void pollCleanup();
                  }}>{t('Tentar consultar novamente')}</button
                >{/if}
              {#if executing}<div
                  class="cleanup-progress"
                  role="status"
                  aria-live="polite"
                >
                  <strong
                    >{formatNumber(cleanupProgress.completed_files)}
                    {t('de')}
                    {formatNumber(cleanupProgress.total_files)}
                    {t('arquivos')}</strong
                  >
                  <span
                    >{formatNumber(cleanupProgress.completed_items)}
                    {t('de')}
                    {formatNumber(cleanupProgress.total_items)}
                    {t('itens concluídos')}</span
                  >
                </div>{/if}
              {#if cleanupReport}
                <div class="report-metrics">
                  <div>
                    <span>{t('Tamanho processado')}</span><strong
                      >{formatBytes(
                        cleanupReport.logical_bytes_processed,
                      )}</strong
                    >
                  </div>
                  <div>
                    <span>{t('Espaço liberado atribuído à limpeza')}</span
                    ><strong
                      >{cleanupReport.attributed_freed_bytes === null
                        ? t('Não atribuído')
                        : formatBytes(
                            cleanupReport.attributed_freed_bytes,
                          )}</strong
                    >
                  </div>
                  <div>
                    <span>{t('Arquivos no relatório')}</span><strong
                      >{formatNumber(
                        cleanupReport.items.reduce(
                          (n, item) => n + item.files.length,
                          0,
                        ),
                      )}</strong
                    >
                  </div>
                </div>
                {#if reportSummary.length}<ul
                    class="outcome-summary"
                    aria-label={t('Resultado por estado')}
                  >
                    {#each reportSummary as part (part.key)}<li
                        class={`outcome-${part.tone}`}
                      >
                        <strong>{formatNumber(part.count)}</strong>
                        <span>{t(part.label)}</span>
                        <small>{formatBytes(part.bytes)}</small>
                      </li>{/each}
                  </ul>{/if}
                {#if reportSummary.some((part) => part.key === 'trashed')}<p
                    class="notice report-recovery"
                  >
                    <CircleAlert size={18} /><span
                      >{t(
                        'Itens na Lixeira podem ser restaurados pela Lixeira do sistema e continuam ocupando espaço até ela ser esvaziada.',
                      )}</span
                    >
                  </p>{/if}
                {#if reportSummary.some((part) => part.key === 'removed')}<p
                    class="notice report-recovery"
                  >
                    <CircleAlert size={18} /><span
                      >{t(
                        'Arquivos removidos permanentemente não podem ser restaurados pelo Inodune. A variação de espaço aparece separada, sem atribuição.',
                      )}</span
                    >
                  </p>{/if}
                {#each reportItems as entry (entry.item.candidate_id)}
                  {#if entry.collapse}<details class="cleanup-report-item done">
                      <summary
                        ><h3>{t(entry.title)}</h3>
                        <span class="badge low"
                          ><Check size={13} /> {t('Item concluído')}</span
                        ><small
                          >{countLabel(
                            entry.item.files.length,
                            'arquivo',
                            'arquivos',
                          )} · {formatBytes(entry.bytes)}</small
                        ></summary
                      >
                      <FileOutcomes
                        files={entry.item.files}
                        label={entry.title}
                        compact
                      />
                    </details>{:else}<section
                      class="cleanup-report-item"
                      class:attention={!entry.item.complete}
                    >
                      <div class="report-item-head">
                        <h3>{t(entry.title)}</h3>
                        <span
                          class="badge {entry.item.complete ? 'low' : 'review'}"
                          >{#if entry.item.complete}<Check
                              size={13}
                            />{:else}<CircleAlert size={13} />{/if}
                          {t(
                            entry.item.complete
                              ? 'Item concluído'
                              : 'Item incompleto',
                          )}</span
                        >
                      </div>
                      <FileOutcomes
                        files={entry.item.files}
                        label={entry.title}
                        compact
                      />
                      {#if entry.item.omitted_files}<p class="inline-note">
                          {countLabel(
                            entry.item.omitted_files,
                            'linha omitida',
                            'linhas omitidas',
                          )}
                          {t('do relatório salvo para mantê-lo limitado.')}
                        </p>{/if}
                    </section>{/if}
                {/each}
                {#if cleanupReport.volumes.length}<section
                    class="volume-observations"
                  >
                    <h3>{t('Variação observada por volume')}</h3>
                    <p>
                      {t(
                        'Variações observadas não são atribuídas à limpeza. Arquivos enviados à Lixeira não são contabilizados como espaço liberado.',
                      )}
                    </p>
                    {#each cleanupReport.volumes as volume (volume.device)}
                      {@const known = nativeResult?.volumes.find(
                        (v) => v.device === volume.device,
                      )}
                      <div class="outcome-row">
                        <span
                          >{known
                            ? `${t('Volume')} ${displayPath(known.scope_path)}`
                            : t('Volume analisado')}</span
                        ><span
                          >{t('Antes')}
                          {formatBytes(volume.available_before)} · {t('depois')}
                          {volume.available_after === null
                            ? t('não medido')
                            : formatBytes(volume.available_after)} · {t(
                            'variação',
                          )}
                          {volume.observed_delta === null
                            ? t('não medida')
                            : formatSignedBytes(volume.observed_delta)}</span
                        >
                      </div>{/each}
                  </section>{/if}
                <p class="measurement-note">
                  {t(cleanupReport.measurement_note)}
                </p>
                {#if cleanupReport.warnings.length}<section
                    class="cleanup-warnings"
                    aria-label={t('Avisos do executor')}
                  >
                    <h3>{t('Avisos e limites')}</h3>
                    <ul>
                      {#each [...new Set(cleanupReport.warnings.map(warningText))] as warning (warning)}<li
                        >
                          {warning}
                        </li>{/each}
                    </ul>
                  </section>{/if}
                {#if cleanupReport.history_path}<p class="inline-note">
                    {t('Relatório salvo em')}
                    <code>{displayPath(cleanupReport.history_path)}</code>
                  </p>{/if}
              {/if}
              <button
                class="button"
                onclick={() => go('review')}
                disabled={executing}>{t('Voltar à revisão')}</button
              >
            {:else}
              <div class="section-head">
                <div>
                  <span class="eyebrow"
                    >{t(
                      executing ? 'EM ANDAMENTO' : 'RELATÓRIO DE DEMONSTRAÇÃO',
                    )}</span
                  >
                  <h2>
                    {t(
                      executing
                        ? 'Processando exemplos, item a item.'
                        : outcomes.some((o) =>
                              [
                                'failed',
                                'skipped_changed',
                                'cancelled',
                              ].includes(o.outcome),
                            )
                          ? 'Exemplo com itens não concluídos.'
                          : 'Exemplo concluído.',
                    )}
                  </h2>
                  <p>
                    {t(
                      'Resultado somente desta sessão. Nenhum arquivo foi alterado.',
                    )}
                  </p>
                </div>
                {#if executing}<button class="button" onclick={cancelExecution}
                    >{t('Cancelar entre itens')}</button
                  >{/if}
              </div>
              <div class="report-metrics">
                <div>
                  <span>{t('Tamanho lógico selecionado')}</span><strong
                    >{formatBytes(reportBytes)}</strong
                  >
                </div>
                <div>
                  <span>{t('Espaço efetivamente recuperado')}</span><strong
                    >{t('Não medido')}</strong
                  >
                </div>
                <div>
                  <span>{t('Operações reais')}</span><strong
                    >{t('Nenhuma')}</strong
                  >
                </div>
              </div>
              {#each outcomes as outcome (outcome.id)}<div class="outcome-row">
                  <span>{outcome.title}</span><span
                    class:outcome-error={outcome.outcome === 'failed'}
                    class:outcome-done={outcome.outcome === 'simulated'}
                    class:outcome-warning={[
                      'cancelled',
                      'skipped_changed',
                    ].includes(outcome.outcome)}
                    >{#if outcome.outcome === 'simulated'}<Check
                        size={16}
                      />{:else if outcome.outcome === 'processing'}<CircleDashed
                        size={16}
                      />{:else}<CircleAlert size={16} />{/if}{t(
                      outcomeLabel[outcome.outcome],
                    )}</span
                  >
                </div>{/each}
              <p class="inline-note">
                {t(
                  'Lixeira e exclusão permanente não foram executadas. Não existe desfazer para este exemplo, pois nada foi modificado.',
                )}
              </p>
              <button
                class="button"
                onclick={() => go('review')}
                disabled={executing}>{t('Voltar à revisão')}</button
              >
            {/if}
          </section>{:else}<section class="panel empty-state">
            <History size={30} />
            <h2>{t('Ainda não há relatórios.')}</h2>
            <p>
              {t(
                'O histórico de demonstração aparece depois de concluir um exemplo e dura somente nesta sessão.',
              )}
            </p>
            <button class="button" onclick={() => go('review')}
              >{t('Ir para revisão')}</button
            >
          </section>{/if}
      {:else if page === 'settings'}
        <section class="panel padded settings-panel">
          <h2>{t('Aparência e acessibilidade')}</h2>
          <label class="setting-row">
            <span
              ><strong>{t('Idioma')}</strong><small
                >{t('Escolha o idioma da interface.')}</small
              ></span
            >
            <select
              aria-label={t('Idioma')}
              value={language.value}
              onchange={(event) =>
                setLanguage(event.currentTarget.value as Locale)}
            >
              <option value="pt-BR">{t('Português (Brasil)')}</option>
              <option value="en-US">{t('Inglês (Estados Unidos)')}</option>
            </select>
          </label>
          <label class="setting-row"
            ><span
              ><strong>{t('Tema')}</strong><small
                >{t('O modo automático acompanha o sistema.')}</small
              ></span
            ><select
              aria-label={t('Tema')}
              bind:value={theme}
              onchange={applyPreferences}
              ><option value="system">{t('Sistema')}</option><option
                value="light">{t('Claro')}</option
              ><option value="dark">{t('Escuro')}</option></select
            ></label
          ><label class="setting-row"
            ><span
              ><strong>{t('Reduzir animações')}</strong><small
                >{t('A preferência do sistema também é respeitada.')}</small
              ></span
            ><input
              type="checkbox"
              class="switch"
              role="switch"
              bind:checked={reduced}
              onchange={applyPreferences}
            /></label
          >
          <FeedbackSettings
            enabled={soundEnabled}
            onchange={(enabled) => void setSoundEnabled(enabled)}
            onpreview={() => void previewSound()}
            message={soundMessage}
          />
          <div class="section-divider"></div>
          <h2>{t('Pastas e origem dos dados')}</h2>
          {#if demoAvailable}<label class="setting-row"
              ><span
                ><strong>{t('Fonte de dados')}</strong><small
                  >{t(
                    'A demonstração só existe em builds de desenvolvimento.',
                  )}</small
                ></span
              ><select
                aria-label={t('Fonte de dados')}
                bind:value={source}
                onchange={changeSource}
                disabled={!isDesktop ||
                  scanBusy ||
                  executing ||
                  preparingCleanup}
                ><option value="local">{t('Meu computador')}</option
                >{#if !isDesktop}<option value="demo"
                    >{t('Demonstração')}</option
                  >{/if}</select
              ></label
            >{/if}
          <p>
            {t(
              source === 'local'
                ? scanMode === 'automatic'
                  ? 'Analisa automaticamente volumes locais acessíveis sem privilégios. Áreas do sistema, outros usuários, rede, links e caminhos sem leitura permanecem protegidos e podem aparecer como avisos. Nenhuma remoção ocorre durante a análise.'
                  : 'Modo avançado: escolha escopos específicos. Links, mounts externos, sessões e estado são preservados.'
                : 'Os caminhos abaixo são fictícios e não são acessados.',
            )}
          </p>
          {#if source === 'local'}<label class="setting-row">
              <span
                ><strong>{t('Modo de análise')}</strong><small
                  >{t(
                    'Automático percorre os volumes locais permitidos; manual usa somente os escopos marcados.',
                  )}</small
                ></span
              >
              <select
                aria-label={t('Modo de análise')}
                bind:value={scanMode}
                disabled={scanBusy || executing || preparingCleanup}
              >
                <option value="automatic">{t('Automático · computador')}</option
                >
                <option value="manual"
                  >{t('Avançado · escopos escolhidos')}</option
                >
              </select>
            </label>{/if}
          {#if source === 'local' && scanMode === 'manual'}<div
              class="inline-actions scope-actions"
            >
              <button
                class="button"
                disabled={choosingScope ||
                  scanBusy ||
                  executing ||
                  preparingCleanup}
                onclick={() => addNativeScope('projects')}
                ><Folder size={16} /> {t('Adicionar pasta de projetos')}</button
              ><button
                class="button"
                disabled={choosingScope ||
                  scanBusy ||
                  executing ||
                  preparingCleanup}
                onclick={() => addNativeScope('thumbnails')}
                >{t('Escolher thumbnails')}</button
              ><button
                class="button"
                disabled={choosingScope ||
                  scanBusy ||
                  executing ||
                  preparingCleanup}
                onclick={() => addNativeScope('protected')}
                ><Shield size={16} /> {t('Inventário protegido de IA')}</button
              >
            </div>{/if}
          {#if scopeNotice}<div class="notice scope-notice" role="status">
              <CircleAlert size={18} />
              <p>{t(scopeNotice)}</p>
            </div>{/if}
          {#if source !== 'local' || scanMode === 'manual'}{#each scopeOptions as scope (scope.id)}<label
                class="scope-option"
                ><input
                  type="checkbox"
                  checked={scopes.includes(scope.id)}
                  onchange={() => chooseScope(scope.id)}
                  disabled={scanBusy || executing || preparingCleanup}
                /><span
                  ><strong
                    >{source === 'local' ? scope.label : t(scope.label)}</strong
                  ><code>{scope.display_path}</code></span
                ></label
              >{/each}{/if}
          {#if source === 'local'}
            {#if scanBusy || executing || preparingCleanup}
              <p class="inline-note">
                {t(
                  'Conclua ou pause a operação atual antes de alterar os escopos aprovados.',
                )}
              </p>
            {:else if scanMode === 'manual' || nativeScopes.length}
              <ScopeControls
                scopes={nativeScopes}
                onchanged={handleScopesChanged}
              />
            {/if}
          {/if}
          {#if source === 'demo'}<label class="setting-row"
              ><span
                ><strong>{t('Cenário de demonstração')}</strong><small
                  >{t('Reproduza estados sem usar arquivos reais.')}</small
                ></span
              ><select
                aria-label={t('Cenário de demonstração')}
                bind:value={scenario}
                disabled={scanBusy || executing || preparingCleanup}
                ><option value="normal">{t('Resultados normais')}</option
                ><option value="partial"
                  >{t('Falha e item alterado na execução')}</option
                ><option value="empty">{t('Nenhum resultado')}</option><option
                  value="error">{t('Erro de permissão no scan')}</option
                ></select
              ></label
            >{/if}<button
            class="button primary"
            onclick={startScan}
            disabled={scanBusy ||
              executing ||
              preparingCleanup ||
              (source === 'local' && scanMode === 'manual' && !scopes.length)}
            >{t(
              source === 'local'
                ? scanMode === 'automatic'
                  ? 'Analisar computador'
                  : 'Analisar pastas aprovadas'
                : 'Analisar cenário',
            )}</button
          >
          <div class="section-divider"></div>
          <button class="text-button" onclick={() => go('catalog')}
            ><SlidersHorizontal size={17} />
            {t('Abrir catálogo de componentes e estados')}
            <ArrowUpRight size={14} /></button
          >
          <FontCredits />
        </section>
      {:else if page === 'catalog'}
        <section class="panel padded">
          <h2>{t('Componentes em contexto')}</h2>
          <div class="inline-actions">
            <button
              class="button primary"
              onclick={() => (status = 'Ação de exemplo reconhecida.')}
              >{t('Ação primária')}</button
            ><button
              class="button"
              onclick={() => (status = 'Ação secundária reconhecida.')}
              >{t('Secundária')}</button
            ><button class="button" disabled>{t('Indisponível')}</button
            ><RiskBadge risk="low" /><RiskBadge risk="review" /><RiskBadge
              risk="protected"
            />
          </div>
          <div class="section-divider"></div>
          <h3>{t('Carregamento indeterminado')}</h3>
          <p>{t('Nenhuma porcentagem sem total conhecido.')}</p>
          <div class="skeleton-group">
            <div class="skeleton"></div>
            <div class="skeleton"></div>
          </div>
          <div class="notice error">
            <CircleAlert size={20} />
            <p>{t('Permissão negada. Este item não foi processado.')}</p>
          </div>
          <div class="empty-state">
            <Folder size={24} />
            <h3>{t('Nenhum resultado')}</h3>
            <p>{t('Escolha outro escopo para continuar.')}</p>
          </div>
          <button class="button" onclick={() => go('settings')}
            >{t('Experimentar cenários completos')}</button
          >
        </section>
      {/if}
    </main>
  </div>
</div>

{#if onboarding !== null}
  <Modal
    title={t(
      onboarding === 0
        ? 'Seu espaço, sob seu controle.'
        : onboarding === 1
          ? 'Onde você quer olhar?'
          : 'Tudo começa pela leitura.',
    )}
    onclose={() => (onboarding = null)}
    wide
  >
    <div class="onboarding-brand">
      <BrandMark size={44} /><span
        >{t('Inodune')}<span class="brand-dot">.</span></span
      >
    </div>
    <div
      class="onboarding-steps"
      aria-label={`${t('Etapa')} ${formatNumber(onboarding + 1)} ${t('de')} ${formatNumber(3)}`}
    >
      {#each [0, 1, 2] as step (step)}<span class:current={step <= onboarding}
        ></span>{/each}<small
        >{formatNumber(onboarding + 1)} {t('de')} {formatNumber(3)}</small
      >
    </div>
    {#if onboarding === 0}<p class="onboarding-intro">
        {t('Mais espaço para o que importa.')}<br />{t(
          'Sem abrir mão do que é seu.',
        )}
      </p>
      <p>
        {t(
          'O Inodune ajuda você a entender caches e artefatos, revisar cada item e preservar o essencial.',
        )}
      </p>
      <div class="onboarding-promises">
        <span><ShieldCheck size={20} /> {t('Sem remoção automática')}</span
        ><span><HardDrive size={20} /> {t('Processamento local')}</span><span
          ><Shield size={20} /> {t('Sessões e estado protegidos')}</span
        >
      </div>
      <div class="notice">
        <FlaskConical size={20} />
        <p>
          {source === 'local'
            ? t(
                'A análise é somente leitura e percorre automaticamente os volumes locais permitidos. Nenhuma pasta precisa ser escolhida e nada é removido ao iniciar.',
              )
            : t(
                'Você está em uma demonstração. Todos os caminhos e valores são exemplos; nenhum acesso aos seus arquivos será solicitado.',
              )}
        </p>
      </div>
    {:else if onboarding === 1}<p>
        {source === 'local'
          ? t(
              scanMode === 'automatic'
                ? 'O computador será analisado automaticamente em modo somente leitura. Áreas sem acesso ou protegidas serão informadas, nunca contornadas.'
                : 'Modo avançado: escolha escopos específicos no diálogo nativo. A aprovação não inicia o scan.',
            )
          : t(
              'Escolha quais exemplos entram na análise. Nenhuma pasta real será aberta.',
            )}
      </p>
      {#if source === 'local' && scanMode === 'manual'}<div
          class="inline-actions scope-actions"
        >
          <button
            class="button"
            disabled={choosingScope ||
              scanBusy ||
              executing ||
              preparingCleanup}
            onclick={() => addNativeScope('projects')}
            ><Folder size={16} /> {t('Adicionar pasta de projetos')}</button
          ><button
            class="button"
            disabled={choosingScope ||
              scanBusy ||
              executing ||
              preparingCleanup}
            onclick={() => addNativeScope('thumbnails')}
            >{t('Escolher thumbnails')}</button
          ><button
            class="button"
            disabled={choosingScope ||
              scanBusy ||
              executing ||
              preparingCleanup}
            onclick={() => addNativeScope('protected')}
            ><Shield size={16} /> {t('Inventário protegido de IA')}</button
          >
        </div>{/if}
      {#if scopeNotice}<div class="notice scope-notice" role="status">
          <CircleAlert size={18} />
          <p>{t(scopeNotice)}</p>
        </div>{/if}
      {#each scopeOptions as scope (scope.id)}<label class="scope-option"
          ><input
            type="checkbox"
            checked={scopes.includes(scope.id)}
            onchange={() => chooseScope(scope.id)}
          /><span
            ><strong>{source === 'local' ? scope.label : t(scope.label)}</strong
            ><code>{scope.display_path}</code></span
          ></label
        >{/each}
    {:else}<div class="onboarding-scan"><Search size={40} /></div>
      <p>
        {source === 'local'
          ? t(
              scanMode === 'automatic'
                ? 'O scanner Rust vai descobrir volumes locais acessíveis, sem privilégios e sem seguir links. Áreas do sistema, outros usuários, rede e caminhos sem leitura serão preservados e relatados. Você poderá pausar, cancelar e revisar os resultados.'
                : 'O scanner Rust percorrerá somente as pastas aprovadas, sem seguir links ou mounts. Você poderá pausar, cancelar e revisar os resultados.',
            )
          : t(
              'O provedor de demonstração emitirá itens e estados de análise. Você poderá pausar, cancelar e revisar tudo antes de concluir um exemplo.',
            )}
      </p>
      {#if source === 'demo'}<label class="setting-row"
          ><span>{t('Cenário')}</span><select
            aria-label={t('Cenário inicial')}
            bind:value={scenario}
            ><option value="normal">{t('Resultados normais')}</option><option
              value="partial">{t('Execução parcial')}</option
            ><option value="empty">{t('Sem resultados')}</option><option
              value="error">{t('Erro de permissão')}</option
            ></select
          ></label
        >{/if}
    {/if}
    <div class="dialog-actions">
      <button class="text-button" onclick={() => (onboarding = null)}
        >{t('Explorar sem analisar')}</button
      >
      <div class="inline-actions">
        {#if onboarding > 0}<button
            class="button"
            onclick={() => (onboarding = onboarding! - 1)}>{t('Voltar')}</button
          >{/if}<button
          class="button primary"
          disabled={!scopes.length &&
            onboarding > 0 &&
            (source !== 'local' || scanMode === 'manual')}
          onclick={() => {
            if (onboarding === 2) startScan();
            else onboarding = onboarding! + 1;
          }}
          >{t(
            onboarding === 2
              ? source === 'local'
                ? 'Iniciar análise somente leitura'
                : 'Iniciar análise de exemplo'
              : 'Continuar',
          )}<ArrowRight size={16} /></button
        >
      </div>
    </div>
  </Modal>
{/if}
{#if detail}
  <Modal
    drawer
    title={t('Por que este item apareceu?')}
    onclose={() => {
      detail = null;
      status = '';
    }}
  >
    <RiskBadge risk={detail.risk} />
    <h3 class="detail-title">{t(detail.title)}</h3>
    <p class="detail-meta">
      <strong
        >{detail.coverage_key
          ? formatBytes(detail.logical_bytes)
          : t('Não contabilizado separadamente')}</strong
      >{#if detail.coverage_key && detail.allocated_bytes_estimate !== null && detail.allocated_bytes_estimate !== detail.logical_bytes}
        · {formatBytes(detail.allocated_bytes_estimate)} {t('no disco')}{/if}
      · {t(detail.activity)}
    </p>
    <code class="detail-path">{displayPath(detail.display_path)}</code><button
      class="text-button"
      onclick={() => copyPath(displayPath(detail!.display_path))}
      ><Copy size={15} />
      {t(
        source === 'local' ? 'Copiar caminho' : 'Copiar caminho de exemplo',
      )}</button
    >
    <p role="status">{status}</p>
    <dl class="explanation">
      <dt>
        {t(
          detail.risk === 'protected'
            ? 'Motivo do bloqueio'
            : 'Por que apareceu',
        )}
      </dt>
      <dd>{t(tidyReason(detail.reason))}</dd>
      <dt>{t('O que pode mudar')}</dt>
      <dd>{t(detail.impact)}</dd>
      <dt>{t('O que será preservado')}</dt>
      <dd>{t(detail.preserved)}</dd>
      <dt>{t('Como recriar')}</dt>
      <dd>{t(detail.rebuild_hint)}</dd>
    </dl>
    <details>
      <summary>{t('Detector e evidências técnicas')}</summary><code
        >{detail.detector_id} @ {detail.detector_version}</code
      >
      <ul>
        {#each detail.evidence as evidence, index (index)}<li>
            {evidence}
          </li>{/each}
      </ul>
      <p>
        {t('Ação suportada:')}
        {t(
          detail.supported_action === 'cache_contents'
            ? 'conteúdo de cache'
            : detail.supported_action === 'reviewed_cache_contents'
              ? 'conteúdo de cache com revisão individual'
              : 'nenhuma',
        )}. {t(
          'A identidade observada pelo scanner local é conferida pelo executor antes da operação.',
        )}
      </p>
    </details>
    <div class="dialog-actions">
      <button class="button full" onclick={() => (detail = null)}
        >{t('Voltar à revisão')}</button
      >
    </div>
  </Modal>
{/if}
{#if homeConfirm}
  <Modal
    title={t(
      policy === 'trash'
        ? 'Enviar itens para a Lixeira?'
        : 'Excluir itens permanentemente?',
    )}
    onclose={() => (homeConfirm = false)}
  >
    <div class="home-confirm">
      <p class="home-confirm-total">
        <strong>{countLabel(homeItems.length, 'item', 'itens')}</strong>
        <span>{formatBytes(homeBytes)} {t('estimados')}</span>
      </p>
      <ul class="home-confirm-groups">
        {#each [...new Set(homeItems.map((item) => item.category))] as category (category)}
          {@const items = homeItems.filter(
            (item) => item.category === category,
          )}
          <li>
            <span>{t(category)}</span><span
              >{countLabel(items.length, 'item', 'itens')} · {formatBytes(
                items.reduce((n, item) => n + item.logical_bytes, 0),
              )}</span
            >
          </li>
        {/each}
      </ul>
      {#if homeItems.some((item) => item.risk === 'review')}<div
          class="notice"
          role="note"
        >
          <CircleAlert size={20} />
          <p>
            {t(
              'Inclui itens marcados por você que exigem revisão: podem pedir novo download ou recompilação.',
            )}
          </p>
        </div>{/if}
      <div class="notice" class:error={policy === 'permanent'} role="note">
        <CircleAlert size={20} />
        <p>
          {t(
            policy === 'trash'
              ? 'Os arquivos vão para a Lixeira e podem ser restaurados. Na mesma unidade, o espaço só é liberado quando você esvaziar a Lixeira.'
              : 'A exclusão permanente é irreversível.',
          )}
          {#if source === 'local'}{t(
              'O sistema ainda mostra uma confirmação final antes de alterar arquivos.',
            )}{/if}
        </p>
      </div>
    </div>
    <div class="dialog-actions">
      <button class="button" onclick={() => (homeConfirm = false)}
        >{t('Voltar')}</button
      ><button
        class="button primary"
        onclick={() => void confirmOverviewClean()}
        >{policy === 'trash'
          ? `${t('Enviar')} ${countLabel(homeItems.length, 'item', 'itens')} ${t('para a Lixeira')}`
          : `${t('Excluir')} ${countLabel(homeItems.length, 'item', 'itens')}`}</button
      >
    </div>
  </Modal>
{/if}
{#if confirmation}
  <Modal
    title={t(
      source === 'local'
        ? policy === 'trash'
          ? 'Confirmar envio para a Lixeira'
          : 'Confirmar exclusão permanente'
        : policy === 'trash'
          ? 'Enviar para a Lixeira neste exemplo?'
          : 'Excluir permanentemente neste exemplo?',
    )}
    onclose={() => {
      confirmation = false;
      cleanupPlan = null;
      consent = false;
    }}
  >
    {#if source === 'local'}
      <span class="badge neutral"
        ><ShieldCheck size={14} />
        {t('Plano local preparado pelo executor')}</span
      >
      {#if cleanupPlan}<p class="plan-summary">
          <strong
            >{countLabel(cleanupPlan.items.length, 'item', 'itens')}</strong
          >
          · {formatBytes(cleanupPlan.logical_bytes_estimate)}
          {t('estimados')} · {t('plano válido por')}
          {cleanupPlan.expires_in_seconds >= 120
            ? `${formatNumber(Math.floor(cleanupPlan.expires_in_seconds / 60))} min`
            : `${formatNumber(cleanupPlan.expires_in_seconds)} s`}
        </p>
        {#if cleanupPlan.inventory_complete === false}<div
            class="notice"
            role="note"
          >
            <CircleAlert size={20} />
            <p>
              {t(
                'A análise tem áreas não examinadas. Este plano contém somente os itens selecionados com manifesto completo; nada fora dele será alterado.',
              )}
            </p>
          </div>{/if}
        {#if cleanupPlan.review_required || cleanupPlan.items.some((item) => item.risk === 'review' || candidates.find((candidate) => candidate.id === item.candidate_id)?.risk === 'review')}<div
            class="notice"
          >
            <CircleAlert size={20} />
            <p>
              {t(
                'Revisão individual obrigatória. Confira o impacto e o custo de reconstrução informados no plano antes de continuar.',
              )}
            </p>
          </div>{/if}
      {:else}<p>{t('Preparando o plano validado pelo executor.')}</p>{/if}
    {:else}<span class="badge neutral"
        ><FlaskConical size={14} />
        {t('Demonstração, sem operações reais')}</span
      >
      <p>
        {countLabel(selected.length, 'item', 'itens')} · {budgetBytesLabel}
        {t('estimados')}.
      </p>
    {/if}
    <ul class="confirmation-list">
      {#if source === 'local' && cleanupPlan}
        {#each cleanupPlan.items as item (item.candidate_id)}{@const candidate =
            candidates.find((candidate) => candidate.id === item.candidate_id)}
          <li>
            <strong
              >{candidate ? t(candidate.title) : t('Item do plano')}</strong
            ><code>{displayPath(item.display_path)}</code><span
              >{formatBytes(
                item.logical_bytes,
              )}{#if item.allocated_bytes_estimate !== null && item.allocated_bytes_estimate !== item.logical_bytes}
                · {formatBytes(item.allocated_bytes_estimate)}
                {t('no disco')}{/if}</span
            >
            {#if (item.risk ?? candidate?.risk) === 'review'}<RiskBadge
                risk="review"
              />
              {#if item.impact ?? candidate?.impact}<span
                  >{t(item.impact ?? candidate?.impact ?? '')}</span
                >{/if}
              {#if item.rebuild_hint ?? candidate?.rebuild_hint}<span
                  >{t('Custo / como recriar')}: {t(
                    item.rebuild_hint ?? candidate?.rebuild_hint ?? '',
                  )}</span
                >{/if}
            {/if}
          </li>{/each}
      {:else}
        {#each budget.picked as item (item.id)}<li>
            <strong>{t(item.title)}</strong><code
              >{displayPath(item.display_path)}</code
            ><RiskBadge risk={item.risk} />
          </li>{/each}
      {/if}
    </ul>
    <div class="notice" class:error={policy === 'permanent'}>
      <CircleAlert size={20} />
      <p>
        {t(
          source === 'local'
            ? policy === 'trash'
              ? 'Enviar arquivos à Lixeira normalmente preserva a possibilidade de restauração e não libera espaço na mesma unidade. A confirmação nativa do backend ainda será necessária.'
              : 'A exclusão permanente é irreversível. A confirmação nativa do backend ainda será necessária antes de executar o plano.'
            : policy === 'trash'
              ? 'Na mesma unidade, mover para a Lixeira não libera espaço. Uma operação real seria em geral reversível.'
              : 'Uma exclusão permanente real não pode ser desfeita. Esta demonstração não exclui arquivos.',
        )}
      </p>
    </div>
    {#if source === 'local' && cleanupError}<div
        class="notice error"
        role="alert"
      >
        <CircleAlert size={20} />
        <p>{cleanupError}</p>
      </div>{/if}
    <label class="consent"
      ><input type="checkbox" bind:checked={consent} /><span
        >{t(
          source === 'local'
            ? 'Revisei os caminhos e autorizo o executor a prosseguir para a confirmação nativa do sistema.'
            : 'Revisei os itens deste exemplo. Nenhum arquivo do computador será alterado.',
        )}</span
      ></label
    >
    <div class="dialog-actions">
      <button
        class="button"
        onclick={() => {
          confirmation = false;
          cleanupPlan = null;
          consent = false;
        }}>{t('Voltar')}</button
      ><button
        class="button primary"
        disabled={!consent ||
          (source === 'local' && (!cleanupPlan || preparingCleanup))}
        onclick={startExecution}
        >{t(
          source === 'local'
            ? 'Prosseguir para confirmação nativa'
            : 'Confirmar exemplo',
        )}</button
      >
    </div>
  </Modal>
{/if}
