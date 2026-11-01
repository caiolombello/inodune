<script lang="ts">
  import {
    HardDrive,
    ShieldCheck,
    ScanLine,
    Pause,
    Play,
    X,
    Volume2,
    VolumeX,
    Eye,
    Code,
    Bot,
    GitBranch,
    Folder,
  } from '@lucide/svelte';
  import { formatNumber, language, t } from './i18n.svelte';
  import { formatBytes } from './model';
  import type { ScanActivity, ScanFinding } from './backend';
  import RiskBadge from './RiskBadge.svelte';

  let {
    paused,
    title,
    progress,
    findings,
    demo = false,
    reduced,
    soundEnabled,
    onpause,
    onresume,
    oncancel,
    onreduce,
    onsound,
  }: {
    paused: boolean;
    title: string;
    progress: ScanActivity;
    findings: ScanFinding[];
    demo?: boolean;
    reduced: boolean;
    soundEnabled: boolean;
    onpause: () => void;
    onresume: () => void;
    oncancel: () => void;
    onreduce: () => void;
    onsound: () => void;
  } = $props();
  let visible = $state(true);
  const en = $derived(language.value === 'en-US');
  const current = $derived(findings.slice(-4).reverse());
  const stage = $derived(
    /worktrees Git/.test(progress.stage)
      ? en
        ? 'Checking Git worktrees'
        : 'Verificando worktrees Git'
      : /checksum/.test(progress.stage)
        ? en
          ? 'Verifying local provider integrity'
          : 'Conferindo integridade dos providers'
        : /pytest/.test(progress.stage)
          ? en
            ? 'Checking the pytest test index'
            : 'Conferindo o índice de testes pytest'
          : /Descobrindo home/.test(progress.stage)
            ? en
              ? 'Discovering local volumes'
              : 'Descobrindo volumes locais'
            : en
              ? 'Inspecting known local items'
              : 'Inspecionando itens locais conhecidos',
  );
  const categoryIcon = (category: string) =>
    category === 'IA & Agentes'
      ? Bot
      : category === 'Git Worktrees'
        ? GitBranch
        : category === 'Desenvolvimento'
          ? Code
          : Folder;
</script>

<svelte:document onvisibilitychange={() => (visible = !document.hidden)} />

<section
  class="scan-experience"
  class:paused
  class:offscreen={!visible}
  data-scan-experience
  aria-label={en ? 'Analysis in progress' : 'Análise em andamento'}
