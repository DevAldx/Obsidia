<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { nav, type InstanceTab } from "../lib/nav.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { lazyBackground } from "../lib/lazyBackground";
  import { formatLastPlayed, formatMemory, formatPlaytime, javaLabel, loaderLabel } from "../lib/format";
  import PlayButton from "../components/PlayButton.svelte";
  import Icon from "../components/Icon.svelte";
  import LogView from "../components/LogView.svelte";
  import ModsPanel from "../components/ModsPanel.svelte";
  import InstanceSettingsPanel from "../components/InstanceSettingsPanel.svelte";
  import CreateInstanceModal from "../components/CreateInstanceModal.svelte";
  import ImportByCodeModal from "../components/ImportByCodeModal.svelte";

  let query = $state("");
  let creating = $state(false);
  let importing = $state(false);
  let addMenu = $state(false);
  let exportedPath = $state<string | null>(null);

  onMount(() => {
    if (nav.createRequested) {
      nav.createRequested = false;
      creating = true;
    }
  });

  const sorted = $derived(
    [...app.list.instances].sort((a, b) => (b.last_played ?? 0) - (a.last_played ?? 0) || a.name.localeCompare(b.name)),
  );
  const filtered = $derived(
    query.trim()
      ? sorted.filter((i) =>
          `${i.name} ${i.minecraft_version} ${i.loader} ${i.id}`.toLowerCase().includes(query.trim().toLowerCase()),
        )
      : sorted,
  );

  const current = $derived(
    app.list.instances.find((i) => i.id === nav.instanceId) ?? app.selected ?? sorted[0] ?? null,
  );
  const bg = $derived(current ? app.backgrounds[current.id] : undefined);
  const isMain = $derived(current?.id === app.settings?.selected_instance);

  $effect(() => {
    if (current) void app.ensureBackground(current.id);
  });

  // Al cambiar de instancia se oculta el resultado de exportar de la anterior.
  $effect(() => {
    current?.id;
    exportedPath = null;
  });

  const tabs: { id: InstanceTab; label: string }[] = [
    { id: "overview", label: "Resumen" },
    { id: "mods", label: "Mods" },
    { id: "settings", label: "Ajustes" },
    { id: "logs", label: "Registros" },
  ];

  function pick(id: string) {
    nav.instanceId = id;
  }

  async function openFolder() {
    if (!current) return;
    try {
      await api.openInstanceFolder(current.id, "");
    } catch (e) {
      toasts.error(e, "No se pudo abrir la carpeta");
    }
  }

  async function exportManifest() {
    if (!current) return;
    try {
      exportedPath = await api.exportManifestTemplate(current.id);
      toasts.success("Plantilla de manifiesto exportada");
    } catch (e) {
      toasts.error(e, "No se pudo exportar");
    }
  }

  function onWindowClick(e: MouseEvent) {
    if (addMenu && !(e.target as HTMLElement).closest(".add-wrap")) addMenu = false;
  }
</script>

<svelte:window onclick={onWindowClick} />

