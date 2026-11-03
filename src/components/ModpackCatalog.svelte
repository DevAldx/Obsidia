<script lang="ts">
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { nav } from "../lib/nav.svelte";
  import { errorText, formatCount } from "../lib/format";
  import { toasts } from "../lib/toast.svelte";
  import type { CatalogEntry, SearchHit, SearchSort } from "../lib/types";
  import Icon from "./Icon.svelte";
  import InstallModpackModal from "./InstallModpackModal.svelte";
  import ImportByCodeModal from "./ImportByCodeModal.svelte";

  type Tab = "modrinth" | "community";
  let tab = $state<Tab>("modrinth");

  // ───── Modrinth ─────
  let query = $state("");
  let sort = $state<SearchSort>("downloads");
  let hits = $state<SearchHit[]>([]);
  let total = $state(0);
  /** Siguiente `offset` de Modrinth: cuenta lo recibido, no lo mostrado (se quitan duplicados). */
  let nextOffset = $state(0);
  let loading = $state(false);
  let error = $state("");
  /** Fallo al pedir la página siguiente: se muestra junto al botón, no arriba. */
  let moreError = $state("");
  let seq = 0;
  const PAGE = 18;

  async function search(append = false) {
    const mine = ++seq;
    loading = true;
    error = "";
    moreError = "";
    try {
      const res = await api.modrinthSearch({
        query,
        project_type: "modpack",
        loader: "fabric",
        offset: append ? nextOffset : 0,
        limit: PAGE,
        sort,
      });
      if (mine !== seq) return;
      // Entre páginas Modrinth puede repetir un proyecto; una clave repetida
      // rompe la lista, así que solo se añaden los que no estaban.
      const seen = new Set(append ? hits.map((h) => h.project_id) : []);
      const fresh = res.hits.filter((h) => !seen.has(h.project_id) && seen.add(h.project_id));
      hits = append ? [...hits, ...fresh] : fresh;
      nextOffset = res.offset + res.hits.length;
      total = res.total_hits;
    } catch (e) {
      if (mine !== seq) return;
      if (append) moreError = errorText(e);
      else error = errorText(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  $effect(() => {
    query;
    sort;
    const t = setTimeout(() => void search(), 300);
    return () => clearTimeout(t);
  });

  // ───── Comunidad ─────
  let catalog = $state<CatalogEntry[] | null>(null);
  let catalogError = $state("");
  let communityQuery = $state("");
  let busy = $state<string | null>(null);

  async function loadCatalog() {
    catalogError = "";
    try {
      catalog = await api.catalogList();
    } catch (e) {
      catalogError = errorText(e);
      catalog = [];
    }
  }

  $effect(() => {
    if (tab === "community" && catalog === null) void loadCatalog();
  });

  const myUuid = $derived(app.account?.uuid?.replace(/-/g, "").toLowerCase() ?? "");
  const packs = $derived(
    (catalog ?? []).filter(
      (e) =>
        e.kind === "pack" &&
        (!communityQuery.trim() ||
          `${e.name} ${e.description ?? ""} ${e.author} ${e.minecraft_version}`
            .toLowerCase()
            .includes(communityQuery.trim().toLowerCase())),
    ),
  );

  async function removeEntry(entry: CatalogEntry) {
    busy = entry.id;
    try {
      await api.catalogDelete(entry.id);
      catalog = (catalog ?? []).filter((e) => e.id !== entry.id);
      toasts.success("Publicación borrada");
    } catch (e) {
      toasts.error(e, "No se pudo borrar");
    } finally {
      busy = null;
    }
  }

  // ───── Instalar ─────
  let installHit = $state<SearchHit | null>(null);
  let installFile = $state<File | null>(null);
  let importCode = $state<string | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  function onFileChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const f = input.files?.[0] ?? null;
    input.value = "";
    if (f) installFile = f;
  }

  function closeInstall() {
    installHit = null;
    installFile = null;
    if (app.importedId) nav.openInstance(app.importedId);
  }
</script>

<section class="catalog">
  <div class="section-head">
    <h3 class="display">Modpacks</h3>
    <div class="head-actions">
      <div class="tabs" role="tablist">
        <button role="tab" aria-selected={tab === "modrinth"} class:active={tab === "modrinth"} onclick={() => (tab = "modrinth")}>Modrinth</button>
        <button role="tab" aria-selected={tab === "community"} class:active={tab === "community"} onclick={() => (tab = "community")}>Comunidad</button>
      </div>
      <input bind:this={fileInput} type="file" accept=".mrpack" hidden onchange={onFileChosen} />
      <button class="btn sm" onclick={() => fileInput?.click()}><Icon name="upload" size={14} /> Instalar .mrpack</button>
    </div>
  </div>

  {#if tab === "modrinth"}
    <div class="bar">
      <div class="search">
        <Icon name="search" size={15} />
        <input class="input" bind:value={query} placeholder="Buscar modpacks de Fabric…" aria-label="Buscar modpacks" />
      </div>
      <select class="input" bind:value={sort} aria-label="Ordenar">
        <option value="downloads">Más descargados</option>
        <option value="relevance">Relevancia</option>
        <option value="follows">Seguidores</option>
        <option value="updated">Actualizados</option>
        <option value="newest">Nuevos</option>
      </select>
    </div>
    {#if error}<div class="alert info"><Icon name="wifiOff" size={16} /><span>No se pudo buscar en Modrinth. {error}</span></div>{/if}

    <div class="grid">
      {#each hits as hit (hit.project_id)}
        <button class="card-pack" onclick={() => (installHit = hit)}>
          <span class="cover" style:background-image={hit.featured_gallery ? `url("${hit.featured_gallery}")` : undefined}>
            {#if hit.icon_url}<img class="pack-icon" src={hit.icon_url} alt="" loading="lazy" />{/if}
          </span>
          <span class="card-body">
            <strong>{hit.title}</strong>
            <span class="by">de {hit.author}</span>
            <span class="desc">{hit.description}</span>
            <span class="meta"><Icon name="download" size={12} /> {formatCount(hit.downloads)}</span>
          </span>
        </button>
      {:else}
        {#if loading}
          {#each [1, 2, 3, 4, 5, 6] as n (n)}<div class="skeleton" style="height: 230px"></div>{/each}
        {:else if !error}
          <p class="hint">No hay modpacks que coincidan.</p>
        {/if}
      {/each}
    </div>
    {#if hits.length > 0 && nextOffset < total}
      <div class="more">
          {#if moreError}<span class="more-error"><Icon name="alert" size={14} /> No se pudieron cargar más: {moreError}</span>{/if}
          <button class="btn" disabled={loading} onclick={() => search(true)}>{loading ? "Cargando…" : moreError ? "Reintentar" : "Cargar más"}</button>
        </div>
    {/if}
  {:else}
    <div class="bar">
      <div class="search">
        <Icon name="search" size={15} />
        <input class="input" bind:value={communityQuery} placeholder="Buscar packs de la comunidad…" aria-label="Buscar en la comunidad" />
      </div>
      <button class="btn icon" title="Actualizar" onclick={loadCatalog}><Icon name="refresh" size={15} /></button>
    </div>
    <p class="hint">
      Packs que comparten otros jugadores. Para publicar el tuyo: <strong>Instancias › Compartir</strong> y, con el código
      generado, <strong>Publicar en la comunidad</strong>.
    </p>
    {#if catalogError}<div class="alert"><Icon name="alert" size={16} /><span>{catalogError}</span></div>{/if}

    <div class="grid">
      {#if catalog === null}
        {#each [1, 2, 3] as n (n)}<div class="skeleton" style="height: 230px"></div>{/each}
      {:else}
        {#each packs as entry (entry.id)}
          <div class="card-pack community">
            <button class="hit-area" aria-label={`Instalar ${entry.name}`} onclick={() => (importCode = entry.code)}></button>
            <span class="cover obsidia-bg" style:background-image={entry.image_url ? `url("${entry.image_url}")` : undefined}></span>
            <span class="card-body">
              <strong>{entry.name}</strong>
              <span class="by">de {entry.author}</span>
              {#if entry.description}<span class="desc">{entry.description}</span>{/if}
              <span class="meta">
                <span class="tag">MC {entry.minecraft_version}</span>
                <span class="tag">{entry.loader === "fabric" ? "Fabric" : "Vanilla"}</span>
                {#if entry.mod_count}<span class="tag">{entry.mod_count} mods</span>{/if}
              </span>
            </span>
            {#if entry.owner === myUuid}
              <button class="btn sm ghost icon remove" title="Borrar mi publicación" disabled={busy !== null} onclick={() => removeEntry(entry)}><Icon name="trash" size={15} /></button>
            {/if}
          </div>
        {:else}
          <p class="hint">Todavía no hay packs publicados. ¡Sé el primero!</p>
        {/each}
      {/if}
    </div>
  {/if}
</section>

{#if installHit || installFile}
  <InstallModpackModal hit={installHit} file={installFile} onclose={closeInstall} />
{/if}
{#if importCode}
  <ImportByCodeModal
    initialCode={importCode}
    onclose={() => {
      importCode = null;
      if (app.importedId) nav.openInstance(app.importedId);
    }}
  />
{/if}

<style>
  .catalog {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .section-head h3 {
    font-size: 18px;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .search {
    position: relative;
    flex: 1;
    max-width: 480px;
    display: flex;
    align-items: center;
  }
  .search :global(svg) {
    position: absolute;
    left: 10px;
    color: var(--muted);
  }
  .search input {
    width: 100%;
    padding-left: 32px;
    height: 36px;
  }
  select.input {
    height: 36px;
    width: auto;
  }
  .hint {
    margin: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px;
  }
  .card-pack {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 0;
    overflow: hidden;
    text-align: left;
    color: var(--text);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
    cursor: pointer;
    transition: border-color var(--ease), transform var(--ease);
  }
  .card-pack:hover {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    transform: translateY(-2px);
  }
  .hit-area {
    position: absolute;
    inset: 0;
    z-index: 1;
    border: none;
    background: none;
    cursor: pointer;
  }
  .remove {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 2;
    background: rgb(0 0 0 / 0.55);
  }
  .cover {
    position: relative;
    height: 104px;
    background: linear-gradient(135deg, color-mix(in srgb, var(--accent) 35%, transparent), rgb(var(--surface-3) / 1));
    background-size: cover;
    background-position: center;
  }
  .pack-icon {
    position: absolute;
    left: 14px;
    bottom: -20px;
    width: 48px;
    height: 48px;
    border-radius: 12px;
    object-fit: cover;
    border: 2px solid rgb(var(--surface) / 1);
    background: rgb(var(--surface-2) / 1);
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 26px 14px 14px;
  }
  .community .card-body {
    padding-top: 12px;
  }
  .card-body strong {
    font-size: 14.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .by {
    font-size: 12px;
    color: var(--muted);
  }
  .desc {
    font-size: 12.5px;
    color: var(--text-2);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .more {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .more-error {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--danger);
    text-align: center;
  }
</style>
