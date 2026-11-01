<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { isDesktop } from './backend';
  import { compareInstalledSize, formatBytes } from './model';
  type Item = {
    source: string;
    id: string;
    name: string;
    version: string | null;
    size_bytes: number | null;
    install_date: number | null;
    protected: boolean;
    notes: string[];
    signals: { kind: string; timestamp: number; confidence: string }[];
  };
  type Report = {
    source: string;
    status: string;
    warnings: string[];
    items: Item[];
  };
  let sources = $state<Report[] | null>(null);
  let loading = $state(false);
  let error = $state('');
  type UninstallPlan = {
    id: string;
    items: { id: string; version: string }[];
    expires_in_seconds: number;
    steam_handoff: boolean;
  };
  let uninstallPlan = $state<UninstallPlan | null>(null);
  let uninstallBusy = $state(false);
  let uninstallMessage = $state('');
  async function prepareUninstall(item: Item) {
    uninstallBusy = true;
    uninstallMessage = '';
    uninstallPlan = null;
    try {
      uninstallPlan = await invoke<UninstallPlan>('uninstall_prepare', {
        source: item.source,
        id: item.id,
      });
    } catch (e) {
      uninstallMessage = String(e);
    } finally {
      uninstallBusy = false;
    }
  }
  async function executeUninstall() {
    const plan = uninstallPlan;
    if (!plan) return;
    uninstallPlan = null;
    uninstallBusy = true;
    uninstallMessage = '';
    try {
      uninstallMessage = await invoke<string>('uninstall_execute', {
        planId: plan.id,
      });
      await load();
    } catch (e) {
      uninstallMessage = String(e);
    } finally {
      uninstallBusy = false;
    }
  }
  async function load() {
    loading = true;
    error = '';
    try {
      sources = (await invoke<{ sources: Report[] }>('list_installed_apps'))
        .sources;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<section aria-label="Inventário de programas e jogos">
  <p>
    A listagem é somente leitura. Desinstalar exige simulação e confirmação
    nativa. Tamanhos instalados não representam espaço recuperável. Indícios de
    atividade não comprovam desuso.
  </p>
  <button
    class="button primary"
    disabled={loading || uninstallBusy || !isDesktop}
    onclick={load}
    >{loading ? 'Consultando fontes…' : 'Listar programas e jogos'}</button
  >
  {#if !isDesktop}<p role="status">
      Disponível no aplicativo desktop. Nenhum inventário real é carregado nesta
      visualização web.
    </p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  <div aria-live="polite">
    {#if loading}<p>Consultando inventário local…</p>{/if}
  </div>
  {#if uninstallMessage}<p role="status">{uninstallMessage}</p>{/if}
  {#if uninstallBusy}<p role="status">
      Verificando transação; aguarde. Não repita a operação.
    </p>{/if}
  {#if uninstallPlan}
    <section aria-label="Plano de desinstalação">
      <h2>
        {uninstallPlan.steam_handoff
          ? 'Abrir confirmação do Steam'
          : 'Lista completa simulada'}
      </h2>
      <p>
        Plano de uso único, válido por até 120 segundos. A lista será revalidada
        antes de executar. Sem autoremove. O app não recebe senhas.
      </p>
      <ul>
        {#each uninstallPlan.items as removal (removal.id)}<li>
            {removal.id} · {removal.version}
          </li>{/each}
      </ul>
      <button
        class="button primary"
        disabled={uninstallBusy}
        onclick={executeUninstall}>Revisar e confirmar na janela nativa</button
      >
      <button
        class="button"
        disabled={uninstallBusy}
        onclick={() => (uninstallPlan = null)}>Cancelar</button
      >
    </section>
  {/if}
  {#each sources ?? [] as source (source.source)}
    <section class="apps-source">
      <h2>
        {source.source} · {source.items.length}
        {source.items.length === 1 ? 'item' : 'itens'}
      </h2>
      <p>
        Estado: {{
          ok: 'consultado',
          partial: 'parcial',
          unavailable: 'indisponível',
          error: 'erro',
        }[source.status] ?? source.status}
      </p>
      {#each source.warnings as warning, index (index)}<p role="status">
          {warning}
        </p>{/each}
      {#if source.status === 'ok' && !source.items.length}<p>
          Nenhum item encontrado nesta fonte.
        </p>{/if}
      <ul>
        {#each source.items
          .slice()
          .sort((a, b) => compareInstalledSize(a.size_bytes, b.size_bytes) || a.name.localeCompare(b.name, 'pt-BR')) as item, index (index)}
          <li>
            <strong>{item.name}</strong> <span>{item.source}</span>
            {#if item.protected}<span class="badge neutral">Protegido</span
              >{/if}
            <p>
              {item.id} · {item.version ?? 'Versão desconhecida'} · {item.size_bytes ===
              null
                ? 'Tamanho desconhecido'
                : formatBytes(item.size_bytes)}
            </p>
            {#each item.signals as signal, index (index)}<p>
                {signal.kind === 'steam_last_played'
                  ? 'Último uso registrado pelo Steam'
                  : 'Alteração da pasta de dados Flatpak'}: {new Date(
                  signal.timestamp * 1000,
                ).toLocaleString('pt-BR')} · confiança {signal.confidence ===
                'low'
                  ? 'baixa'
                  : 'média'}
              </p>{:else}<p>Último uso desconhecido.</p>{/each}
            <button
              class="button"
              disabled={!isDesktop ||
                loading ||
                uninstallBusy ||
                item.protected}
              onclick={() => prepareUninstall(item)}
              >{item.source === 'steam'
                ? 'Preparar abertura no Steam'
                : 'Simular desinstalação'}</button
            >
            <details>
              <summary>Notas e limites</summary
              >{#each item.notes as note, index (index)}<p>
                  {note}
                </p>{/each}
            </details>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</section>

<style>
  .button.primary:not(:disabled):hover {
    background: var(--accent);
  }
  .apps-source {
    margin-block: 2rem;
  }
  ul {
    list-style: none;
    padding: 0;
  }
  li {
    padding-block: 1rem;
    border-bottom: 1px solid var(--stroke);
    overflow-wrap: anywhere;
  }
  p {
    line-height: 1.6;
  }
</style>
