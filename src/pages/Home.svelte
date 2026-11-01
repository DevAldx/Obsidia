<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { nav } from "../lib/nav.svelte";
  import { news } from "../lib/news.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { lazyBackground } from "../lib/lazyBackground";
  import { formatDay, formatLastPlayed, formatPlaytime, javaLabel, loaderLabel } from "../lib/format";
  import type { NewsItem } from "../lib/types";
  import PlayButton from "../components/PlayButton.svelte";
  import Icon from "../components/Icon.svelte";
  import CreateInstanceModal from "../components/CreateInstanceModal.svelte";
  import ImportByCodeModal from "../components/ImportByCodeModal.svelte";

  const inst = $derived(app.selected);
  const bg = $derived(inst ? app.backgrounds[inst.id] : undefined);
  const running = $derived(inst ? app.games[inst.id] : undefined);

  let switcherOpen = $state(false);
  let switcherEl = $state<HTMLDivElement | null>(null);
  let creating = $state(false);
  let importing = $state(false);
  const ms = $derived(app.settings?.ui.animations === false ? 0 : 140);

  // Las otras instancias, por uso reciente, para «Continuar».
  const recent = $derived(
    [...app.list.instances]
      .filter((i) => i.id !== inst?.id)
      .sort((a, b) => (b.last_played ?? 0) - (a.last_played ?? 0))
      .slice(0, 4),
  );

  $effect(() => {
    if (inst) void app.ensureBackground(inst.id);
  });

  onMount(() => {
    void news.load();
  });

  const featured = $derived(news.articles[0]);
  const secondary = $derived(news.articles.slice(1, 3));
  const versions = $derived(news.versions.slice(0, 4));

  async function openFolder() {
    if (!inst) return;
    try {
      await api.openInstanceFolder(inst.id, "");
    } catch (e) {
      toasts.error(e, "No se pudo abrir la carpeta");
    }
  }

  async function openArticle(item: NewsItem) {
    if (!item.link) return;
    try {
      await api.openHttpsUrl(item.link);
    } catch (e) {
      toasts.error(e, "No se pudo abrir el navegador");
    }
  }

  function onWindowClick(e: MouseEvent) {
    if (switcherOpen && switcherEl && !switcherEl.contains(e.target as Node)) switcherOpen = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (switcherOpen = false)} />

