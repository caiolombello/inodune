<script lang="ts">
  import { language, t } from './i18n.svelte';
  import type { CleanupFileResult } from './backend';
  import { displayPath } from './present';

  type Action = 'trash' | 'permanent';
  let {
    action,
    complete,
    cancelled = false,
    warnings = [],
    file,
  }: {
    action: Action | null;
    complete?: boolean;
    cancelled?: boolean;
    warnings?: string[];
    file?: CleanupFileResult;
  } = $props();

  let copyState = $state<'idle' | 'copied' | 'failed'>('idle');
  const en = $derived(language.value === 'en-US');
  const recoveryPath = $derived(
    displayPath(file?.preserved_at ?? file?.display_path ?? ''),
  );
  const needsAttention = $derived(
    cancelled ||
      complete === false ||
      warnings.length > 0 ||
      file?.status === 'failed' ||
      file?.status === 'cancelled' ||
      file?.status === 'skipped_changed' ||
      file?.status === 'quarantined',
  );

  async function copyRecoveryPath() {
    copyState = 'idle';
    try {
      if (!navigator.clipboard?.writeText)
        throw new Error('Clipboard unavailable');
      await navigator.clipboard.writeText(recoveryPath);
      copyState = 'copied';
    } catch {
      copyState = 'failed';
    }
  }
</script>

{#if file}
  {#if file.status === 'trashed'}
    <p class="note">
      {en
        ? 'Restore through the system Trash. The file continues to occupy disk space.'
        : 'Restaure pela Lixeira do sistema. O arquivo continua ocupando espaço no disco.'}
    </p>
  {:else if file.status === 'removed'}
    <p class="note">
      {en
        ? 'Permanently removed. Inodune cannot undo this action; measured free space is reported separately.'
        : 'Removido permanentemente. O Inodune não pode desfazer esta ação; o espaço medido é informado separadamente.'}
    </p>
  {:else if file.status === 'quarantined'}
    <div class="recovery" aria-label={t('Recuperação manual')}>
      <p class="note">
        {en
          ? 'Preserved for manual inspection. Inodune will not restore it automatically.'
          : 'Preservado para inspeção manual. O Inodune não fará a restauração automaticamente.'}
      </p>
      <p class="location">
        <span>{en ? 'Recovery location' : 'Local de recuperação'}</span>
        <code>{recoveryPath}</code>
      </p>
      <button class="copy" type="button" onclick={copyRecoveryPath}>
        {en ? 'Copy recovery path' : 'Copiar caminho de recuperação'}
      </button>
      {#if file.durability_confirmed === false}
        <p class="warning" role="status">
          {en
            ? 'Durability was not confirmed. Keep this path and inspect it manually.'
            : 'A durabilidade não foi confirmada. Guarde este caminho e confira o item manualmente.'}
        </p>
      {/if}
      {#if copyState === 'copied'}
        <p class="copy-status" role="status">
          {en ? 'Path copied.' : 'Caminho copiado.'}
        </p>
      {:else if copyState === 'failed'}
        <p class="warning" role="alert">
          {en
            ? 'Could not copy the path. Select the path above and copy it manually.'
            : 'Não foi possível copiar o caminho. Selecione o texto acima e copie manualmente.'}
        </p>
      {/if}
    </div>
  {:else if ['failed', 'cancelled', 'skipped_changed'].includes(file.status)}
    <p class="note">
      {en
        ? 'No removal was confirmed for this item. Review its current state before trying again.'
        : 'Nenhuma remoção foi confirmada para este item. Confira o estado atual antes de tentar novamente.'}
    </p>
  {/if}
{:else}
  <aside class:attention={needsAttention} class="guidance">
    {#if action === 'trash'}
      <h4>{en ? 'Moved to Trash' : 'Movido para a Lixeira'}</h4>
      <p>
        {en
          ? 'Use your Linux environment’s Trash to restore these items. They continue to occupy space while there.'
          : 'Use a Lixeira do seu ambiente Linux para restaurar estes itens. Enquanto estiverem nela, continuam ocupando espaço.'}
      </p>
    {:else if action === 'permanent'}
      <h4>{en ? 'Permanent removal' : 'Remoção permanente'}</h4>
      <p>
        {en
          ? 'Inodune cannot undo permanently removed files. Free-space observations are shown separately and are not attributed to this operation.'
          : 'O Inodune não pode desfazer arquivos removidos permanentemente. As variações de espaço são exibidas separadamente, sem atribuição a esta operação.'}
      </p>
    {/if}
    {#if cancelled || complete === false}
      <p class="attention-copy" role="alert">
        {cancelled
          ? en
            ? 'Execution was cancelled. Check each item below; completed changes remain in effect.'
            : 'Execução cancelada. Confira cada item abaixo; alterações já concluídas continuam válidas.'
          : en
            ? 'Execution was incomplete. Review failures and preserved items before deciding what to do next.'
            : 'Execução incompleta. Revise falhas e itens preservados antes de decidir o próximo passo.'}
      </p>
    {/if}
    {#each warnings as warning, index (index)}
      <p class="attention-copy" role="alert">{t(warning)}</p>
    {/each}
  </aside>
{/if}

<style>
  .guidance,
  .recovery {
    display: grid;
    gap: 0.5rem;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-inner);
    background: var(--surface);
    padding: 0.9rem 1rem;
  }

  .guidance {
    border-inline-start: 3px solid var(--accent);
  }

  .guidance.attention,
  .recovery:has(.warning) {
    border-inline-start: 3px solid var(--warning);
  }

  h4,
  p {
    margin: 0;
  }

  p {
    color: var(--secondary-text);
    overflow-wrap: anywhere;
    line-height: 1.5;
  }

  .attention-copy,
  .warning {
    color: var(--warning-text);
  }

  .location {
    display: grid;
    gap: 0.25rem;
    font-size: 0.9rem;
  }

  code {
    overflow-wrap: anywhere;
    color: var(--text);
  }

  .copy {
    justify-self: start;
    min-height: 2.5rem;
    font: inherit;
    font-weight: 600;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--stroke);
    border-radius: 10px;
    padding: 0.5rem 0.75rem;
    cursor: pointer;
    transition:
      color 140ms ease-out,
      border-color 140ms ease-out,
      background-color 140ms ease-out;
  }

  .copy:hover {
    border-color: var(--accent);
  }

  .copy:focus-visible {
    outline: 3px solid var(--accent);
    outline-offset: 2px;
  }

  .copy-status {
    color: var(--accent);
  }

  @media (prefers-reduced-motion: reduce) {
    .copy {
      transition: none;
    }
  }
</style>
