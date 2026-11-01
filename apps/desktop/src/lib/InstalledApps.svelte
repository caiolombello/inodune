<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { isDesktop } from './backend';
  import { formatBytes } from './model';
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
    Inventário somente leitura. Tamanhos instalados não representam espaço
    recuperável. Indícios de atividade não comprovam desuso.
  </p>
  <button class="button primary" disabled={loading || !isDesktop} onclick={load}
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
        {#each source.items as item, index (index)}
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