>
  <div class="scan-topline">
    <span class="scan-mode"
      ><span class="activity-light"></span>{en
        ? 'LOCAL ANALYSIS'
        : 'ANÁLISE LOCAL'}{demo
        ? en
          ? ' · DEMO'
          : ' · DEMONSTRAÇÃO'
        : ''}</span
    >
    <div class="scan-preferences">
      <button class="text-button" aria-pressed={reduced} onclick={onreduce}>
        <Eye size={15} />
        {en ? 'Reduce motion' : 'Reduzir movimento'}
      </button>
      <button
        class="icon-button"
        aria-label={en
          ? soundEnabled
            ? 'Mute sounds'
            : 'Enable sounds'
          : soundEnabled
            ? 'Silenciar sons'
            : 'Ativar sons'}
        aria-pressed={soundEnabled}
        onclick={onsound}
      >
        {#if soundEnabled}<Volume2 size={17} />{:else}<VolumeX size={17} />{/if}
      </button>
    </div>
  </div>
  <div class="scan-body">
    <div class="scan-story">
      <div class="scan-instrument" aria-hidden="true">
        <svg viewBox="0 0 208 208" fill="none">
          <circle class="orbit-base" cx="104" cy="104" r="89" />
          <circle class="orbit-track" cx="104" cy="104" r="72" />
          <g class="orbit"
            ><circle class="orbit-signal" cx="104" cy="104" r="89" /><rect
              x="99"
              y="10"
              width="10"
              height="10"
              rx="2"
            /></g
          >
          <g class="orbit inner"
            ><circle class="inner-signal" cx="104" cy="104" r="72" /></g
          >
        </svg>
        <div class="instrument-center">
          {#if paused}<Pause size={26} />{:else}<HardDrive size={26} />{/if}
          <span>{en ? 'Reading only' : 'Somente leitura'}</span>
        </div>
      </div>
      <div class="scan-description">
        <h2>{title}</h2>
        <p class="stage" role="status" aria-live="polite">
          {paused
            ? en
              ? 'Paused. No next item will be read.'
              : 'Pausada. Nenhum próximo item será lido.'
            : stage}
        </p>
        <p class="scan-reassurance">
          {en
            ? 'Finding what can be recreated. Preserving what matters.'
            : 'Encontrando o que pode ser recriado. Preservando o essencial.'}
        </p>
        <div class="scan-metrics" aria-live="off">
          <div>
            <strong>{formatNumber(progress.visited)}</strong><span
              >{en ? 'entries inspected' : 'entradas inspecionadas'}</span
            >
          </div>
          <div>
            <strong>{formatNumber(progress.candidates)}</strong><span
              >{en ? 'items identified' : 'itens identificados'}</span
            >
          </div>
        </div>
        <p class="indeterminate-note">
          {en
            ? 'Total still unknown. No estimate of time remaining.'
            : 'Total ainda desconhecido. Sem estimativa de tempo restante.'}
        </p>
      </div>
    </div>
    <div class="discovery-pane" aria-live="off">
      <div class="discovery-heading">
        <ScanLine size={18} />
        <h3>{en ? 'Recent discoveries' : 'Descobertas recentes'}</h3>
      </div>
      <p class="discovery-caption">
        {en
          ? 'Partial inventory. Review is available when analysis stops.'
          : 'Inventário parcial. A revisão fica disponível ao encerrar a análise.'}
      </p>
      <div class="findings">
        {#each current as finding (finding.id)}
          {@const Icon = categoryIcon(finding.category)}
          <div class="finding">
            <span class="finding-icon"
              >{#if finding.risk === 'protected'}<ShieldCheck
                  size={19}
                />{:else}<Icon size={19} />{/if}</span
            >
            <div class="finding-copy">
              <strong>{t(finding.title)}</strong><span
                >{t(finding.category)}</span
              >
            </div>
            <div class="finding-meta">
              <span
                >{finding.logical_bytes === 0 && finding.risk === 'protected'
                  ? en
                    ? 'Not measured'
                    : 'Não medido'
                  : formatBytes(finding.logical_bytes)}</span
              ><RiskBadge risk={finding.risk} />
            </div>
          </div>
        {:else}
          <div class="waiting-discoveries">
            <Folder size={24} /><strong
              >{en
                ? 'Making a first pass'
                : 'Preparando as primeiras descobertas'}</strong
            >
            <p>
              {en
                ? 'Only confirmed discoveries will appear here.'
                : 'Somente descobertas confirmadas aparecerão aqui.'}
            </p>
          </div>
        {/each}
      </div>
      <p class="protected-promise">
        <ShieldCheck size={16} />{en
          ? 'Code, sessions and state stay protected.'
          : 'Código, sessões e estado permanecem protegidos.'}
      </p>
    </div>
  </div>
  <div class="scan-bottomline">
    <span
      ><ShieldCheck size={16} />{en
        ? 'No files removed by analysis'
        : 'Nenhum arquivo removido pela análise'}</span
    >
    <div class="inline-actions">
      {#if paused}<button class="button" onclick={onresume}
          ><Play size={16} />{en ? 'Continue' : 'Continuar'}</button
        >{:else}<button class="button" onclick={onpause}
          ><Pause size={16} />{en ? 'Pause' : 'Pausar'}</button
        >{/if}
      <button class="button" onclick={oncancel}
        ><X size={16} />{en ? 'Cancel' : 'Cancelar'}</button
      >
    </div>
  </div>
</section>

<style>
  .scan-experience {
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface);
    margin-bottom: var(--space-6);
    overflow: hidden;
    animation: scan-enter var(--motion-standard) var(--ease-enter);
  }
  .scan-topline,
  .scan-bottomline {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-6);
  }
  .scan-topline {
    border-bottom: 1px solid var(--stroke);
  }
  .scan-mode {
    display: inline-flex;
    gap: var(--space-2);
    align-items: center;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.06em;
    color: var(--secondary-text);
  }
  .activity-light {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .scan-preferences {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .scan-preferences button {
    font-size: 12px;
  }
  .scan-body {
    display: grid;
    grid-template-columns: 1.15fr 1fr;
  }
  .scan-story {
    padding: var(--space-6);
    display: flex;
    align-items: center;
    gap: var(--space-6);
    min-width: 0;
  }
  .scan-instrument {
    position: relative;
    width: 160px;
    height: 160px;
    flex-shrink: 0;
  }
  .scan-instrument svg {
    width: 100%;
    height: 100%;
  }
  .orbit-base {
    stroke: var(--stroke);
    stroke-width: 1;
    stroke-dasharray: 2 7;
  }
  .orbit-track {
    stroke: var(--stroke);
    stroke-width: 8;
  }
  .orbit-signal {
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-dasharray: 80 480;
  }
  .inner-signal {
    stroke: var(--accent);
    stroke-width: 8;
    stroke-linecap: round;
    stroke-dasharray: 64 400;
    opacity: 0.65;
  }
  .orbit {
    transform-origin: 104px 104px;
    animation: activity-orbit 8s linear infinite;
    fill: none;
  }
  .orbit rect {
    fill: var(--accent);
  }
  .inner {
    animation-direction: reverse;
    animation-duration: 12s;
  }
  .paused .orbit,
  .offscreen .orbit {
    animation-play-state: paused;
  }
  .paused .finding,
  .paused.scan-experience {
    animation: none;
  }
  .instrument-center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    color: var(--accent);
  }
  .instrument-center span {
    font-size: 12px;
    color: var(--secondary-text);
  }
  h2 {
    font-size: 22px;
    font-weight: 600;
    letter-spacing: -0.02em;
    line-height: 1.25;
    margin: 0 0 var(--space-3);
  }
  .scan-description {
    min-width: 0;
  }
  .stage {
    font-size: 14px;
    color: var(--accent);
    font-weight: 500;
    margin: 0 0 var(--space-2);
  }
  .scan-reassurance {
    font-size: 14px;
    color: var(--secondary-text);
    line-height: 1.5;
    margin: 0;
    max-width: 38ch;
  }
  .scan-metrics {
    display: flex;
    gap: var(--space-6);
    margin-top: var(--space-6);
  }
  .scan-metrics div {
    display: grid;
    gap: var(--space-1);
  }
  .scan-metrics strong {
    font-size: 24px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    line-height: 1.1;
  }
  .scan-metrics span,
  .indeterminate-note {
    font-size: 12px;
    color: var(--secondary-text);
  }
  .indeterminate-note {
    margin: var(--space-3) 0 0;
    line-height: 1.5;
  }
  .discovery-pane {
    border-left: 1px solid var(--stroke);
    padding: var(--space-6);
    background: var(--canvas);
    min-width: 0;
  }
  .discovery-heading {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--accent);
  }
  h3 {
    margin: 0;
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
  }
  .discovery-caption {
    font-size: 12px;
    color: var(--secondary-text);
    line-height: 1.5;
    margin: var(--space-2) 0 var(--space-4);
  }
  .findings {
    min-height: 150px;
  }
  .finding {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--stroke);
    animation: finding-enter var(--motion-standard) var(--ease-enter);
  }
  .finding-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--secondary-text);
    flex-shrink: 0;
  }
  .finding-copy {
    display: grid;
    gap: var(--space-1);
    min-width: 0;
    flex: 1;
  }
  .finding-copy strong {
    font-size: 14px;
    font-weight: 500;
    overflow-wrap: anywhere;
  }
  .finding-copy span,
  .finding-meta {
    font-size: 12px;
    color: var(--secondary-text);
  }
  .finding-meta {
    display: grid;
    gap: var(--space-1);
    justify-items: end;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .waiting-discoveries {
    display: grid;
    align-content: center;
    justify-items: start;
    min-height: 150px;
    gap: var(--space-2);
    color: var(--secondary-text);
  }
  .waiting-discoveries strong {
    font-size: 14px;
    font-weight: 500;
  }
  .waiting-discoveries p {
    font-size: 12px;
    margin: 0;
  }
  .protected-promise {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    font-size: 12px;
    line-height: 1.5;
    color: var(--secondary-text);
    margin: var(--space-4) 0 0;
  }
  .protected-promise :global(svg) {
    flex-shrink: 0;
  }
  .scan-bottomline {
    border-top: 1px solid var(--stroke);
  }
  .scan-bottomline > span {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: 12px;
    color: var(--secondary-text);
  }
  @keyframes activity-orbit {
    to {
      transform: rotate(360deg);
    }
  }
  @keyframes scan-enter {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @keyframes finding-enter {
    from {
      opacity: 0;
      transform: translateY(3px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @media (max-width: 1200px) {
    .scan-story {
      gap: var(--space-3);
    }
    .scan-instrument {
      width: 120px;
      height: 120px;
    }
    .instrument-center span {
      font-size: 12px;
    }
  }
  @media (max-width: 1000px) {
    .scan-body {
      grid-template-columns: 1fr;
    }
    .discovery-pane {
      border-left: 0;
      border-top: 1px solid var(--stroke);
    }
    .findings {
      min-height: 0;
    }
    .scan-instrument {
      width: 144px;
      height: 144px;
    }
    .scan-story {
      gap: var(--space-6);
    }
  }
  @media (max-width: 520px) {
    .scan-topline,
    .scan-bottomline {
      padding: var(--space-3) var(--space-4);
      flex-wrap: wrap;
      gap: var(--space-3);
    }
    .scan-mode {
      font-size: 12px;
    }
    .scan-preferences {
      gap: var(--space-1);
    }
    .scan-story {
      padding: var(--space-6) var(--space-4);
      gap: var(--space-3);
      align-items: flex-start;
    }
    .scan-instrument {
      width: 72px;
      height: 72px;
      margin-top: var(--space-1);
    }
    .instrument-center span {
      display: none;
    }
    .instrument-center :global(svg) {
      width: 20px;
    }
    h2 {
      font-size: 20px;
    }
    .scan-metrics {
      gap: var(--space-4);
      margin-top: var(--space-4);
      flex-wrap: wrap;
    }
    .scan-metrics strong {
      font-size: 22px;
    }
    .discovery-pane {
      padding: var(--space-4);
    }
    .finding {
      gap: var(--space-2);
    }
    .finding-icon {
      display: none;
    }
    .finding-meta :global(.risk-badge) {
      font-size: 12px;
    }
    .scan-bottomline .inline-actions {
      width: 100%;
    }
    .scan-bottomline button {
      flex: 1;
    }
  }
</style>
