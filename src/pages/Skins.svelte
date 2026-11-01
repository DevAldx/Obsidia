<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { toasts } from "../lib/toast.svelte";
  import { errorText, formatDay } from "../lib/format";
  import { readAsDataUrl, base64Of } from "../lib/files";
  import type { CapeInfo, LibrarySkin, SkinProfile, SkinVariant } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import SkinViewer, { loadImage } from "../components/SkinViewer.svelte";

  const isMicrosoft = $derived(app.account?.kind === "microsoft");

  let profile = $state<SkinProfile | null>(null);
  let library = $state<LibrarySkin[]>([]);
  let loading = $state(false);
  let loadError = $state("");
  /** Acción en curso (bloquea el resto de botones; Mojang limita los cambios seguidos). */
  let busy = $state<string | null>(null);

  // Skin elegida del disco, pendiente de aplicar.
  let pending = $state<{ dataUrl: string; name: string; variant: SkinVariant } | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);

  const active = $derived(profile?.skins.find((s) => s.active) ?? null);
  const activeCape = $derived(profile?.capes.find((c) => c.active) ?? null);
  const shownSkin = $derived(pending?.dataUrl ?? active?.url ?? null);
  const shownVariant = $derived<SkinVariant>(pending?.variant ?? active?.variant ?? "classic");

  async function load() {
    loading = true;
    loadError = "";
    try {
      const [p, lib] = await Promise.all([api.getSkinProfile(), api.listSkinLibrary().catch(() => [])]);
      profile = p;
      library = lib;
    } catch (e) {
      loadError = errorText(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    if (isMicrosoft) void load();
  });

  async function run(label: string, action: () => Promise<SkinProfile>, ok: string) {
    busy = label;
    try {
      profile = await action();
      void app.refreshAccount(); // la cara del menú de cuenta
      toasts.success(ok);
      return true;
    } catch (e) {
      toasts.error(e);
      return false;
    } finally {
      busy = null;
    }
  }

  /** Las skins slim dejan transparente la columna 54-55 del brazo derecho. */
  async function guessVariant(dataUrl: string): Promise<SkinVariant> {
    try {
      const img = await loadImage(dataUrl);
      if (img.width !== 64 || img.height !== 64) return "classic";
      const c = document.createElement("canvas");
      c.width = 64;
      c.height = 64;
      const ctx = c.getContext("2d");
      if (!ctx) return "classic";
      ctx.drawImage(img, 0, 0);
      const px = ctx.getImageData(54, 20, 2, 12).data;
      for (let i = 3; i < px.length; i += 4) if (px[i] !== 0) return "classic";
      return "slim";
    } catch {
      return "classic";
    }
  }

  async function onFileChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    if (!file.name.toLowerCase().endsWith(".png") && file.type !== "image/png") {
      return toasts.push("error", "La skin debe ser un PNG.");
    }
    if (file.size > 256 * 1024) return toasts.push("error", "La skin supera los 256 KB.");
    const dataUrl = await readAsDataUrl(file);
    const img = await loadImage(dataUrl).catch(() => null);
    if (!img || img.width !== 64 || (img.height !== 64 && img.height !== 32)) {
      return toasts.push("error", "Una skin debe medir 64×64 o 64×32 píxeles.");
    }
    pending = { dataUrl, name: file.name.replace(/\.png$/i, ""), variant: await guessVariant(dataUrl) };
  }

  async function applyPending() {
    if (!pending) return;
    const p = pending;
    const done = await run("upload", () => api.uploadSkin(p.name, p.variant, base64Of(p.dataUrl)), "Skin aplicada");
    if (done) {
      pending = null;
      library = await api.listSkinLibrary().catch(() => library);
    }
  }

  async function applyLibrary(s: LibrarySkin) {
    await run(`lib-${s.id}`, () => api.applyLibrarySkin(s.id), `Skin «${s.name}» aplicada`);
  }

  async function removeLibrary(s: LibrarySkin) {
    try {
      await api.deleteLibrarySkin(s.id);
      library = library.filter((x) => x.id !== s.id);
    } catch (e) {
      toasts.error(e);
    }
  }

  async function chooseCape(cape: CapeInfo | null) {
    if ((cape?.id ?? null) === (activeCape?.id ?? null)) return;
    await run(`cape-${cape?.id ?? "none"}`, () => api.setCape(cape?.id ?? null), cape ? `Capa «${cape.alias}» activada` : "Capa oculta");
  }

  /** Miniatura de la parte trasera de una capa (la que se ve en el juego). */
  function capeThumb(node: HTMLCanvasElement, url: string) {
    const paint = (src: string) => {
      const ctx = node.getContext("2d");
      if (!ctx) return;
      loadImage(src)
        .then((img) => {
          const k = img.width / 64;
          ctx.imageSmoothingEnabled = false;
          ctx.clearRect(0, 0, node.width, node.height);
          ctx.drawImage(img, k, k, 10 * k, 16 * k, 0, 0, node.width, node.height);
        })
        .catch(() => {});
    };
    paint(url);
    return { update: paint };
  }