<div class="library">
  <!-- ───────────── Lista ───────────── -->
  <aside class="list">
    <div class="list-head">
      <h1>Instancias</h1>
      <span class="count">{app.list.instances.length}</span>
      <span class="spacer"></span>
      <div class="add-wrap">
        <button class="btn primary sm" onclick={() => (addMenu = !addMenu)} aria-haspopup="menu" aria-expanded={addMenu}>
          <Icon name="plus" size={15} /> Nueva
        </button>
        {#if addMenu}
          <div class="add-menu" role="menu">
            <button role="menuitem" onclick={() => { addMenu = false; creating = true; }}>
              <Icon name="plus" size={16} />
              <span><strong>Crear instancia</strong><span class="hint">Elige versión y loader</span></span>
            </button>
            <button role="menuitem" onclick={() => { addMenu = false; importing = true; }}>
              <Icon name="download" size={16} />
              <span><strong>Importar por código</strong><span class="hint">Desde un manifiesto compartido</span></span>
            </button>
          </div>
        {/if}
      </div>
    </div>

    {#if app.list.instances.length > 3}
      <div class="search">
        <Icon name="search" size={15} />
        <input class="input" bind:value={query} placeholder="Buscar…" aria-label="Buscar instancias" />
      </div>
    {/if}

    <div class="rows">
      {#each filtered as i (i.id)}
        {@const tbg = app.backgrounds[i.id]}
        {@const state = app.games[i.id]}
        <button class="row" class:active={current?.id === i.id} onclick={() => pick(i.id)} use:lazyBackground={i.id}>
          <span class="thumb" class:obsidia-bg={!tbg} style:background-image={tbg ? `url(${tbg})` : undefined}></span>
          <span class="row-text">
            <span class="row-name">
              {i.name}
              {#if i.id === app.settings?.selected_instance}<Icon name="star" size={12} />{/if}
            </span>
            <span class="row-meta">{i.minecraft_version} · {loaderLabel(i)}</span>
          </span>
          <span
            class="dot"
            class:live={state === "running"}
            class:busy={state === "preparing"}
            class:ok={!state && i.installed}
            title={state === "running" ? "En juego" : state === "preparing" ? "Preparando" : i.installed ? "Instalada" : "Se descargará al jugar"}
          ></span>
        </button>
      {:else}
        {#if app.list.instances.length > 0}
          <p class="hint pad">Nada coincide con «{query}».</p>
        {/if}
      {/each}
    </div>

    {#if app.list.broken.length > 0}
      <div class="alert warn broken">
        <Icon name="alert" size={16} />
        <div>
          <strong>{app.list.broken.length} instancia(s) con errores</strong>
          {#each app.list.broken as b (b.id)}<p class="hint"><span class="mono">{b.id}</span>: {b.error}</p>{/each}
        </div>
      </div>
    {/if}
  </aside>

  <!-- ───────────── Detalle ───────────── -->
  <section class="detail">
    {#if current}
      {#key current.id}
        <div class="detail-inner" in:fade={{ duration: app.settings?.ui.animations === false ? 0 : 160 }}>
          <header class="banner" class:obsidia-bg={!bg} style:background-image={bg ? `url(${bg})` : undefined}>
            <div class="banner-shade"></div>
            <div class="banner-top">
              <span class="tag mono">{current.id}</span>
              {#if isMain}
                <span class="tag accent"><Icon name="star" size={12} /> Principal</span>
              {:else}
                <button class="tag action" onclick={() => app.select(current.id)} title="Mostrar en Inicio">
                  <Icon name="star" size={12} /> Hacer principal
                </button>
              {/if}
            </div>
            <div class="banner-main">
              <div class="banner-text">
                <h1 class="name">{current.name}</h1>
                <p class="meta">Minecraft {current.minecraft_version} · {loaderLabel(current)} · {current.mod_count} mods</p>
              </div>
              <div class="banner-actions">
                <PlayButton id={current.id} />
                <button class="glass-btn" title="Abrir carpeta" onclick={openFolder}><Icon name="folder" size={17} /></button>
                <button class="glass-btn" title="Exportar plantilla de manifiesto" onclick={exportManifest}><Icon name="share" size={17} /></button>
              </div>
            </div>
          </header>

          <div class="tabs" role="tablist">
            {#each tabs as t (t.id)}
              <button role="tab" aria-selected={nav.instanceTab === t.id} class:active={nav.instanceTab === t.id} onclick={() => (nav.instanceTab = t.id)}>
                {t.label}
                {#if t.id === "mods"}<span class="count">{current.mod_count}</span>{/if}
                {#if t.id === "logs" && app.games[current.id]}<span class="dot live"></span>{/if}
              </button>
            {/each}
          </div>

          <div class="tab-body">
            {#if app.errors[current.id]}
              <div class="alert run-error">
                <Icon name="alert" size={16} />
                <span>{app.errors[current.id]}</span>
                {#if nav.instanceTab !== "logs"}<button class="btn sm ghost" onclick={() => (nav.instanceTab = "logs")}>Ver registros</button>{/if}
              </div>
            {/if}

            {#if nav.instanceTab === "overview"}
              <div class="stats">
                <div class="stat big">
                  <span class="stat-label"><Icon name="clock" size={14} /> Tiempo jugado</span>
                  <strong class="display">{formatPlaytime(current.total_playtime_secs)}</strong>
                  <span class="hint">Última sesión: {formatLastPlayed(current.last_played)}</span>
                </div>
                <div class="stat">
                  <span class="stat-label"><Icon name="puzzle" size={14} /> Mods</span>
                  <strong class="display">{current.mod_count}</strong>
                  <button class="link" onclick={() => (nav.instanceTab = "mods")}>Gestionar</button>
                </div>
                <div class="stat">
                  <span class="stat-label"><Icon name="cpu" size={14} /> Memoria</span>
                  <strong class="display">{formatMemory(current.memory.max_mb)}</strong>
                  <span class="hint">mín. {formatMemory(current.memory.min_mb)}</span>
                </div>
                <div class="stat">
                  <span class="stat-label"><Icon name="coffee" size={14} /> Java</span>
                  <strong class="display small">{javaLabel(current, app.java)}</strong>
                  <button class="link" onclick={() => (nav.instanceTab = "settings")}>Cambiar</button>
                </div>
                <div class="stat">
                  <span class="stat-label"><Icon name="download" size={14} /> Estado</span>
                  <strong class="display small">{current.installed ? "Instalada" : "Pendiente"}</strong>
                  <span class="hint">{current.installed ? "Todo descargado en este PC" : "Se descarga al pulsar Jugar"}</span>
                </div>
              </div>

              <div class="location">
                <Icon name="folder" size={16} />
                <span class="mono path">{app.info?.data_dir}/instances/{current.id}</span>
                <button class="btn sm" onclick={openFolder}>Abrir</button>
              </div>

              {#if exportedPath}
                <div class="alert info">
                  <Icon name="check" size={16} />
                  <span>Plantilla guardada en <span class="mono">{exportedPath}</span>. Súbela a tu servicio de manifiestos y rellena las URLs de cada mod.</span>
                </div>
              {/if}

              <details class="explain">
                <summary>¿Por qué en otro PC pone «se descargará al jugar»?</summary>
                <p>
                  Cada instancia es una carpeta independiente (<span class="mono">instance.json</span>, mods, config,
                  mundos…). Minecraft, librerías, assets y Java son compartidos y viven fuera de ella. Si copias solo
                  la carpeta de la instancia a otro PC, faltan esos archivos comunes: pulsa <strong>Jugar</strong> y
                  Obsidia los descarga. Para evitarlo, copia también <span class="mono">versions</span>,
                  <span class="mono">libraries</span>, <span class="mono">assets</span> y <span class="mono">java</span>
                  del directorio de datos.
                </p>
              </details>
            {:else if nav.instanceTab === "mods"}
              <ModsPanel instance={current} />
            {:else if nav.instanceTab === "settings"}
              <InstanceSettingsPanel instance={current} />
            {:else}
              <LogView id={current.id} height="calc(100vh - 380px)" />
            {/if}
          </div>
        </div>
      {/key}
    {:else}
      <div class="empty">
        <div class="empty-art obsidia-bg facet"></div>
        <h1>Aún no tienes instancias</h1>
        <p class="muted">Una instancia es un Minecraft independiente con su versión, mods y mundos.</p>
        <div class="empty-actions">
          <button class="btn primary" onclick={() => (creating = true)}><Icon name="plus" size={16} /> Crear instancia</button>
          <button class="btn" onclick={() => (importing = true)}>Importar por código</button>
        </div>
      </div>
    {/if}
  </section>
</div>

{#if creating}
  <CreateInstanceModal
    onclose={(id) => {
      creating = false;
      if (id) nav.openInstance(id);
    }}
  />
{/if}
{#if importing}
  <ImportByCodeModal
    onclose={() => {
      importing = false;
      if (app.importedId) nav.openInstance(app.importedId);
    }}
  />
{/if}

<style>
  .library {
    display: grid;
    grid-template-columns: 290px minmax(0, 1fr);
    height: 100%;
  }

  /* ── Lista ── */
  .list {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    padding: 18px 12px 12px 16px;
    border-right: 1px solid var(--border);
    background: rgb(var(--surface) / 0.55);
  }
  .list-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 4px;
  }
  .list-head h1 {
    font-size: 20px;
  }
  .count {
    font-size: 12px;
    color: var(--muted);
    padding: 1px 7px;
    border-radius: 6px;
    background: rgb(255 255 255 / 0.06);
  }
  .spacer {
    flex: 1;
  }
  .add-wrap {
    position: relative;
  }
  .add-menu {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 30;
    width: 250px;
    padding: 6px;
    background: rgb(var(--surface-2) / 1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 20px 50px -20px rgb(0 0 0 / 0.8);
  }
  .add-menu button {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    padding: 9px;
    border: none;
    border-radius: 8px;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text);
  }
  .add-menu button:hover {
    background: rgb(255 255 255 / 0.06);
  }
  .add-menu button > span {
    display: flex;
    flex-direction: column;
  }
  .add-menu strong {
    font-size: 13px;
  }
  .add-menu :global(svg) {
    margin-top: 2px;
    color: var(--accent-hover);
  }
  .search {
    position: relative;
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
  }
  .rows {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-right: -4px;
    padding-right: 4px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px 7px 7px;
    border-radius: 10px;
    border: 1px solid transparent;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text);
    transition: background var(--ease), border-color var(--ease);
  }
  .row:hover {
    background: rgb(255 255 255 / 0.04);
  }
  .row.active {
    background: color-mix(in srgb, var(--accent) 11%, transparent);
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
  }
  .row.active::before {
    content: "";
    position: absolute;
    left: -9px;
    top: 12px;
    bottom: 12px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .thumb {
    width: 48px;
    height: 48px;
    flex: none;
    border-radius: 9px;
    background-size: cover;
    background-position: center;
    border: 1px solid rgb(255 255 255 / 0.08);
  }
  .row-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .row-name {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-name :global(svg) {
    flex: none;
    color: var(--accent-hover);
    fill: currentColor;
  }
  .row-meta {
    font-size: 12px;
    color: var(--muted);
  }
  .pad {
    padding: 8px;
  }
  .broken {
    max-height: 160px;
    overflow-y: auto;
  }

  /* ── Detalle ── */
  .detail {
    min-height: 0;
    overflow-y: auto;
  }
  .detail-inner {
    padding: 18px 24px 28px;
  }
  .banner {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    height: 210px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background-size: cover;
    background-position: center 35%;
    border: 1px solid var(--border);
  }
  .banner-shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(0deg, rgb(9 8 13 / 0.95) 0%, rgb(9 8 13 / 0.45) 50%, rgb(9 8 13 / 0.15) 100%);
  }
  .banner > :not(.banner-shade) {
    position: relative;
  }
  .banner-top {
    display: flex;
    justify-content: space-between;
    padding: 14px 16px;
  }
  .banner-top .tag {
    background: rgb(9 8 13 / 0.6);
  }
  .tag.action {
    cursor: pointer;
  }
  .tag.action:hover {
    color: var(--text);
    border-color: var(--border-strong);
  }
  .banner-main {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 0 20px 18px;
  }
  .banner-text {
    min-width: 0;
  }
  .name {
    font-size: clamp(26px, 3vw, 38px);
    letter-spacing: -0.03em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    color: var(--text-2);
    font-size: 13px;
  }
  .banner-actions {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .tabs {
    margin-top: 14px;
  }
  .tab-body {
    padding-top: 16px;
  }
  .run-error {
    margin-bottom: 14px;
    align-items: center;
  }
  .run-error span {
    flex: 1;
  }

  /* Resumen: cuadrícula con una ficha grande */
  .stats {
    display: grid;
    grid-template-columns: 1.5fr repeat(2, 1fr);
    grid-auto-rows: minmax(104px, auto);
    gap: 12px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: rgb(var(--surface) / var(--surface-alpha));
    border: 1px solid var(--border);
  }
  .stat.big {
    grid-row: span 2;
    justify-content: flex-end;
    background:
      radial-gradient(circle at 100% 0%, color-mix(in srgb, var(--accent) 22%, transparent), transparent 60%),
      rgb(var(--surface) / var(--surface-alpha));
  }
  .stat.big strong {
    font-size: 40px;
    line-height: 1.05;
  }
  .stat-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
    margin-bottom: auto;
  }
  .stat strong {
    font-size: 24px;
    letter-spacing: -0.02em;
  }
  .stat strong.small {
    font-size: 17px;
  }
  .link {
    align-self: flex-start;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--accent-hover);
    font-size: 12px;
    font-weight: 600;
  }
  .location {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px dashed var(--border-strong);
    color: var(--muted);
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-2);
  }
  .location + .alert {
    margin-top: 12px;
  }
  .explain {
    margin-top: 14px;
    color: var(--muted);
    font-size: 13px;
  }
  .explain summary {
    cursor: pointer;
    font-weight: 600;
  }
  .explain p {
    margin-top: 8px;
    line-height: 1.6;
    max-width: 760px;
  }
  .explain strong {
    color: var(--text);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    height: 100%;
    padding: 40px;
    text-align: center;
  }
  .empty-art {
    --facet: 18px;
    width: 120px;
    height: 120px;
    margin-bottom: 8px;
  }
  .empty-actions {
    display: flex;
    gap: 10px;
    margin-top: 6px;
  }
  @media (max-width: 1100px) {
    .stats {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
