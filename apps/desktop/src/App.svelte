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
    FlaskConical,
    CircleDashed,
  } from '@lucide/svelte';
  import logo from '../../../assets/brand/inodune-logo.svg';
  import icon from '../../../assets/brand/inodune-icon.svg';
  import InstalledApps from './lib/InstalledApps.svelte';
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
    isLocallyEligible,
    toggleSelection,
    simulateBudget,
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
    { id: 'apps' as Page, label: 'Programas & Jogos', icon: HardDrive },
    { id: 'overview' as Page, label: 'Visão geral', icon: House },
    { id: 'projects' as Page, label: 'Projetos', icon: Layers },
    { id: 'development' as Page, label: 'Desenvolvimento', icon: Code },
    { id: 'git' as Page, label: 'Git Worktrees', icon: GitBranch },
    { id: 'ai' as Page, label: 'IA & Agentes', icon: Bot },
    { id: 'system' as Page, label: 'Sistema', icon: HardDrive },
  ];
  const pageNames: Record<Page, string> = {
    apps: 'Programas & Jogos',
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

  const budget = $derived(simulateBudget(candidates, selected, policy));
  const protectedItems = $derived(
    candidates.filter((c) => c.risk === 'protected'),
  );
  const available = $derived(candidates.filter((c) => c.risk !== 'protected'));
  const locallyEligible = $derived(candidates.filter(isLocallyEligible));
  const potential = $derived(
    available.reduce((n, c) => n + c.logical_bytes, 0),
  );
  const batchWarning = $derived(cleanupBatchWarning(selected.length));
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
    try {
      const scope = await native.chooseScope(kind, language.value);
      if (scope) {
        nativeScopes = await native.listScopes();
        if (!scopes.includes(scope.id)) scopes = [...scopes, scope.id];
      }
    } catch (e) {
      nativeError = String(e);
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
        nativeResult = result.result;
        candidates = result.result.candidates;
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
        selected =
          !result.result.cancelled &&
          (result.result.complete || scanMode === 'automatic')
            ? selectOverviewDefaults(candidates, true)
            : [];
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
    if (!consent || !budget.picked.length || executing) return;
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
  ><title
    >Inodune · {t(
      page === 'overview'
        ? 'Visão geral'
        : page === 'report'
          ? 'Relatório'
          : 'Seu espaço, sob seu controle',
    )}</title
  ><link rel="icon" href={icon} /></svelte:head
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
      ><img src={icon} alt="" /><span
        >{t('Inodune')}<span class="brand-dot">.</span></span
      ></a
    >
    <div class="workspace-label">{t('ESPAÇO DE TRABALHO')}</div>
    <nav aria-label={t('Áreas do aplicativo')}>
      {#each nav as item (item.id)}<button
          class:active={page === item.id}
          aria-current={page === item.id ? 'page' : undefined}
          onclick={() => go(item.id)}
          ><item.icon size={19} /><span>{t(item.label)}</span
          >{#if page === item.id}<span class="nav-marker"></span>{/if}</button
        >{/each}
      <div class="nav-divider"></div>
      <button
        class:active={page === 'protected'}
        aria-current={page === 'protected' ? 'page' : undefined}
        onclick={() => go('protected')}
        ><Shield size={19} /><span>{t('Itens protegidos')}</span
        >{#if candidates.length}<span class="nav-count"
            >{protectedItems.length}</span
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
    <div class="sidebar-bottom">
      <div class="local-note">
        <span class="status-dot"></span><span
          >{t('Local por natureza')}<small
            >{t('Sem conta. Sem telemetria.')}</small
          ></span
        >
      </div>
      <button class:active={page === 'settings'} onclick={() => go('settings')}
        ><Settings size={18} /> {t('Configurações')}</button
      >
      <div class="sidebar-version">
        <span>{t('INODUNE')}</span><span>{t('Linux desktop')}</span>
      </div>
    </div>
  </aside>
  <div class="main-shell">
    <header class="topbar">
      <div>
        <span class="device-mark"><HardDrive size={16} /></span>
        {t(source === 'local' ? 'Meu computador' : 'Ambiente de demonstração')}
        <span class="topbar-separator">/</span>
        <strong
          >{t(
            page === 'apps'
              ? 'Programas e jogos'
              : source === 'local'
                ? scanMode === 'automatic'
                  ? 'Análise automática · limpeza com confirmação'
                  : 'Escopos aprovados · limpeza com confirmação'
                : 'Somente dados sintéticos',
          )}</strong
        >
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
        ><span class="private-label"
          ><ShieldCheck size={15} /> {t('100% local')}</span
        >
      </div>
    </header>
    <main id="main" tabindex="-1">
      <div class="page-head">
        <div>
          <div class="eyebrow">
            {t(
              page === 'overview'
                ? 'VISÃO GERAL'
                : page === 'impact'
                  ? 'SIMULADOR DE ESPAÇO'
                  : page === 'projects'
                    ? 'LENTE DE PROJETOS'
                    : page === 'protected'
                      ? 'REGISTRO DE PROTEÇÕES'
                      : page === 'apps'
                        ? 'PROGRAMAS INSTALADOS'
                        : page === 'settings'
                          ? 'PREFERÊNCIAS'
                          : page === 'history'
                            ? 'HISTÓRICO'
                            : page === 'report'
                              ? 'RELATÓRIO'
                              : [
                                    'development',
                                    'git',
                                    'ai',
                                    'system',
                                    'review',
                                  ].includes(page)
                                ? 'REVISÃO DE ITENS'
                                : 'INODUNE',
            )}
          </div>
          <h1 bind:this={heading} tabindex="-1">{t(pageNames[page])}</h1>
          <p>
            {t(
              page === 'overview'
                ? 'Liberte espaço. Preserve o essencial.'
                : page === 'protected'
                  ? 'Veja o que preservamos e entenda cada motivo.'
                  : page === 'projects'
                    ? 'Artefatos, custos e proteções no contexto de cada projeto.'
                    : page === 'impact'
                      ? 'Uma estimativa para revisar. Nenhuma operação será executada.'
                      : page === 'apps'
                        ? 'Encontre, filtre e remova programas com um plano revisável.'
                        : page === 'settings'
                          ? 'Idioma, aparência e como a análise funciona.'
                          : 'Decisões claras, com você no controle.',
            )}
          </p>
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
        {#if source === 'local' && page === 'overview' && !scanBusy && nativeResult && (!nativeResult.complete || nativeResult.issues.length)}<CoverageNotice
            result={nativeResult}
          />{/if}
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
            cleanLabel={selected.length
              ? `${t('Limpar')} ${countLabel(selected.length, 'item', 'itens')} · ${formatBytes(budget.logicalBytes)}`
              : t('Limpar')}
          />{:else if __INODUNE_DEMO__}
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
            <article class="category-card">
              <div class="category-top">
                <span class="category-icon"><cat.icon size={22} /></span
                ><ArrowUpRight size={17} />
              </div>
              <h2>{t(cat.label)}</h2>
              <div
                class="category-number"
                class:protected-summary={protectedOnly}
              >
                {protectedOnly
                  ? protectedOnlyLabel(
                      group.length,
                      formatBytes(
                        group.reduce((n, c) => n + c.logical_bytes, 0),
                      ),
                    )
                  : t(
                      !candidates.length
                        ? '—'
                        : !measured.length && group.length
                          ? 'Não medido'
                          : (uncertain ? '≥ ' : '') +
                            formatBytes(
                              measured.reduce((n, c) => n + c.logical_bytes, 0),
                            ),
                    )}
              </div>
              <p>{t(cat.caption)}</p>
              <span class="category-qualifier"
                >{t(
                  eligible.length
                    ? 'Potencial lógico estimado'
                    : 'Somente consulta',
                )}</span
              >
              <div class="category-footer">
                <span
                  >{#if eligible.length}{formatNumber(chosen.length)}
                    {t('de')}
                    {countLabel(
                      group.length,
                      'item selecionado',
                      'itens selecionados',
                    )}<br /><small
                      >{formatBytes(selectedBytesOf(eligible))}</small
                    >{:else}{countLabel(group.length, 'item', 'itens')}
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
                    >{t('Protegido')}</span
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
          {#if candidates.length}<details class="review-drawer">
              <summary
                >{`${t('Revisar itens')} · ${formatNumber(selected.length)} ${t('de')} ${countLabel(available.length, 'item selecionado', 'itens selecionados')}`}</summary
              >
              <OverviewReview
                {candidates}
                {selected}
                {isSelectable}
                disabled={scanBusy ||
                  executing ||
                  preparingCleanup ||
                  localScanStale}
                onsetmany={setMany}
                ondetail={(item) => (detail = item)}
              />
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
        <div class="trust-footer">
          <Shield size={17} /><span
            >{t('Preservar também é cuidar do seu espaço.')}</span
          ><button class="text-button" onclick={() => go('protected')}
            >{t('Conheça os itens protegidos')}
            <ArrowUpRight size={14} /></button
          >
        </div>
      {:else if page === 'projects' && !scanBusy}
        <div class="section-head">
          <h2>
            {t(
              source === 'local' ? 'Seus projetos' : 'Seus projetos de exemplo',
            )}
          </h2>
          <span class="muted"
            >{formatNumber(projects.length)}
            {t('grupos · tamanhos sem contêineres sobrepostos')}</span
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
              <p>{t('potencial lógico estimado')}</p>
              <div class="category-footer">
                <span
                  >{formatNumber(items.length)}
                  {t('itens')} · {formatNumber(
                    items.filter((c) => c.risk === 'protected').length,
                  )}
                  {t('protegidos')}</span
                ><ChevronRight size={16} />
              </div></button
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
        <section class="panel">
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
                )}
                · {countLabel(selected.length, 'selecionado', 'selecionados')}
                {t('no total')}
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
            ><label
              ><span class="sr-only">{t('Filtrar por risco')}</span><select
                aria-label={t('Filtrar por risco')}
                bind:value={riskFilter}
                ><option value="all">{t('Todos os riscos')}</option><option
                  value="low">{t('Baixo risco')}</option
                ><option value="review">{t('Revisar')}</option><option
                  value="protected">{t('Protegido')}</option
                ></select
              ></label
            ><label
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
                >{formatBytes(budget.logicalBytes)} · {t(
                  'estimativa, sem recuperação medida',
                )}</span
              >
            </div>
            {#if batchWarning}<p class="inline-note" role="status">
                {batchWarning}
              </p>{/if}
            <div class="inline-actions">
              <button
                class="button"
                onclick={() => (selected = [])}
                disabled={!selected.length ||
                  scanBusy ||
                  executing ||
                  preparingCleanup ||
                  localScanStale}>{t('Desmarcar todos')}</button
              ><button
                class="button"
                onclick={selectAllDeletable}
                disabled={!locallyEligible.length ||
                  scanBusy ||
                  executing ||
                  preparingCleanup ||
                  localScanStale}>{t('Selecionar todos')}</button
              >{#if source === 'local'}<button
                  class="button"
                  onclick={() => void cleanSafe()}
                  disabled={scanBusy ||
                    executing ||
                    preparingCleanup ||
                    localScanStale}>{t('Limpar com 1 clique')}</button
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
          <section class="panel padded">
            <span class="eyebrow">{t('SUA SELEÇÃO')}</span>
            <h2 class="impact-number">{formatBytes(budget.logicalBytes)}</h2>
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
                      ? 'Antes · último scan'
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
            {#each budget.picked as item (item.id)}<div class="cost-row">
                <span>{t(item.title)}</span><span>{t(item.rebuild_hint)}</span>
              </div>{/each}
          </section>
          <section class="panel padded">
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
                <p>{cleanupError}</p>
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
                  <p>{cleanupError}</p>
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
                {#each cleanupReport.items as item (item.candidate_id)}
                  {@const itemCandidate = candidates.find(
                    (candidate) => candidate.id === item.candidate_id,
                  )}
                  <section class="cleanup-report-item">
                    <h3>
                      {itemCandidate
                        ? t(itemCandidate.title)
                        : item.candidate_id}
                    </h3>
                    <span class="badge neutral"
                      >{t(
                        item.complete ? 'Item concluído' : 'Item incompleto',
                      )}</span
                    >
                    <FileOutcomes files={item.files} />
                  </section>
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
      <footer class="page-footer">
        <span>{t('Feito para abrir espaço. Projetado para preservar.')}</span
        ><span
          >{t(
            source === 'local'
              ? 'Inodune · dados locais'
              : 'Prévia de produto · dados sintéticos',
          )}</span
        >
      </footer>
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
    <div class="onboarding-brand"><img src={logo} alt="Inodune" /></div>
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
    title={t('Por que este item apareceu?')}
    onclose={() => {
      detail = null;
      status = '';
    }}
  >
    <RiskBadge risk={detail.risk} />
    <h3 class="detail-title">{t(detail.title)}</h3>
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
      <dt>{t('Como recriar')}</dt>
      <dd>{t(detail.rebuild_hint)}</dd>
      <dt>{t('O que será preservado')}</dt>
      <dd>{t(detail.preserved)}</dd>
      <dt>{t('Última atividade')}</dt>
      <dd>{t(detail.activity)}</dd>
      <dt>{t('Tamanho')}</dt>
      <dd>
        {detail.coverage_key
          ? formatBytes(detail.logical_bytes)
          : t(
              'Não contabilizado separadamente',
            )}{#if detail.coverage_key && detail.allocated_bytes_estimate !== null && detail.allocated_bytes_estimate !== detail.logical_bytes}
          · {formatBytes(detail.allocated_bytes_estimate)} {t('no disco')}{/if}
      </dd>
    </dl>
    <details>
      <summary>{t('Detector e evidências técnicas')}</summary><code
        >{detail.detector_id} @ {detail.detector_version}</code
      >
      <ul>
        {#each detail.evidence as evidence (evidence)}<li>{evidence}</li>{/each}
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
        {countLabel(selected.length, 'item', 'itens')} · {formatBytes(
          budget.logicalBytes,
        )}
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