</script>

{#if !app.account || !isMicrosoft}
  <div class="locked facet">
    <Icon name="user" size={26} />
    <div>
      <strong class="display">Necesitas una cuenta de Microsoft</strong>
      <p class="muted">
        {#if app.account}
          Tu cuenta local no tiene perfil en Mojang, así que su skin y su capa no se pueden cambiar
          (otros jugadores verían la skin por defecto igualmente).
        {:else}
          Las skins y capas se guardan en tu perfil de Minecraft: inicia sesión para cambiarlas.
        {/if}
      </p>
    </div>
    <button class="btn primary" onclick={() => app.startLegacySignIn()} disabled={app.signInBusy}>
      {app.signInBusy ? "Esperando a Microsoft…" : "Iniciar sesión con Microsoft"}
    </button>
  </div>
{:else if loading && !profile}
  <div class="layout">
    <div class="skeleton stage-skel"></div>
    <div class="side"><div class="skeleton" style="height: 120px"></div><div class="skeleton" style="height: 160px"></div></div>
  </div>
{:else if loadError && !profile}
  <div class="alert">
    <Icon name="alert" size={16} />
    <span>{loadError}</span>
    <button class="btn sm" onclick={load}>Reintentar</button>
  </div>
{:else if profile}
  <div class="layout">
    <!-- Escenario con la skin de frente y de espaldas -->
    <section class="stage">
      <div class="stage-top">
        {#if pending}
          <span class="tag ember">Vista previa · sin aplicar</span>
        {:else}
          <span class="tag accent">Skin actual</span>
        {/if}
        <span class="tag">{shownVariant === "slim" ? "Slim (Alex)" : "Clásico (Steve)"}</span>
      </div>
      <div class="figures">
        <SkinViewer skin={shownSkin} variant={shownVariant} view="front" height={250} />
        <SkinViewer skin={shownSkin} variant={shownVariant} cape={activeCape?.url ?? null} view="back" height={250} />
      </div>
      <p class="who display">{profile.username}</p>
      {#if !pending}
        <button class="btn ghost sm" onclick={() => run("reset", () => api.resetSkin(), "Skin restablecida")} disabled={busy !== null}>
          <Icon name="refresh" size={14} /> Volver a la skin por defecto
        </button>
      {/if}
    </section>

    <div class="side">
      <!-- Subir -->
      <section class="block">
        <h2>Cambiar skin</h2>
        <input bind:this={fileInput} type="file" accept="image/png,.png" hidden onchange={onFileChosen} />
        {#if pending}
          <label class="field">
            <span>Nombre en tu biblioteca</span>
            <input bind:value={pending.name} maxlength="40" />
          </label>
          <p class="label">Modelo de brazos</p>
          <div class="variants">
            {#each [["classic", "Clásico", "Brazos de 4 px"], ["slim", "Slim", "Brazos de 3 px"]] as [id, title, text] (id)}
              <button class="variant" class:active={pending.variant === id} onclick={() => pending && (pending.variant = id as SkinVariant)}>
                <strong>{title}</strong><span class="hint">{text}</span>
              </button>
            {/each}
          </div>
          <div class="row-actions">
            <button class="btn ghost" onclick={() => (pending = null)} disabled={busy === "upload"}>Cancelar</button>
            <button class="btn primary" onclick={applyPending} disabled={busy !== null}>
              {busy === "upload" ? "Subiendo…" : "Aplicar skin"}
            </button>
          </div>
        {:else}
          <button class="drop" onclick={() => fileInput?.click()} disabled={busy !== null}>
            <Icon name="upload" size={22} />
            <strong>Elegir un PNG…</strong>
            <span class="hint">64×64 o 64×32. Verás la vista previa antes de aplicarla.</span>
          </button>
        {/if}
      </section>

      <!-- Capas -->
      <section class="block">
        <h2>Capa</h2>
        {#if profile.capes.length === 0}
          <p class="hint">Esta cuenta no tiene capas. Las capas oficiales se consiguen en eventos de Mojang o con Minecraft Legacy (Migrator).</p>
        {:else}
          <div class="capes">
            <button class="cape" class:active={!activeCape} onclick={() => chooseCape(null)} disabled={busy !== null} title="Sin capa">
              <span class="cape-none"><Icon name="close" size={18} /></span>
              <span class="cape-name">Ninguna</span>
            </button>
            {#each profile.capes as c (c.id)}
              <button class="cape" class:active={c.active} onclick={() => chooseCape(c)} disabled={busy !== null} title={c.alias}>
                <canvas width="40" height="64" use:capeThumb={c.url}></canvas>
                <span class="cape-name">{c.alias || "Capa"}</span>
              </button>
            {/each}
          </div>
        {/if}
      </section>
    </div>
  </div>

  <!-- Biblioteca -->
  <section class="library">
    <div class="section-head">
      <h3 class="display">Mi biblioteca</h3>
      <span class="hint">Las skins que subes se guardan aquí para volver a ponerlas con un clic.</span>
    </div>
    {#if library.length === 0}
      <p class="hint empty">Todavía no hay skins guardadas.</p>
    {:else}
      <div class="lib-grid">
        {#each library as s (s.id)}
          <div class="lib-item">
            <SkinViewer skin={`data:image/png;base64,${s.data_base64}`} variant={s.variant} height={120} />
            <strong title={s.name}>{s.name}</strong>
            <span class="hint">{s.variant === "slim" ? "Slim" : "Clásico"} · {formatDay(new Date(s.added_at * 1000).toISOString().slice(0, 10))}</span>
            <div class="lib-actions">
              <button class="btn sm primary" onclick={() => applyLibrary(s)} disabled={busy !== null}>
                {busy === `lib-${s.id}` ? "…" : "Usar"}
              </button>
              <button class="btn sm ghost icon" title="Quitar de la biblioteca" onclick={() => removeLibrary(s)}><Icon name="trash" size={14} /></button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
{/if}

<style>
  .locked {
    --facet: 14px;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 22px;
    background: rgb(var(--surface-2) / 1);
  }
  .locked > :global(svg) {
    color: var(--accent-hover);
    flex: none;
  }
  .locked div {
    flex: 1;
  }
  .locked strong {
    font-size: 17px;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(300px, 1fr) minmax(280px, 1fr);
    gap: 16px;
  }
  .stage-skel {
    height: 380px;
    border-radius: var(--radius-lg);
  }
  .stage {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 16px 16px 18px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background:
      radial-gradient(circle at 50% 55%, color-mix(in srgb, var(--accent) 22%, transparent), transparent 60%),
      repeating-linear-gradient(30deg, rgb(255 255 255 / 0.025) 0 1px, transparent 1px 34px),
      repeating-linear-gradient(150deg, rgb(255 255 255 / 0.025) 0 1px, transparent 1px 34px),
      rgb(var(--surface) / var(--surface-alpha));
  }
  .stage-top {
    align-self: stretch;
    display: flex;
    gap: 6px;
  }
  .figures {
    display: flex;
    gap: 34px;
    padding: 12px 0 4px;
  }
  .who {
    font-size: 20px;
    font-weight: 700;
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .block {
    padding: 16px 18px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
  }
  .drop {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 22px 14px;
    border-radius: var(--radius);
    border: 1px dashed var(--border-strong);
    background: rgb(255 255 255 / 0.02);
    color: var(--text);
    cursor: pointer;
    transition: border-color var(--ease), background var(--ease);
  }
  .drop:hover:not(:disabled) {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }
  .drop :global(svg) {
    color: var(--accent-hover);
  }
  .label {
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 6px;
  }
  .variants {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .variant {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: none;
    color: var(--text);
    cursor: pointer;
  }
  .variant.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .row-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .capes {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .cape {
    width: 78px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 10px 6px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: rgb(255 255 255 / 0.03);
    color: var(--text);
    cursor: pointer;
  }
  .cape:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  .cape.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .cape canvas {
    width: 30px;
    height: 48px;
    image-rendering: pixelated;
  }
  .cape-none {
    width: 30px;
    height: 48px;
    display: grid;
    place-items: center;
    border: 1px dashed var(--border-strong);
    color: var(--muted);
  }
  .cape-name {
    width: 100%;
    font-size: 11px;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .library {
    margin-top: 24px;
  }
  .library h3 {
    font-size: 17px;
  }
  .empty {
    padding: 14px 0;
  }
  .lib-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: 10px;
  }
  .lib-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 12px 8px 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
    min-width: 0;
  }
  .lib-item strong {
    max-width: 100%;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 6px;
  }
  .lib-actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .alert span {
    flex: 1;
  }
  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
</style>
