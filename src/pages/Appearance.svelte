<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { accentColors, normalizeHex } from "../lib/color";
  import { toasts } from "../lib/toast.svelte";
  import { BACKGROUND_ACCEPT, BACKGROUND_TYPES, MAX_BACKGROUND_BYTES, base64Of, mimeOf, readAsDataUrl } from "../lib/files";
  import { errorText } from "../lib/format";
  import type { Settings, Theme } from "../lib/types";
  import Icon from "../components/Icon.svelte";
  import Logo from "../components/Logo.svelte";

  const DEFAULT_UI: Settings["ui"] = {
    theme: "dark",
    animations: true,
    transparency: true,
    accent: null,
    background_fit: "cover",
    background_blur: 0,
    background_dim: 40,
  };

  // El fondo y el icono se guardan al elegirlos; colores y efectos, con «Guardar».
  let uiDraft = $state<Settings["ui"]>({ ...(app.settings?.ui ?? DEFAULT_UI) });
  let saving = $state(false);
  let error = $state("");

  let iconInput = $state<HTMLInputElement | null>(null);
  let bgInput = $state<HTMLInputElement | null>(null);
  let iconBusy = $state(false);
  let bgBusy = $state(false);

  const MAX_ICON_BYTES = 4 * 1024 * 1024;

  const themes: { id: Theme; label: string; bg: string; accent: string }[] = [
    { id: "dark", label: "Obsidiana", bg: "#09080d", accent: "#a06bff" },
    { id: "black", label: "Basalto", bg: "#000000", accent: "#e8e8ee" },
    { id: "purple", label: "Amatista", bg: "#110a20", accent: "#c39bff" },
  ];
  const accentPresets = ["#a06bff", "#38bdf8", "#3ddc84", "#f5b544", "#ff8a4c", "#ff5c72", "#ec4899"];

  const themeAccent = $derived(themes.find((t) => t.id === uiDraft.theme)?.accent ?? "#a06bff");
  const preview = $derived(accentColors(uiDraft.accent ?? themeAccent));
  const previewStyle = $derived(
    preview ? `--accent: ${preview.accent}; --accent-hover: ${preview.hover}; --accent-glow: ${preview.glow};` : "",
  );
  const previewBg = $derived(themes.find((t) => t.id === uiDraft.theme)?.bg ?? "#09080d");

  const dirty = $derived(JSON.stringify(uiDraft) !== JSON.stringify(app.settings?.ui ?? DEFAULT_UI));

  async function onIconChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    if (mimeOf(file) !== "image/png") return toasts.push("error", "Usa un PNG cuadrado (por ejemplo, 512×512).");
    if (file.size > MAX_ICON_BYTES) return toasts.push("error", "El icono supera los 4 MB.");
    iconBusy = true;
    try {
      await app.setLauncherIcon(base64Of(await readAsDataUrl(file)));
      toasts.success("Icono actualizado");
    } catch (e) {
      toasts.error(e);
    } finally {
      iconBusy = false;
    }
  }

  async function clearIcon() {
    iconBusy = true;
    try {
      await app.clearLauncherIcon();
    } catch (e) {
      toasts.error(e);
    } finally {
      iconBusy = false;
    }
  }

  async function onBackgroundChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    const mime = mimeOf(file);
    if (!BACKGROUND_TYPES.includes(mime)) return toasts.push("error", "Usa una imagen PNG, JPG, WEBP o GIF.");
    if (file.size > MAX_BACKGROUND_BYTES) return toasts.push("error", "El fondo supera los 15 MB.");
    bgBusy = true;
    try {
      await app.setLauncherBackground(mime, base64Of(await readAsDataUrl(file)));
    } catch (e) {
      toasts.error(e);
    } finally {
      bgBusy = false;
    }
  }

  async function clearBackground() {
    bgBusy = true;
    try {
      await app.clearLauncherBackground();
    } catch (e) {
      toasts.error(e);
    } finally {
      bgBusy = false;
    }
  }

  async function save() {
    if (!app.settings) return;
    error = "";
    const raw = uiDraft.accent?.trim() ?? "";
    const accent = raw ? normalizeHex(raw) : null;
    if (raw && !accent) {
      error = "El color de acento debe tener el formato #RRGGBB.";
      return;
    }
    saving = true;
    try {
      const next = $state.snapshot(app.settings) as Settings;
      next.ui = { ...$state.snapshot(uiDraft), accent };
      await app.saveSettings(next);
      uiDraft.accent = accent;
      toasts.success("Apariencia guardada");
    } catch (e) {
      error = errorText(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="layout">
  <!-- Vista previa en miniatura del propio launcher -->
  <div class="preview" style={previewStyle} style:background-color={previewBg}>
    {#if app.launcherBackground}
      <div
        class="p-bg"
        style:background-image={`url("${app.launcherBackground}")`}
        style:background-size={uiDraft.background_fit === "cover" ? "cover" : uiDraft.background_fit === "contain" ? "contain" : "auto"}
        style:background-repeat={uiDraft.background_fit === "tile" ? "repeat" : "no-repeat"}
        style:filter={uiDraft.background_blur > 0 ? `blur(${uiDraft.background_blur / 3}px)` : undefined}
      ></div>
      <div class="p-shade" style:opacity={uiDraft.background_dim / 100}></div>
    {/if}
    <div class="p-bar">
      <Logo size={14} custom={app.launcherIcon} />
      <span class="p-word">OBSIDIA</span>
      <span class="p-tab active"></span><span class="p-tab"></span><span class="p-tab"></span>
    </div>
    <div class="p-stage obsidia-bg">
      <span class="p-title">Mi instancia</span>
      <span class="p-play">JUGAR</span>
    </div>
  </div>

  <div class="controls">
    <section>
      <h2>Tema</h2>
      <div class="themes">
        {#each themes as t (t.id)}
          <button class="theme" class:active={uiDraft.theme === t.id} onclick={() => (uiDraft.theme = t.id)} aria-pressed={uiDraft.theme === t.id}>
            <span class="swatch" style:background={t.bg}><span style:background={t.accent}></span></span>
            {t.label}
          </button>
        {/each}
      </div>
    </section>

    <section>
      <h2>Color de acento</h2>
      <div class="accents">
        <button class="acc auto" class:active={uiDraft.accent === null} onclick={() => (uiDraft.accent = null)} title="Color del tema">
          Auto
        </button>
        {#each accentPresets as c (c)}
          <button
            class="acc"
            class:active={normalizeHex(uiDraft.accent ?? "") === c}
            style:background={c}
            aria-label={`Usar ${c}`}
            onclick={() => (uiDraft.accent = c)}
          ></button>
        {/each}
        <label class="acc custom" title="Color personalizado">
          <input type="color" value={uiDraft.accent ?? themeAccent} oninput={(e) => (uiDraft.accent = e.currentTarget.value)} />
          <Icon name="palette" size={15} />
        </label>
        <input
          class="input hex mono"
          value={uiDraft.accent ?? ""}
          placeholder={themeAccent}
          aria-label="Color hexadecimal"
          oninput={(e) => {
            const v = e.currentTarget.value.trim();
            uiDraft.accent = v === "" ? null : v;
          }}
        />
      </div>
    </section>

    <section class="toggles">
      <label class="field check"><input type="checkbox" bind:checked={uiDraft.animations} /><span>Animaciones de la interfaz</span></label>
      <label class="field check"><input type="checkbox" bind:checked={uiDraft.transparency} /><span>Paneles translúcidos sobre el fondo</span></label>
    </section>
  </div>
</div>

<div class="assets">
  <section class="asset">
    <div class="asset-head">
      <Logo size={40} custom={app.launcherIcon} />
      <div>
        <strong>Icono del launcher</strong>
        <p class="hint">PNG cuadrado, hasta 4 MB. Se usa en la barra superior y en la ventana.</p>
      </div>
    </div>
    <input bind:this={iconInput} type="file" accept="image/png,.png" hidden onchange={onIconChosen} />
    <div class="asset-actions">
      <button class="btn" onclick={() => iconInput?.click()} disabled={iconBusy}>{iconBusy ? "Guardando…" : "Elegir icono…"}</button>
      {#if app.launcherIcon}<button class="btn ghost" onclick={clearIcon} disabled={iconBusy}>Usar el de Obsidia</button>{/if}
    </div>
  </section>

  <section class="asset">
    <div class="asset-head">
      <span class="bg-thumb" class:obsidia-bg={!app.launcherBackground} style:background-image={app.launcherBackground ? `url("${app.launcherBackground}")` : undefined}></span>
      <div>
        <strong>Fondo del launcher</strong>
        <p class="hint">PNG, JPG, WEBP o GIF animado, hasta 15 MB.</p>
      </div>
    </div>
    <input bind:this={bgInput} type="file" accept={BACKGROUND_ACCEPT} hidden onchange={onBackgroundChosen} />
    <div class="asset-actions">
      <button class="btn" onclick={() => bgInput?.click()} disabled={bgBusy}>{bgBusy ? "Guardando…" : "Elegir fondo…"}</button>
      {#if app.launcherBackground}<button class="btn ghost" onclick={clearBackground} disabled={bgBusy}>Quitar</button>{/if}
    </div>
    {#if app.launcherBackground}
      <div class="bg-controls">
        <label class="field">
          <span>Ajuste</span>
          <select bind:value={uiDraft.background_fit}>
            <option value="cover">Cubrir</option>
            <option value="contain">Contener</option>
            <option value="tile">Mosaico</option>
          </select>
        </label>
        <label class="field"><span>Desenfoque · {uiDraft.background_blur}px</span><input type="range" min="0" max="40" bind:value={uiDraft.background_blur} /></label>
        <label class="field"><span>Oscurecer · {uiDraft.background_dim}%</span><input type="range" min="0" max="90" bind:value={uiDraft.background_dim} /></label>
      </div>
      {#if uiDraft.background_blur > 0}
        <p class="hint">El desenfoque consume GPU; en equipos modestos, mejor 0.</p>
      {/if}
    {/if}
  </section>
</div>

{#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}

<div class="actions">
  <button class="btn ghost" onclick={() => (uiDraft = { ...DEFAULT_UI })} disabled={saving}>Restablecer</button>
  <button class="btn primary" onclick={save} disabled={saving || !dirty}>{saving ? "Guardando…" : dirty ? "Guardar apariencia" : "Guardado"}</button>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(240px, 0.9fr) 1.1fr;
    gap: 22px;
    align-items: start;
  }
  /* Vista previa */
  .preview {
    position: relative;
    aspect-ratio: 16 / 10;
    border-radius: var(--radius);
    overflow: hidden;
    border: 1px solid var(--border-strong);
    display: flex;
    flex-direction: column;
  }
  .p-bg,
  .p-shade {
    position: absolute;
    inset: 0;
    background-position: center;
  }
  .p-shade {
    background: #050409;
  }
  .p-bar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px;
    background: rgb(19 17 27 / 0.86);
    border-bottom: 1px solid rgb(255 255 255 / 0.07);
  }
  .p-word {
    font-family: var(--font-display);
    font-size: 7px;
    letter-spacing: 0.2em;
    margin-right: 6px;
  }
  .p-tab {
    width: 22px;
    height: 4px;
    border-radius: 2px;
    background: rgb(255 255 255 / 0.15);
  }
  .p-tab.active {
    background: var(--accent);
  }
  .p-stage {
    position: relative;
    flex: 1;
    margin: 8px;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: flex-start;
    gap: 6px;
    padding: 12px;
  }
  .p-title {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 18px;
  }
  .p-play {
    padding: 5px 14px;
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 9px;
    letter-spacing: 0.18em;
    clip-path: polygon(5px 0, 100% 0, 100% calc(100% - 5px), calc(100% - 5px) 100%, 0 100%, 0 5px);
  }

  .controls {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .themes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .theme {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 14px 0 8px;
    border-radius: 10px;
    border: 1px solid var(--border-strong);
    background: rgb(255 255 255 / 0.03);
    color: var(--text-2);
    cursor: pointer;
    font-weight: 600;
    font-size: 13px;
  }
  .theme.active {
    border-color: var(--accent);
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  .swatch {
    position: relative;
    width: 26px;
    height: 26px;
    border-radius: 6px;
    border: 1px solid rgb(255 255 255 / 0.15);
  }
  .swatch span {
    position: absolute;
    right: 3px;
    bottom: 3px;
    width: 9px;
    height: 9px;
    border-radius: 2px;
  }
  .accents {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .acc {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    border: 2px solid transparent;
    cursor: pointer;
    outline-offset: 2px;
  }
  .acc.active {
    border-color: #fff;
  }
  .acc.auto {
    width: auto;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    background: rgb(255 255 255 / 0.04);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
  }
  .acc.auto.active {
    border-color: var(--accent);
    color: var(--text);
  }
  .acc.custom {
    position: relative;
    display: grid;
    place-items: center;
    border: 1px dashed var(--border-strong);
    color: var(--muted);
  }
  .acc.custom input {
    position: absolute;
    inset: 0;
    opacity: 0;
    width: 100%;
    height: 100%;
  }
  .hex {
    width: 100px;
    height: 30px;
  }
  .toggles {
    display: flex;
    flex-direction: column;
  }
  .toggles .field {
    margin-bottom: 8px;
  }

  .assets {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    margin-top: 24px;
  }
  .asset {
    padding: 16px 18px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
  }
  .asset-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 12px;
  }
  .bg-thumb {
    width: 64px;
    height: 40px;
    flex: none;
    border-radius: 8px;
    background-size: cover;
    background-position: center;
    border: 1px solid var(--border-strong);
  }
  .asset-actions {
    display: flex;
    gap: 8px;
  }
  .bg-controls {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 12px;
    margin-top: 14px;
  }
  .alert {
    margin-top: 16px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 20px;
  }
  @media (max-width: 1100px) {
    .layout,
    .assets {
      grid-template-columns: 1fr;
    }
    .preview {
      max-width: 420px;
    }
  }
</style>
