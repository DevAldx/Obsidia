<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { readAsBase64 } from "../lib/files";
  import { errorText, formatBytes } from "../lib/format";
  import type { ModpackPreview, MrVersion, SearchHit } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  /** Un modpack de Modrinth (`hit`) o un `.mrpack` elegido desde el PC (`file`). */
  let { hit = null, file = null, onclose }: { hit?: SearchHit | null; file?: File | null; onclose: () => void } = $props();

  let versions = $state<MrVersion[]>([]);
  let versionId = $state("");
  let preview = $state<ModpackPreview | null>(null);
  let loading = $state(true);
  let error = $state("");
  let seq = 0;

  onMount(async () => {
    app.importedId = null;
    app.importError = null;
    try {
      if (file) {
        preview = await api.modpackPreviewFile(await readAsBase64(file));
      } else if (hit) {
        versions = await api.modpackVersions(hit.project_id);
        if (versions.length === 0) throw new Error("Este modpack no tiene versiones para Fabric.");
        const release = versions.find((v) => v.version_type === "release") ?? versions[0];
        versionId = release?.id ?? "";
        await loadPreview();
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  });

  async function loadPreview() {
    if (!hit || !versionId) return;
    const mine = ++seq;
    preview = null;
    error = "";
    loading = true;
    try {
      const p = await api.modpackPreviewVersion(versionId, hit.icon_url);
      if (mine === seq) preview = p;
    } catch (e) {
      if (mine === seq) error = errorText(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  const progress = $derived(preview ? app.progress[preview.token] : undefined);
  const pct = $derived(
    progress && progress.total > 0 ? Math.min(100, Math.round((progress.done / progress.total) * 100)) : null,
  );
  const title = $derived(hit?.title ?? preview?.name ?? "Modpack");
  const iconUrl = $derived(hit?.icon_url ?? null);
</script>

<Modal title="Instalar modpack" subtitle="Se crea una instancia nueva con todos sus mods y su configuración." width={500} busy={app.importBusy} {onclose}>
  {#if app.importedId}
    <div class="done">
      <span class="done-icon facet"><Icon name="check" size={22} strokeWidth={2.4} /></span>
      <strong class="display">¡Modpack instalado!</strong>
      <p class="muted">Ya lo tienes en Instancias, listo para jugar.</p>
    </div>
  {:else}
    <div class="head">
      {#if iconUrl}
        <img class="icon" src={iconUrl} alt="" />
      {:else}
        <span class="icon placeholder"><Icon name="layers" size={22} /></span>
      {/if}
      <div>
        <strong class="display">{title}</strong>
        {#if hit}<p class="hint">de {hit.author}</p>{:else if file}<p class="hint mono">{file.name}</p>{/if}
      </div>
    </div>

    {#if versions.length > 0}
      <label class="field">
        <span>Versión</span>
        <select class="input" bind:value={versionId} onchange={loadPreview} disabled={app.importBusy}>
          {#each versions as v (v.id)}
            <option value={v.id}>{v.version_number} · MC {v.game_versions.join(", ")}{v.version_type !== "release" ? ` · ${v.version_type}` : ""}</option>
          {/each}
        </select>
      </label>
    {/if}

    {#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}

    {#if loading && !preview}
      <div class="skeleton" style="height: 96px"></div>
    {:else if preview}
      <div class="preview">
        {#if preview.summary}<p class="muted">{preview.summary}</p>{/if}
        <div class="tags">
          <span class="tag">Minecraft {preview.minecraft_version}</span>
          <span class="tag">{preview.loader === "fabric" ? `Fabric ${preview.loader_version ?? ""}`.trim() : "Vanilla"}</span>
          <span class="tag">{preview.mod_count} mods</span>
          <span class="tag">{formatBytes(preview.download_bytes)}</span>
          {#if preview.override_count > 0}<span class="tag">{preview.override_count} archivos de configuración</span>{/if}
        </div>
        <p class="hint">También se descargan Minecraft y el Java que necesite, así que «Jugar» arranca sin más esperas.</p>
        {#if !hit}
          <div class="alert warn">
            <Icon name="info" size={16} />
            <span>Este .mrpack viene de tu PC: instálalo solo si confías en quien lo hizo.</span>
          </div>
        {:else if preview.external_files > 0}
          <div class="alert info">
            <Icon name="info" size={16} />
            <span>{preview.external_files} {preview.external_files === 1 ? "archivo se descarga" : "archivos se descargan"} de GitHub o GitLab, como permite Modrinth.</span>
          </div>
        {/if}
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
      <button class="btn primary" onclick={() => preview && app.installModpack(preview.token)} disabled={!preview || app.importBusy || loading}>
        {app.importBusy ? "Instalando…" : "Instalar"}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }
  .head strong {
    font-size: 17px;
  }
  .head p {
    margin: 2px 0 0;
  }
  .icon {
    width: 52px;
    height: 52px;
    flex: none;
    border-radius: 12px;
    object-fit: cover;
    background: rgb(255 255 255 / 0.06);
  }
  .placeholder {
    display: grid;
    place-items: center;
    color: var(--accent-hover);
  }
  .field {
    margin-bottom: 12px;
  }
  .alert {
    margin: 10px 0 0;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px;
    border-radius: var(--radius);
    background: rgb(0 0 0 / 0.25);
    border: 1px solid var(--border);
  }
  .preview p {
    margin: 0;
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
