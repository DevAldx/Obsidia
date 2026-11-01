<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { nav } from "../lib/nav.svelte";
  import { news } from "../lib/news.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { formatDay } from "../lib/format";
  import type { Loader, NewsItem, VersionSummary } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import CreateInstanceModal from "../components/CreateInstanceModal.svelte";
  import ImportByCodeModal from "../components/ImportByCodeModal.svelte";

  type Filter = "all" | "news" | "versions";
  let filter = $state<Filter>("all");
  let code = $state("");
  let importCode = $state<string | null>(null);
  let preset = $state<{ version?: string; loader?: Loader; snapshots?: boolean } | null>(null);
  let versions = $state<VersionSummary[]>([]);

  onMount(() => {
    void news.load();
    api
      .listVersions()
      .then((v) => (versions = v))
      .catch(() => (versions = []));
  });

  const latestRelease = $derived(versions.find((v) => v.kind === "release")?.id ?? null);
  const latestSnapshot = $derived(versions.find((v) => v.kind === "snapshot")?.id ?? null);
  const known = $derived(new Set(versions.map((v) => v.id)));

  const items = $derived(
    filter === "news" ? news.articles : filter === "versions" ? news.versions : (news.feed?.items ?? []),
  );
  const lead = $derived(items[0]);
  const rest = $derived(items.slice(1));

  async function open(item: NewsItem) {
    if (!item.link) return;
    try {
      await api.openHttpsUrl(item.link);
    } catch (e) {
      toasts.error(e, "No se pudo abrir el navegador");
    }
  }

  function startImport() {
    importCode = code.trim();
  }
</script>

