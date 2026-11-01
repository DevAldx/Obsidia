<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { nav } from "../lib/nav.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { formatMemory, loaderLabel } from "../lib/format";
  import type { ExportedCode, Instance, ShareProgressEvent, UploadedPack } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  let { instance, onclose }: { instance: Instance; onclose: () => void } = $props();

  // Siempre hay servidor: el propio de Ajustes o el oficial de Obsidia.
  const hasServer = true;
  const isMicrosoft = $derived(app.account?.kind === "microsoft");
  const hasBackground = $derived(!!app.backgrounds[instance.id]);

  /** `server`: código corto subiendo al servidor. `long`: código OBS1 sin servidor. */
  let mode = $state<"server" | "long">("server");
  $effect(() => {
    if (!hasServer) mode = "long";
  });

  let description = $state("");
  let includeConfig = $state(true);
  let backgroundUrl = $state("");
  let busy = $state(false);
  let error = $state("");
  let uploaded = $state<UploadedPack | null>(null);
  let longCode = $state<ExportedCode | null>(null);
  let progress = $state<ShareProgressEvent | null>(null);
  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;
  let templatePath = $state<string | null>(null);
  let unlisten: UnlistenFn | undefined;

  onMount(async () => {
    void app.ensureBackground(instance.id);
    unlisten = await listen<ShareProgressEvent>("share-progress", (e) => {
      if (e.payload.instance_id === instance.id) progress = e.payload;
    });
  });
  onDestroy(() => unlisten?.());

  const pct = $derived(
    progress && progress.total > 0 ? Math.min(100, Math.round((progress.done / progress.total) * 100)) : null,
  );
  const resultCode = $derived(mode === "server" ? uploaded?.code : longCode?.code);

  async function generate() {
    error = "";
    busy = true;
    progress = null;
    try {
      if (mode === "server") {
        uploaded = await api.uploadSharePack(instance.id, description.trim() || null, includeConfig);
      } else {
        longCode = await api.exportShareCode(instance.id, description.trim() || null, backgroundUrl.trim() || null);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  function reset() {
    uploaded = null;
    longCode = null;
    error = "";
  }

  async function copy() {
    if (!resultCode) return;
    try {
      await navigator.clipboard.writeText(resultCode);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copied = false), 2500);
    } catch {
      // Se puede seleccionar el texto a mano.
    }
  }

  async function exportTemplate() {
    try {
      templatePath = await api.exportManifestTemplate(instance.id);
    } catch (e) {
      toasts.error(e, "No se pudo exportar");
    }
  }

  const mb = (bytes: number) => `${(bytes / 1024 / 1024).toFixed(1)} MB`;
</script>

<Modal title="Compartir instancia" subtitle="Genera un código que lleva dentro toda la configuración." width={520} {busy} {onclose}>
  <div class="summary">
    <strong class="display">{instance.name}</strong>
    <div class="tags">
      <span class="tag">Minecraft {instance.minecraft_version}</span>
      <span class="tag">{loaderLabel(instance)}</span>
      <span class="tag">RAM {formatMemory(instance.memory.min_mb)}–{formatMemory(instance.memory.max_mb)}</span>
      <span class="tag">{instance.mod_count} mods</span>
    </div>
  </div>

  {#if resultCode}
    <!-- ───── Resultado ───── -->
    {#if mode === "server" && uploaded}
      <div class="short-code">
        <span class="mono">{uploaded.code}</span>
      </div>
      <p class="hint">
        {uploaded.from_modrinth} desde Modrinth · {uploaded.hosted} en tu servidor
        {#if uploaded.uploaded_bytes > 0}({mb(uploaded.uploaded_bytes)} subidos){/if}
        {#if uploaded.has_background} · con imagen{/if}.
        Quien lo reciba solo tiene que pegarlo en <strong>Importar por código</strong>.
      </p>
    {:else if longCode}
      <label class="field">
        <span>Tu código</span>
        <textarea class="input mono code" readonly rows="5" value={longCode.code} onfocus={(e) => e.currentTarget.select()}></textarea>
      </label>
      <p class="hint">
        {longCode.included} {longCode.included === 1 ? "archivo incluido" : "archivos incluidos"} · {longCode.code.length} caracteres.
        Funciona sin servidor: basta con pegarlo en <strong>Importar por código</strong>.
      </p>
      {#if longCode.missing.length}
        <div class="alert warn">
          <Icon name="info" size={16} />
          <span>
            {longCode.missing.length === 1 ? "Este archivo no está" : "Estos archivos no están"} en Modrinth y
            <strong>no van en el código</strong>:
            <span class="mono missing">{longCode.missing.join(", ")}</span>
          </span>
        </div>
      {/if}
    {/if}
  {:else}
    <!-- ───── Formulario ───── -->
    {#if hasServer}
      <div class="modes" role="tablist">
        <button role="tab" class:active={mode === "server"} aria-selected={mode === "server"} onclick={() => (mode = "server")} disabled={busy}>
          <strong>Código corto</strong><span class="hint">Todo incluido · usa tu servidor</span>
        </button>
        <button role="tab" class:active={mode === "long"} aria-selected={mode === "long"} onclick={() => (mode = "long")} disabled={busy}>
          <strong>Código largo</strong><span class="hint">Sin servidor · solo Modrinth</span>
        </button>
      </div>
    {/if}

    <label class="field">
      <span>Descripción <span class="hint">(opcional)</span></span>
      <input class="input" bind:value={description} maxlength="300" placeholder="Survival con amigos, mods de rendimiento…" disabled={busy} />
    </label>

    {#if mode === "server"}
      <label class="check">
        <input type="checkbox" bind:checked={includeConfig} disabled={busy} />
        <span>Incluir la configuración de los mods (carpeta <span class="mono">config</span>)</span>
      </label>
      <p class="hint">
        Los mods de Modrinth se descargan de Modrinth; el resto
        {hasBackground ? "y la imagen de la instancia se suben" : "se sube"} a tu servidor.
        {#if !hasBackground}La instancia no tiene imagen; puedes ponerle una en sus ajustes.{/if}
      </p>
      {#if !isMicrosoft}
        <div class="alert warn">
          <Icon name="info" size={16} />
          <span>Para subir necesitas iniciar sesión con una cuenta de Microsoft: el servidor la verifica con Mojang para evitar abusos.</span>
        </div>
      {/if}
      {#if busy}
        <div class="progress"><div class:indeterminate={pct === null} style:width={pct === null ? undefined : `${pct}%`}></div></div>
        <p class="hint">{progress?.phase ?? "Preparando…"}{pct !== null ? ` · ${pct}%` : ""}</p>
      {/if}
    {:else}
      <label class="field">
        <span>URL de la imagen <span class="hint">(opcional)</span></span>
        <input class="input mono" bind:value={backgroundUrl} placeholder="https://i.imgur.com/….png" disabled={busy} />
      </label>
      <p class="hint">
        Sin servidor no se pueden incluir archivos: la imagen va como enlace https y solo viajan los mods,
        resource packs y shaders que estén en Modrinth.
        {#if !hasServer}
          <button class="inline-link" onclick={() => { onclose(); nav.openSettings("sharing"); }}>Configura un servidor</button>
          para códigos cortos con todo incluido.
        {/if}
      </p>
    {/if}
    {#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}
  {/if}

  <details class="advanced">
    <summary>Avanzado: plantilla de manifiesto para alojarlo a mano</summary>
    <p class="hint">Exporta una plantilla JSON y rellena tú las URLs, el SHA-1 y el tamaño de cada archivo.</p>
    <button class="btn sm" onclick={exportTemplate}>Exportar plantilla</button>
    {#if templatePath}<p class="hint">Guardada en <span class="mono">{templatePath}</span></p>{/if}
  </details>

  {#snippet footer()}
    {#if resultCode}
      <button class="btn ghost" onclick={reset}>Volver</button>
      <button class="btn primary" onclick={copy}>
        <Icon name={copied ? "check" : "share"} size={15} /> {copied ? "¡Copiado!" : "Copiar código"}
      </button>
    {:else}
      <button class="btn ghost" onclick={onclose} disabled={busy}>Cancelar</button>
      <button class="btn primary" onclick={generate} disabled={busy || (mode === "server" && !isMicrosoft)}>
        {#if busy}{mode === "server" ? "Subiendo…" : "Buscando mods en Modrinth…"}{:else}Generar código{/if}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .summary {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
    padding: 14px;
    border-radius: var(--radius);
    background: rgb(0 0 0 / 0.25);
    border: 1px solid var(--border);
  }
  .summary strong {
    font-size: 17px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .modes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 14px;
  }
  .modes button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 10px 12px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: rgb(0 0 0 / 0.2);
    color: inherit;
    cursor: pointer;
    text-align: left;
  }
  .modes button.active {
    border-color: var(--accent);
    background: rgb(0 0 0 / 0.35);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    margin-bottom: 8px;
    cursor: pointer;
  }
  .short-code {
    display: grid;
    place-items: center;
    padding: 18px;
    border-radius: var(--radius);
    border: 1px dashed var(--accent);
    background: rgb(0 0 0 / 0.25);
    margin-bottom: 8px;
  }
  .short-code span {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: 0.12em;
    user-select: all;
  }
  .code {
    width: 100%;
    resize: none;
    word-break: break-all;
    font-size: 12px;
  }
  .missing {
    word-break: break-word;
  }
  .alert {
    margin: 10px 0 0;
  }
  .progress {
    margin-top: 12px;
  }
  .inline-link {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--accent-hover);
    font-weight: 600;
    text-decoration: underline;
  }
  .advanced {
    margin-top: 14px;
  }
  .advanced summary {
    cursor: pointer;
    font-size: 13px;
    color: var(--muted);
  }
  .advanced .btn {
    margin-top: 6px;
  }
</style>
