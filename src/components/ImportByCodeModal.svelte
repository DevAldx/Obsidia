<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { nav } from "../lib/nav.svelte";
  import type { InstanceManifest } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  let { onclose, initialCode = "" }: { onclose: () => void; initialCode?: string } = $props();

  let code = $state(untrack(() => initialCode));
  let manifest = $state<InstanceManifest | null>(null);
  let looking = $state(false);
  let lookupError = $state("");

  const noService = $derived(!app.settings?.sharing.manifest_base_url);

  onMount(() => {
    // Un resultado de una importación anterior no debe aparecer al reabrir.
    app.importedId = null;
    app.importError = null;
    if (code.trim()) void lookup();
  });

  async function lookup() {
    lookupError = "";
    manifest = null;
    if (!code.trim()) return;
    looking = true;
    try {
      manifest = await api.fetchShareManifest(code.trim());
    } catch (e) {
      lookupError = e instanceof Error ? e.message : String(e);
    } finally {
      looking = false;
    }
  }

  async function install() {
    if (!manifest) return;
    await app.importFromCode(manifest);
  }

  const progress = $derived(manifest ? app.progress[manifest.code] : undefined);
  const pct = $derived(
    progress && progress.total > 0 ? Math.min(100, Math.round((progress.done / progress.total) * 100)) : null,
  );
  const fileCount = $derived(
    manifest
      ? manifest.mods.length + manifest.resourcepacks.length + manifest.shaderpacks.length + manifest.config_files.length
      : 0,
  );
  const javaOk = $derived(!manifest?.java || app.java.some((j) => j.major >= manifest!.java!.min_major));
</script>

<Modal title="Importar por código" subtitle="Instala una instancia compartida a partir de su manifiesto." width={460} busy={app.importBusy} {onclose}>
  {#if app.importedId}
    <div class="done">
      <span class="done-icon facet"><Icon name="check" size={22} strokeWidth={2.4} /></span>
      <strong class="display">¡Instancia instalada!</strong>
      <p class="muted">Ya la tienes disponible en Instancias.</p>
    </div>
  {:else}
    {#if noService}
      <div class="alert warn">
        <Icon name="info" size={16} />
        <span>
          No hay un servicio de manifiestos configurado, así que no hay dónde buscar el código.
          <button class="inline-link" onclick={() => { onclose(); nav.openSettings("sharing"); }}>Configurarlo en Ajustes</button>
        </span>
      </div>
    {/if}
    <div class="code-row">
      <input
        class="input mono"
        bind:value={code}
        placeholder="OBS-K7P2X9"
        aria-label="Código"
        onkeydown={(e) => e.key === "Enter" && lookup()}
      />
      <button class="btn" onclick={lookup} disabled={looking || !code.trim()}>
        {looking ? "Buscando…" : "Buscar"}
      </button>
    </div>
    {#if lookupError}<div class="alert"><Icon name="alert" size={16} /><span>{lookupError}</span></div>{/if}

    {#if manifest}
      <div class="preview">
        <strong class="display">{manifest.name}</strong>
        {#if manifest.description}<p class="muted">{manifest.description}</p>{/if}
        <div class="tags">
          <span class="tag">Minecraft {manifest.minecraft_version}</span>
          <span class="tag">{manifest.loader === "fabric" ? `Fabric ${manifest.loader_version ?? ""}`.trim() : "Vanilla"}</span>
          <span class="tag">{fileCount} archivos</span>
        </div>
        {#if manifest.java && !javaOk}
          <p class="hint">Requiere Java {manifest.java.min_major}+; se instalará automáticamente al importar.</p>
        {/if}
        {#if manifest.background_url}<p class="hint">También se descargará el fondo de la instancia.</p>{/if}
      </div>
      {#if app.importBusy}
        <div class="progress"><div class:indeterminate={pct === null} style:width={pct === null ? undefined : `${pct}%`}></div></div>
        <p class="hint">{progress?.phase ?? "Preparando…"}{pct !== null ? ` · ${pct}%` : ""}</p>
      {/if}
      {#if app.importError}<div class="alert"><Icon name="alert" size={16} /><span>{app.importError}</span></div>{/if}
    {/if}
  {/if}

  {#snippet footer()}
    {#if app.importedId}
      <button class="btn primary" onclick={onclose}>Listo</button>
    {:else}
      <button class="btn ghost" onclick={onclose} disabled={app.importBusy}>Cancelar</button>
      <button class="btn primary" onclick={install} disabled={!manifest || app.importBusy}>
        {app.importBusy ? "Instalando…" : "Instalar"}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .code-row {
    display: flex;
    gap: 8px;
  }
  .code-row input {
    flex: 1;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .alert {
    margin: 10px 0;
  }
  .alert:first-child {
    margin-top: 0;
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
  .preview {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 14px;
    padding: 14px;
    border-radius: var(--radius);
    background: rgb(0 0 0 / 0.25);
    border: 1px solid var(--border);
  }
  .preview strong {
    font-size: 17px;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .progress {
    margin-top: 14px;
  }
  .done {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 18px 0 6px;
    text-align: center;
  }
  .done-icon {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    background: var(--accent);
    color: var(--accent-ink);
    margin-bottom: 6px;
  }
  .done strong {
    font-size: 18px;
  }
</style>
