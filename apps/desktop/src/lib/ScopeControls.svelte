<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { NativeScope } from './backend';
  import { t } from './i18n.svelte';

  let {
    scopes,
    onchanged,
  }: { scopes: NativeScope[]; onchanged: (scopes: NativeScope[]) => void } =
    $props();
  let draft = $state<Record<string, string>>({});
  let pending = $state<Record<string, boolean>>({});
  let error = $state<Record<string, string>>({});
  let status = $state<Record<string, string>>({});

  async function update(scope: NativeScope, paths: string[]) {
    pending[scope.id] = true;
    error[scope.id] = '';
    status[scope.id] = '';
    try {
      const updated = await invoke<NativeScope>('set_scope_exclusions', {
        scopeId: scope.id,
        relativePaths: paths,
      });
      onchanged(
        scopes.map((item) => (item.id === updated.id ? updated : item)),
      );
      draft[scope.id] = '';
      status[scope.id] = t('Exclusões atualizadas.');
    } catch (reason) {
      error[scope.id] = String(reason);
    } finally {
      pending[scope.id] = false;
    }
  }

  async function revoke(scope: NativeScope) {
    pending[scope.id] = true;
    error[scope.id] = '';
    status[scope.id] = '';
    try {
      await invoke('forget_scope', { scopeId: scope.id });
      onchanged(scopes.filter((item) => item.id !== scope.id));
      status[scope.id] = t('Aprovação revogada.');
    } catch (reason) {
      error[scope.id] = String(reason);
    } finally {
      pending[scope.id] = false;
    }
  }
</script>

<section class="scope-controls" aria-labelledby="scope-controls-title">
  <h3 id="scope-controls-title">{t('Controles dos escopos')}</h3>
  <p>
    {t('Exclusões usam caminhos relativos literais. “.” exclui todo o escopo.')}
  </p>
  {#each scopes as scope (scope.id)}
    <article>
      <header>
        <strong>{scope.label}</strong>
        <code>{scope.display_path}</code>
      </header>
      <ul>
        {#each scope.exclusions ?? [] as path (path)}
          <li>
            <code>{path}</code><button
              disabled={pending[scope.id]}
              onclick={() =>
                update(
                  scope,
                  (scope.exclusions ?? []).filter((item) => item !== path),
                )}>{t('Remover exclusão')}</button
            >
          </li>
        {/each}
      </ul>
      <form
        onsubmit={(event) => {
          event.preventDefault();
          const value = draft[scope.id]?.trim();
          if (value) update(scope, [...(scope.exclusions ?? []), value]);
        }}
      >
        <label>
          {t('Caminho relativo a excluir')}
          <input
            bind:value={draft[scope.id]}
            placeholder="build/generated"
            autocomplete="off"
            spellcheck="false"
          />
        </label>
        <button
          type="submit"
          disabled={pending[scope.id] || !draft[scope.id]?.trim()}
          >{t('Adicionar exclusão')}</button
        >
      </form>
      {#if error[scope.id]}<p role="alert">{error[scope.id]}</p>{/if}
      {#if pending[scope.id]}<p role="status">
          {t('Atualizando escopo…')}
        </p>{/if}
      {#if status[scope.id]}<p role="status">{status[scope.id]}</p>{/if}
      <button
        class="revoke"
        disabled={pending[scope.id]}
        onclick={() => revoke(scope)}>{t('Revogar aprovação')}</button
      >
    </article>
  {/each}
</section>

<style>
  .scope-controls {
    display: grid;
    gap: 1rem;
  }
  article {
    border: 1px solid var(--stroke);
    border-radius: 0.75rem;
    padding: 1rem;
    display: grid;
    gap: 0.75rem;
  }
  header,
  li,
  form {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  code {
    overflow-wrap: anywhere;
  }
  ul {
    margin: 0;
    padding-left: 1.25rem;
  }
  button,
  input {
    font: inherit;
  }
  label {
    display: grid;
    gap: 0.35rem;
    flex: 1;
  }
  .revoke {
    justify-self: start;
  }
</style>