<div class="discover">
  <header class="intro">
    <p class="eyebrow">Descubrir</p>
    <h1>Encuentra tu próxima partida</h1>
    <p class="muted">Instala configuraciones compartidas, empieza una versión nueva en un clic o ponte al día.</p>
  </header>

  <!-- Importar por código: barra ancha, protagonista -->
  <section class="import facet">
    <div class="import-text">
      <span class="import-icon"><Icon name="download" size={20} /></span>
      <div>
        <strong class="display">¿Tienes un código?</strong>
        <p class="hint">Pega un código y se instala la instancia con sus mods, RAM e imagen.</p>
      </div>
    </div>
    <form class="import-form" onsubmit={(e) => { e.preventDefault(); startImport(); }}>
      <input class="input mono" bind:value={code} placeholder="OBS1-… o OBS-K7P2X9" aria-label="Código de instancia" />
      <button class="btn primary" type="submit">Buscar</button>
    </form>
  </section>

  <!-- Inicio rápido: tres fichas de tamaños distintos -->
  <section>
    <div class="section-head"><h3 class="display">Empieza rápido</h3></div>
    <div class="quick">
      <button class="q q-main obsidia-bg" disabled={!latestRelease} onclick={() => (preset = { version: latestRelease ?? undefined, loader: "fabric" })}>
        <span class="tag accent">Recomendado</span>
        <strong class="display">Fabric {latestRelease ?? "…"}</strong>
        <span class="q-text">La última versión con soporte para mods de rendimiento como Sodium.</span>
        <span class="q-cta">Crear <Icon name="chevronRight" size={15} /></span>
      </button>
      <button class="q" disabled={!latestRelease} onclick={() => (preset = { version: latestRelease ?? undefined, loader: "vanilla" })}>
        <Icon name="layers" size={22} />
        <strong>Vanilla {latestRelease ?? "…"}</strong>
        <span class="q-text">Minecraft tal cual.</span>
      </button>
      <button class="q" disabled={!latestSnapshot} onclick={() => (preset = { version: latestSnapshot ?? undefined, loader: "vanilla", snapshots: true })}>
        <Icon name="sparkles" size={22} />
        <strong>Snapshot {latestSnapshot ?? "…"}</strong>
        <span class="q-text">Prueba lo próximo antes que nadie.</span>
      </button>
    </div>
  </section>

  <!-- Revista de novedades -->
  <section>
    <div class="section-head">
      <h3 class="display">Novedades de Minecraft</h3>
      <div class="filters" role="tablist">
        {#each [["all", "Todo"], ["news", "Noticias"], ["versions", "Versiones"]] as [id, label] (id)}
          <button role="tab" aria-selected={filter === id} class:active={filter === id} onclick={() => (filter = id as Filter)}>{label}</button>
        {/each}
        <button class="refresh" title="Actualizar" onclick={() => news.load(true)} disabled={news.loading}><Icon name="refresh" size={14} /></button>
      </div>
    </div>

    {#if news.feed?.stale}
      <div class="alert info stale"><Icon name="wifiOff" size={16} /><span>Sin conexión: se muestra la última copia guardada.</span></div>
    {/if}

    {#if news.loading && !news.feed}
      <div class="mag">
        <div class="skeleton m-lead"></div>
        {#each [1, 2, 3, 4] as n (n)}<div class="skeleton m-card"></div>{/each}
      </div>
    {:else if !news.feed}
      <div class="alert info">
        <Icon name="wifiOff" size={16} />
        <span>No se pudieron cargar las noticias. {news.error}</span>
        <button class="btn sm" onclick={() => news.load(true)}>Reintentar</button>
      </div>
    {:else if items.length === 0}
      <p class="hint">No hay nada en esta categoría.</p>
    {:else}
      <div class="mag">
        {#if lead}
          {@render card(lead, true)}
        {/if}
        {#each rest as item (item.id)}
          {@render card(item, false)}
        {/each}
      </div>
    {/if}
  </section>

  <!-- Preparado para el futuro catálogo de mods/modpacks -->
  <section class="soon">
    <div>
      <span class="tag ember">Próximamente</span>
      <h3 class="display">Catálogo de modpacks y mods</h3>
      <p class="muted">Buscar e instalar contenido directamente desde Obsidia, sin descargar archivos a mano.</p>
    </div>
    <div class="soon-art" aria-hidden="true">
      <span class="facet"></span><span class="facet"></span><span class="facet"></span>
    </div>
  </section>
</div>

{#snippet card(item: NewsItem, isLead: boolean)}
  {@const isVersion = item.kind !== "news"}
  <article class="m-card" class:m-lead={isLead} class:version={isVersion}>
    <button class="m-hit" onclick={() => open(item)} disabled={!item.link} aria-label={item.title}></button>
    {#if item.image_url}
      <img src={item.image_url} alt="" loading="lazy" decoding="async" />
    {:else}
      <span class="m-noimg obsidia-bg"></span>
    {/if}
    <div class="m-body">
      <div class="m-top">
        {#if isVersion}
          <span class="tag" class:accent={item.kind === "release"}>{item.kind === "release" ? "Release" : "Snapshot"}</span>
        {:else}
          <span class="tag">Noticia</span>
        {/if}
        <span class="m-date">{formatDay(item.date)}</span>
      </div>
      <strong>{item.title}</strong>
      {#if item.summary && (isLead || !isVersion)}<p class="m-sum">{item.summary}</p>{/if}
      {#if isVersion && item.version && known.has(item.version)}
        <button
          class="btn sm m-action"
          onclick={() => (preset = { version: item.version ?? undefined, loader: item.kind === "release" ? "fabric" : "vanilla", snapshots: item.kind !== "release" })}
        >
          <Icon name="plus" size={14} /> Crear instancia
        </button>
      {:else if item.link}
        <span class="m-more">Leer en minecraft.net <Icon name="external" size={13} /></span>
      {/if}
    </div>
  </article>
{/snippet}

{#if preset}
  <CreateInstanceModal
    {preset}
    onclose={(id) => {
      preset = null;
      if (id) nav.openInstance(id);
    }}
  />
{/if}
{#if importCode !== null}
  <ImportByCodeModal
    initialCode={importCode}
    onclose={() => {
      importCode = null;
      if (app.importedId) nav.openInstance(app.importedId);
    }}
  />
{/if}

<style>
  .discover {
    display: flex;
    flex-direction: column;
    gap: 30px;
    max-width: 1240px;
    margin: 0 auto;
    padding: 28px 28px 40px;
  }
  .intro h1 {
    font-size: clamp(26px, 3vw, 36px);
    margin: 4px 0 6px;
  }
  .section-head h3 {
    font-size: 18px;
  }

  /* Importar */
  .import {
    --facet: 16px;
    display: grid;
    grid-template-columns: 1fr minmax(260px, 380px);
    align-items: center;
    gap: 14px 24px;
    padding: 22px 24px;
    background:
      linear-gradient(100deg, color-mix(in srgb, var(--accent) 20%, transparent), transparent 70%),
      rgb(var(--surface-2) / 1);
  }
  .import-text {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .import-text strong {
    font-size: 18px;
  }
  .import-icon {
    width: 44px;
    height: 44px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-ink);
  }
  .import-form {
    display: flex;
    gap: 8px;
  }
  .import-form input {
    flex: 1;
    height: 40px;
  }
  .import-form .btn {
    height: 40px;
  }

  /* Inicio rápido */
  .quick {
    display: grid;
    grid-template-columns: 1.4fr 1fr 1fr;
    gap: 14px;
  }
  .q {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 18px;
    min-height: 150px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
    color: var(--text);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--ease), transform var(--ease);
  }
  .q:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
    transform: translateY(-2px);
  }
  .q:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .q > :global(svg) {
    color: var(--accent-hover);
    margin-bottom: auto;
  }
  .q strong {
    font-size: 16px;
  }
  .q-main {
    justify-content: flex-end;
  }
  .q-main .tag {
    margin-bottom: auto;
  }
  .q-main strong {
    font-size: 24px;
  }
  .q-text {
    font-size: 13px;
    color: var(--text-2);
  }
  .q-cta {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 6px;
    font-weight: 650;
    font-size: 13px;
    color: var(--accent-hover);
  }

  /* Revista */
  .filters {
    display: flex;
    gap: 4px;
    padding: 3px;
    border-radius: 10px;
    background: rgb(255 255 255 / 0.04);
    border: 1px solid var(--border);
  }
  .filters button {
    height: 28px;
    padding: 0 12px;
    border: none;
    border-radius: 7px;
    background: none;
    cursor: pointer;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .filters button.active {
    background: rgb(255 255 255 / 0.1);
    color: var(--text);
  }
  .filters .refresh {
    padding: 0 8px;
  }
  .stale {
    margin-bottom: 12px;
  }
  .mag {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }
  .m-card {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 120px;
    border-radius: var(--radius);
    overflow: hidden;
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
    transition: border-color var(--ease), transform var(--ease);
  }
  .m-card:hover {
    border-color: var(--border-strong);
  }
  .m-hit {
    position: absolute;
    inset: 0;
    z-index: 1;
    background: none;
    border: none;
    cursor: pointer;
  }
  .m-hit:disabled {
    cursor: default;
  }
  .m-card img,
  .m-noimg {
    width: 100%;
    aspect-ratio: 772 / 350;
    object-fit: cover;
  }
  .m-card.version {
    flex-direction: row;
  }
  .m-card.version img,
  .m-card.version .m-noimg {
    width: 34%;
    aspect-ratio: 1;
    flex: none;
  }
  .m-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px 14px;
    min-width: 0;
  }
  .m-top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .m-date {
    font-size: 11px;
    color: var(--muted);
  }
  .m-body strong {
    font-size: 14px;
    line-height: 1.3;
  }
  .m-sum {
    font-size: 13px;
    color: var(--text-2);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .m-action {
    position: relative;
    z-index: 2;
    align-self: flex-start;
    margin-top: auto;
  }
  .m-more {
    margin-top: auto;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--accent-hover);
  }
  /* La primera pieza ocupa dos columnas y lleva el texto sobre la imagen */
  .m-lead {
    grid-column: span 2;
    grid-row: span 2;
    min-height: 320px;
    justify-content: flex-end;
  }
  .m-lead.version {
    flex-direction: column;
  }
  .m-lead img,
  .m-lead .m-noimg,
  .m-lead.version img,
  .m-lead.version .m-noimg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    aspect-ratio: auto;
  }
  .m-lead::after {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(0deg, rgb(9 8 13 / 0.95) 0%, rgb(9 8 13 / 0.3) 60%, transparent 100%);
  }
  .m-lead .m-body {
    position: relative;
    z-index: 2;
    padding: 22px 24px;
    pointer-events: none;
  }
  .m-lead .m-body .m-action {
    pointer-events: auto;
  }
  .m-lead strong {
    font-family: var(--font-display);
    font-size: 26px;
    line-height: 1.15;
  }
  .m-lead .m-sum {
    font-size: 14px;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }
  .skeleton.m-card {
    min-height: 150px;
    border: none;
  }
  .skeleton.m-lead {
    min-height: 320px;
  }

  /* Próximamente */
  .soon {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 24px 28px;
    border-radius: var(--radius-lg);
    border: 1px dashed var(--border-strong);
  }
  .soon h3 {
    font-size: 20px;
    margin: 10px 0 4px;
  }
  .soon-art {
    display: flex;
    gap: 10px;
  }
  .soon-art span {
    --facet: 10px;
    width: 54px;
    height: 54px;
    background: rgb(255 255 255 / 0.05);
  }
  .soon-art span:nth-child(2) {
    background: color-mix(in srgb, var(--accent) 30%, transparent);
    transform: translateY(-10px);
  }

  @media (max-width: 1050px) {
    .import {
      grid-template-columns: 1fr;
    }
    .quick {
      grid-template-columns: 1fr 1fr;
    }
    .q-main {
      grid-column: 1 / -1;
    }
    .mag {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
