<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { errorText, formatBytes, formatCount } from "../lib/format";
  import { toasts } from "../lib/toast.svelte";
  import type { CatalogEntry, Instance, SearchHit, SearchSort } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  let { instance, onclose, onchanged }: { instance: Instance; onclose: () => void; onchanged: () => void } = $props();

  type Tab = "modrinth" | "community";
  let tab = $state<Tab>("modrinth");

  // ───── Modrinth ─────
  let query = $state("");
  let sort = $state<SearchSort>("relevance");
  let hits = $state<SearchHit[]>([]);
  let total = $state(0);
  /** Siguiente `offset` de Modrinth: cuenta lo recibido, no lo mostrado (se quitan duplicados). */
  let nextOffset = $state(0);
  let loading = $state(false);
  let error = $state("");
  /** Fallo al pedir la página siguiente: se muestra junto al botón, no arriba. */
  let moreError = $state("");
  let installed = $state(new Set<string>());
  let busy = $state<string | null>(null);
  let seq = 0;

  const PAGE = 20;

  async function search(append = false) {
    const mine = ++seq;
    loading = true;
    error = "";
    moreError = "";
    try {
      const res = await api.modrinthSearch({
        query,
        project_type: "mod",
        game_version: instance.minecraft_version,
        loader: instance.loader,
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

  // Busca al escribir (con una pequeña espera) y al cambiar el orden.
  $effect(() => {
    query;
    sort;
    const t = setTimeout(() => void search(), 300);
    return () => clearTimeout(t);
  });

  onMount(() => {
    api
      .modrinthInstalled(instance.id)
      .then((ids) => (installed = new Set(ids)))
      .catch(() => {});
  });

  async function install(hit: SearchHit) {
    busy = hit.project_id;
    try {
      const out = await api.modrinthInstallMod(instance.id, hit.project_id);
      const next = new Set(installed);
      for (const f of out.installed) next.add(f.project_id);
      installed = next;
      const deps = out.installed.filter((f) => !f.requested).length;
      toasts.success(deps > 0 ? `${hit.title} instalado (+${deps} ${deps === 1 ? "dependencia" : "dependencias"})` : `${hit.title} instalado`);
      if (out.missing_dependencies.length > 0) {
        toasts.error(new Error(`Faltan ${out.missing_dependencies.length} dependencias sin versión para ${instance.minecraft_version}`));
      }
      onchanged();
    } catch (e) {
      toasts.error(e, `No se pudo instalar ${hit.title}`);
    } finally {
      busy = null;
    }
  }

  // ───── Comunidad ─────
  let catalog = $state<CatalogEntry[] | null>(null);
  let catalogError = $state("");
  let communityQuery = $state("");
  let allVersions = $state(false);
  let confirmId = $state<string | null>(null);
  let doneIds = $state(new Set<string>());

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
  const communityMods = $derived(
    (catalog ?? []).filter(
      (e) =>
        e.kind === "mod" &&
        e.loader === instance.loader &&
        (allVersions || e.minecraft_version === instance.minecraft_version) &&
        (!communityQuery.trim() ||
          `${e.name} ${e.description ?? ""} ${e.author}`.toLowerCase().includes(communityQuery.trim().toLowerCase())),
    ),
  );

  async function installCommunity(entry: CatalogEntry) {
    if (!entry.file) return;
    busy = entry.id;
    try {
      await api.catalogInstallMod(instance.id, entry.file);
      doneIds = new Set([...doneIds, entry.id]);
      confirmId = null;
      toasts.success(`${entry.name} instalado`);
      onchanged();
    } catch (e) {
      toasts.error(e, `No se pudo instalar ${entry.name}`);
    } finally {
      busy = null;
    }
  }

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
</script>

<Modal
  title="Añadir mods"
  subtitle={`Para Minecraft ${instance.minecraft_version} con ${instance.loader === "fabric" ? "Fabric" : "Vanilla"}. Las dependencias se instalan solas.`}
  width={760}
  {onclose}
>
  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "modrinth"} class:active={tab === "modrinth"} onclick={() => (tab = "modrinth")}>Modrinth</button>
    <button role="tab" aria-selected={tab === "community"} class:active={tab === "community"} onclick={() => (tab = "community")}>Comunidad</button>
  </div>

  {#if instance.loader === "vanilla"}
    <div class="alert warn"><Icon name="info" size={16} /><span>Esta instancia es Vanilla: crea una con Fabric para poder instalar mods.</span></div>
  {:else if tab === "modrinth"}
    <div class="bar">
      <div class="search">
        <Icon name="search" size={15} />
        <input class="input" bind:value={query} placeholder="Buscar en Modrinth… (Sodium, Iris, JEI…)" aria-label="Buscar mods" />
      </div>
      <select class="input" bind:value={sort} aria-label="Ordenar">
        <option value="relevance">Relevancia</option>
        <option value="downloads">Descargas</option>
        <option value="follows">Seguidores</option>
        <option value="updated">Actualizados</option>
        <option value="newest">Nuevos</option>
      </select>
    </div>

    {#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}

    <div class="results">
      {#each hits as hit (hit.project_id)}
        {@const isInstalled = installed.has(hit.project_id)}
        <div class="hit">
          {#if hit.icon_url}
            <img class="icon" src={hit.icon_url} alt="" loading="lazy" />
          {:else}
            <span class="icon placeholder"><Icon name="puzzle" size={20} /></span>
          {/if}
          <div class="body">
            <div class="title">
              <strong>{hit.title}</strong>
              <span class="by">de {hit.author}</span>
            </div>
            <p class="desc">{hit.description}</p>
            <div class="meta">
              <span><Icon name="download" size={12} /> {formatCount(hit.downloads)}</span>
              {#each hit.display_categories.slice(0, 3) as c (c)}<span class="tag">{c}</span>{/each}
            </div>
          </div>
          {#if isInstalled}
            <span class="tag ok"><Icon name="check" size={12} /> Instalado</span>
          {:else}
            <button class="btn sm primary" disabled={busy !== null} onclick={() => install(hit)}>
              {busy === hit.project_id ? "Instalando…" : "Instalar"}
            </button>
          {/if}
        </div>
      {:else}
        {#if loading}
          {#each [1, 2, 3, 4, 5] as n (n)}<div class="skeleton" style="height: 72px"></div>{/each}
        {:else if !error}
          <p class="hint empty">No hay mods que coincidan para esta versión.</p>
        {/if}
      {/each}
      {#if hits.length > 0 && nextOffset < total}
        <div class="more">
          {#if moreError}<span class="more-error"><Icon name="alert" size={14} /> No se pudieron cargar más: {moreError}</span>{/if}
          <button class="btn" disabled={loading} onclick={() => search(true)}>{loading ? "Cargando…" : moreError ? "Reintentar" : "Cargar más"}</button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="bar">
      <div class="search">
        <Icon name="search" size={15} />
        <input class="input" bind:value={communityQuery} placeholder="Buscar mods de la comunidad…" aria-label="Buscar en la comunidad" />
      </div>
      <label class="check"><input type="checkbox" bind:checked={allVersions} /> Otras versiones</label>
      <button class="btn icon" title="Actualizar" onclick={loadCatalog}><Icon name="refresh" size={15} /></button>
    </div>
    <div class="alert info">
      <Icon name="info" size={16} />
      <span>Mods subidos por otros jugadores que no están en Modrinth. Nadie los revisa: un .jar ejecuta código en tu PC, así que instala solo los de gente en la que confíes.</span>
    </div>
    {#if catalogError}<div class="alert"><Icon name="alert" size={16} /><span>{catalogError}</span></div>{/if}
    <div class="results">
      {#if catalog === null}
        {#each [1, 2, 3] as n (n)}<div class="skeleton" style="height: 72px"></div>{/each}
      {:else}
        {#each communityMods as entry (entry.id)}
          <div class="hit">
            <span class="icon placeholder community"><Icon name="user" size={20} /></span>
            <div class="body">
              <div class="title">
                <strong>{entry.name}</strong>
                <span class="by">de {entry.author}</span>
              </div>
              {#if entry.description}<p class="desc">{entry.description}</p>{/if}
              <div class="meta">
                <span class="tag ember">Sin verificar</span>
                <span class="tag">MC {entry.minecraft_version}</span>
                {#if entry.file}<span>{formatBytes(entry.file.size)}</span>{/if}
              </div>
            </div>
            <div class="actions">
              {#if entry.owner === myUuid}
                <button class="btn sm ghost icon" title="Borrar mi publicación" disabled={busy !== null} onclick={() => removeEntry(entry)}><Icon name="trash" size={15} /></button>
              {/if}
              {#if doneIds.has(entry.id)}
                <span class="tag ok"><Icon name="check" size={12} /> Instalado</span>
              {:else if confirmId === entry.id}
                <button class="btn sm danger-soft" disabled={busy !== null} onclick={() => installCommunity(entry)}>{busy === entry.id ? "Instalando…" : "Confío, instalar"}</button>
                <button class="btn sm ghost" onclick={() => (confirmId = null)}>No</button>
              {:else}
                <button class="btn sm primary" disabled={busy !== null} onclick={() => (confirmId = entry.id)}>Instalar</button>
              {/if}
            </div>
          </div>
        {:else}
          <p class="hint empty">
            Todavía nadie ha publicado mods para Minecraft {instance.minecraft_version}.
            Publica uno tuyo con el botón <Icon name="upload" size={12} /> de la lista de mods.
          </p>
        {/each}
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    <button class="btn primary" onclick={onclose}>Listo</button>
  {/snippet}
</Modal>

<style>
  .tabs {
    margin-bottom: 12px;
  }
  .bar {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 12px;
  }
  .search {
    position: relative;
    flex: 1;
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
  .alert {
    margin-bottom: 12px;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: min(58vh, 560px);
    overflow-y: auto;
    padding-right: 2px;
  }
  .hit {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: rgb(255 255 255 / 0.025);
    border: 1px solid transparent;
    transition: background var(--ease), border-color var(--ease);
  }
  .hit:hover {
    background: rgb(255 255 255 / 0.05);
    border-color: var(--border);
  }
  .icon {
    width: 44px;
    height: 44px;
    flex: none;
    border-radius: 10px;
    object-fit: cover;
    background: rgb(255 255 255 / 0.06);
  }
  .placeholder {
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .placeholder.community {
    color: var(--ember);
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .title strong {
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .by {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
  }
  .desc {
    margin: 0;
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
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .meta > span:first-child {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .tag :global(svg) {
    vertical-align: -2px;
  }
  .more {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
  }
  .empty {
    text-align: center;
    padding: 28px 12px;
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