<div class="home">
  <!-- ───────────── Escenario: la instancia seleccionada ───────────── -->
  <section class="stage" class:obsidia-bg={!bg} style:background-image={bg ? `url(${bg})` : undefined}>
    <div class="shade"></div>

    {#if inst}
      <div class="stage-main">
        <div class="tags">
          {#if running === "running"}
            <span class="tag ok"><span class="dot live"></span> En juego</span>
          {:else if inst.installed}
            <span class="tag ok" title="Minecraft y Java ya están descargados en este PC.">Lista para jugar</span>
          {:else}
            <span class="tag ember" title="Minecraft, librerías y Java se descargan en este PC al pulsar Jugar.">Se descargará al jugar</span>
          {/if}
          <span class="tag">{loaderLabel(inst)}</span>
          <span class="tag">Minecraft {inst.minecraft_version}</span>
        </div>

        <h1 class="title">{inst.name}</h1>

        <p class="meta">
          <span><Icon name="puzzle" size={14} /> {inst.mod_count} mods</span>
          <span><Icon name="coffee" size={14} /> {javaLabel(inst, app.java)}</span>
          <span><Icon name="clock" size={14} /> {formatPlaytime(inst.total_playtime_secs)} · {formatLastPlayed(inst.last_played)}</span>
        </p>

        <div class="launch">
          <div class="split" bind:this={switcherEl}>
            <PlayButton id={inst.id} big />
            <button
              class="switch facet"
              onclick={() => (switcherOpen = !switcherOpen)}
              aria-haspopup="listbox"
              aria-expanded={switcherOpen}
              title="Cambiar de instancia"
            >
              <Icon name="chevronDown" size={18} />
            </button>
            {#if switcherOpen}
              <div class="switcher" role="listbox" transition:fly={{ y: 6, duration: ms }}>
                <p class="eyebrow">Cambiar instancia</p>
                {#each app.list.instances as i (i.id)}
                  <button
                    role="option"
                    aria-selected={i.id === inst.id}
                    class:current={i.id === inst.id}
                    onclick={() => {
                      void app.select(i.id);
                      switcherOpen = false;
                    }}
                  >
                    <span class="sw-name">{i.name}</span>
                    <span class="sw-meta">{i.minecraft_version} · {loaderLabel(i)}</span>
                    {#if i.id === inst.id}<Icon name="check" size={15} />{/if}
                  </button>
                {/each}
                <button class="sw-new" onclick={() => { switcherOpen = false; creating = true; }}>
                  <Icon name="plus" size={15} /> Nueva instancia
                </button>
              </div>
            {/if}
          </div>
          <div class="quick">
            <button class="glass-btn" title="Mods" onclick={() => nav.openInstance(inst.id, "mods")}><Icon name="puzzle" size={17} /></button>
            <button class="glass-btn" title="Ajustes de la instancia" onclick={() => nav.openInstance(inst.id, "settings")}><Icon name="settings" size={17} /></button>
            <button class="glass-btn" title="Abrir carpeta" onclick={openFolder}><Icon name="folder" size={17} /></button>
          </div>
        </div>

        {#if app.errors[inst.id]}
          <div class="alert stage-alert">
            <Icon name="alert" size={16} />
            <span>{app.errors[inst.id]}</span>
            <button class="btn sm ghost" onclick={() => nav.openInstance(inst.id, "logs")}>Ver registros</button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="stage-main empty">
        <span class="eyebrow">Bienvenido a Obsidia</span>
        <h1 class="title">Tu primera instancia<br />empieza aquí.</h1>
        <p class="meta">Elige versión y loader; Obsidia descarga Minecraft y Java por ti.</p>
        <div class="launch">
          <button class="btn primary big-cta facet" onclick={() => (creating = true)}>
            <Icon name="plus" size={18} /> Crear instancia
          </button>
          <button class="btn" onclick={() => (importing = true)}>Importar por código</button>
        </div>
      </div>
    {/if}

    <!-- Columna lateral: continuar con otras instancias + cuenta -->
    {#if recent.length > 0 || !app.account}
      <aside class="rail">
        {#if !app.account}
          <div class="rail-callout">
            <Icon name="user" size={18} />
            <div>
              <strong>Sin sesión</strong>
              <p class="hint">Inicia sesión para jugar online.</p>
            </div>
            <button class="btn sm primary" onclick={() => app.startLegacySignIn()} disabled={app.signInBusy}>Entrar</button>
          </div>
        {/if}
        {#if recent.length > 0}
          <p class="rail-title">Continuar</p>
          {#each recent as r (r.id)}
            {@const rbg = app.backgrounds[r.id]}
            <button class="rail-item" onclick={() => app.select(r.id)} use:lazyBackground={r.id}>
              <span class="thumb" class:obsidia-bg={!rbg} style:background-image={rbg ? `url(${rbg})` : undefined}></span>
              <span class="rail-text">
                <span class="rail-name">{r.name}</span>
                <span class="rail-meta">{r.minecraft_version} · {formatLastPlayed(r.last_played)}</span>
              </span>
              {#if app.games[r.id]}<span class="dot live"></span>{/if}
            </button>
          {/each}
          <button class="rail-all" onclick={() => nav.go("instances")}>
            Ver todas <Icon name="chevronRight" size={14} />
          </button>
        {/if}
      </aside>
    {/if}
  </section>

  <!-- ───────────── Novedades ───────────── -->
  <section class="news">
    <div class="section-head">
      <div>
        <p class="eyebrow">Novedades</p>
        <h3 class="display news-title">Minecraft: Java Edition</h3>
      </div>
      <div class="news-actions">
        {#if news.feed?.stale}<span class="tag"><Icon name="wifiOff" size={13} /> Copia guardada</span>{/if}
        <button class="btn ghost sm" onclick={() => nav.go("discover")}>Ver todo <Icon name="chevronRight" size={14} /></button>
      </div>
    </div>

    {#if news.loading && !news.feed}
      <div class="news-grid">
        <div class="skeleton n-featured"></div>
        <div class="n-stack"><div class="skeleton"></div><div class="skeleton"></div></div>
        <div class="skeleton n-versions"></div>
      </div>
    {:else if !news.feed}
      <div class="alert info">
        <Icon name="wifiOff" size={16} />
        <span>No se pudieron cargar las noticias. {news.error}</span>
        <button class="btn sm" onclick={() => news.load(true)}>Reintentar</button>
      </div>
    {:else}
      <div class="news-grid">
        {#if featured}
          <button class="n-featured" onclick={() => openArticle(featured)} disabled={!featured.link}>
            {#if featured.image_url}
              <img src={featured.image_url} alt="" loading="lazy" decoding="async" width="772" height="350" />
            {/if}
            <span class="n-shade"></span>
            <span class="n-body">
              <span class="tag accent">Destacado</span>
              <strong>{featured.title}</strong>
              <span class="n-sum">{featured.summary}</span>
            </span>
          </button>
        {/if}

        <div class="n-stack">
          {#each secondary as item (item.id)}
            <button class="n-small" onclick={() => openArticle(item)} disabled={!item.link}>
              {#if item.image_url}
                <img src={item.image_url} alt="" loading="lazy" decoding="async" width="160" height="100" />
              {:else}
                <span class="n-noimg obsidia-bg"></span>
              {/if}
              <span class="n-small-body">
                <span class="n-date">{formatDay(item.date)}</span>
                <strong>{item.title}</strong>
              </span>
            </button>
          {/each}
        </div>

        <div class="n-versions">
          <p class="rail-title">Últimas versiones</p>
          {#each versions as v (v.id)}
            <div class="v-row">
              <span class="tag" class:accent={v.kind === "release"}>{v.kind === "release" ? "Release" : "Snapshot"}</span>
              <span class="v-text">
                <span class="v-name">{v.version ?? v.title}</span>
                <span class="n-date">{formatDay(v.date)}</span>
              </span>
            </div>
          {:else}
            <p class="hint">Sin datos de versiones.</p>
          {/each}
        </div>
      </div>
    {/if}
  </section>
</div>

{#if creating}
  <CreateInstanceModal
    onclose={(id) => {
      creating = false;
      if (id) void app.select(id);
    }}
  />
{/if}
{#if importing}
  <ImportByCodeModal onclose={() => (importing = false)} />
{/if}

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 22px;
    padding: 20px 24px 28px;
    min-height: 100%;
  }

  /* ── Escenario ── */
  .stage {
    position: relative;
    display: flex;
    align-items: stretch;
    min-height: clamp(360px, 60vh, 560px);
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background-size: cover;
    background-position: center;
    overflow: hidden;
  }
  .shade {
    position: absolute;
    inset: 0;
    background:
      linear-gradient(90deg, rgb(9 8 13 / 0.92) 0%, rgb(9 8 13 / 0.55) 45%, rgb(9 8 13 / 0.15) 75%, rgb(9 8 13 / 0.5) 100%),
      linear-gradient(0deg, rgb(9 8 13 / 0.85) 0%, transparent 55%);
    pointer-events: none;
  }
  .stage-main {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 14px;
    padding: clamp(22px, 4vw, 40px);
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .title {
    font-size: clamp(34px, 5.2vw, 64px);
    font-weight: 700;
    letter-spacing: -0.035em;
    line-height: 1;
    max-width: 16ch;
    overflow-wrap: anywhere;
    text-shadow: 0 2px 30px rgb(0 0 0 / 0.5);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    color: var(--text-2);
    font-size: 13px;
  }
  .meta span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .launch {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 14px;
    margin-top: 8px;
  }
  .split {
    position: relative;
    display: flex;
    gap: 3px;
  }
  .switch {
    --facet: 10px;
    width: 46px;
    height: 56px;
    display: grid;
    place-items: center;
    border: none;
    cursor: pointer;
    background: color-mix(in srgb, var(--accent) 70%, #000);
    color: var(--accent-ink);
    transition: background var(--ease);
  }
  .switch:hover {
    background: var(--accent-hover);
  }
  .switcher {
    position: absolute;
    left: 0;
    bottom: calc(100% + 10px);
    z-index: 30;
    width: 320px;
    max-height: 340px;
    overflow-y: auto;
    padding: 10px;
    background: rgb(var(--surface-2) / 1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 24px 60px -20px rgb(0 0 0 / 0.85);
  }
  .switcher .eyebrow {
    padding: 4px 8px 8px;
  }
  .switcher button {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    column-gap: 10px;
    align-items: center;
    width: 100%;
    padding: 8px;
    border: none;
    border-radius: 8px;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text);
  }
  .switcher button:hover {
    background: rgb(255 255 255 / 0.06);
  }
  .switcher button.current {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }
  .switcher button :global(svg) {
    grid-row: 1 / span 2;
    grid-column: 2;
    color: var(--accent-hover);
  }
  .sw-name {
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sw-meta {
    font-size: 12px;
    color: var(--muted);
  }
  .switcher .sw-new {
    display: flex;
    gap: 8px;
    margin-top: 6px;
    border-top: 1px solid var(--border);
    border-radius: 0 0 8px 8px;
    color: var(--text-2);
    font-size: 13px;
  }
  .switcher .sw-new :global(svg) {
    color: inherit;
  }
  .quick {
    display: flex;
    gap: 8px;
  }
  .stage-alert {
    max-width: 640px;
    align-items: center;
  }
  .stage-alert span {
    flex: 1;
  }
  .empty .title {
    font-size: clamp(30px, 4.4vw, 54px);
  }
  .big-cta {
    height: 52px;
    padding: 0 26px;
    font-size: 15px;
    border-radius: 0;
  }

  /* ── Columna lateral ── */
  .rail {
    position: relative;
    width: 270px;
    flex: none;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 6px;
    padding: 20px 18px;
    background: linear-gradient(270deg, rgb(9 8 13 / 0.78), rgb(9 8 13 / 0.35));
    border-left: 1px solid rgb(255 255 255 / 0.06);
  }
  .rail-title {
    font-size: 11px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--muted);
    margin: 6px 0 4px;
  }
  .rail-callout {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px;
    margin-bottom: auto;
    border-radius: var(--radius);
    border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, rgb(9 8 13 / 0.6));
  }
  .rail-callout > div {
    flex: 1;
    min-width: 0;
  }
  .rail-callout strong {
    font-size: 13px;
  }
  .rail-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px;
    border-radius: 10px;
    border: 1px solid transparent;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text);
    transition: background var(--ease), border-color var(--ease);
  }
  .rail-item:hover {
    background: rgb(255 255 255 / 0.06);
    border-color: var(--border);
  }
  .thumb {
    width: 54px;
    height: 38px;
    flex: none;
    border-radius: 7px;
    background-size: cover;
    background-position: center;
    border: 1px solid rgb(255 255 255 / 0.1);
  }
  .rail-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .rail-name {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rail-meta {
    font-size: 11px;
    color: var(--muted);
  }
  .rail-all {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    align-self: flex-start;
    margin-top: 4px;
    padding: 4px 6px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--accent-hover);
    font-size: 12px;
    font-weight: 600;
  }

  /* ── Noticias ── */
  .news-title {
    font-size: 18px;
    margin-top: 2px;
  }
  .news-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .news-grid {
    display: grid;
    grid-template-columns: minmax(0, 1.55fr) minmax(0, 1fr) minmax(210px, 0.8fr);
    gap: 14px;
    min-height: 230px;
  }
  .news-grid button {
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
    color: var(--text);
    cursor: pointer;
    text-align: left;
    padding: 0;
    overflow: hidden;
    transition: border-color var(--ease), transform var(--ease);
  }
  .news-grid button:hover:not(:disabled) {
    border-color: var(--border-strong);
    transform: translateY(-2px);
  }
  .news-grid button:disabled {
    cursor: default;
  }
  .n-featured {
    position: relative;
    display: flex;
    align-items: flex-end;
    min-height: 230px;
    border-radius: var(--radius-lg);
  }
  .n-featured img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .n-shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(0deg, rgb(9 8 13 / 0.95) 0%, rgb(9 8 13 / 0.4) 55%, transparent 100%);
  }
  .n-body {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 18px 20px;
  }
  .n-body strong {
    font-family: var(--font-display);
    font-size: 20px;
    line-height: 1.2;
  }
  .n-sum {
    color: var(--text-2);
    font-size: 13px;
  }
  .n-stack {
    display: grid;
    grid-template-rows: 1fr 1fr;
    gap: 14px;
  }
  .n-small {
    display: flex;
    border-radius: var(--radius);
  }
  .n-small img,
  .n-noimg {
    width: 42%;
    flex: none;
    height: 100%;
    object-fit: cover;
  }
  .n-small-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    min-width: 0;
  }
  .n-small-body strong {
    font-size: 13px;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .n-date {
    font-size: 11px;
    color: var(--muted);
  }
  .n-versions {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 14px;
    border-left: 2px solid var(--border-strong);
  }
  .v-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 0;
    border-bottom: 1px dashed var(--border);
  }
  .v-row:last-child {
    border-bottom: none;
  }
  .v-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .v-name {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n-featured.skeleton,
  .n-versions.skeleton {
    min-height: 230px;
    border-left: none;
  }

  @media (max-width: 1080px) {
    .rail {
      width: 230px;
    }
    .news-grid {
      grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    }
    .n-versions {
      grid-column: 1 / -1;
      flex-direction: row;
      flex-wrap: wrap;
      gap: 6px 20px;
      border-left: none;
      border-top: 2px solid var(--border-strong);
    }
    .n-versions .rail-title {
      width: 100%;
    }
    .v-row {
      border-bottom: none;
    }
  }
</style>
