<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import type { InstanceManifest } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { formatMemory } from "../lib/format";

  let { onclose, initialCode = "" }: { onclose: () => void; initialCode?: string } = $props();

  let code = $state(untrack(() => initialCode));
  let manifest = $state<InstanceManifest | null>(null);
  let looking = $state(false);
  let lookupError = $state("");

  // Los códigos `OBS1-…` llevan todo dentro; los cortos `OBS-XXXXXX` se buscan
  // en el servidor para compartir (el oficial si no hay uno propio).
  const isPackCode = $derived(code.trim().startsWith("OBS1-"));

  onMount(() => {
    // Un resultado de una importación anterior no debe aparecer al reabrir.
    app.importedId = null;
    app.importError = null;
    // Viene de Descubrir con un código ya escrito: se busca aunque esté incompleto,
    // para que el error explique qué falla.
    if (code.trim()) void lookup();
  });

  /** Los cortos se pasan a mayúsculas (el ID es `OBS-XXXXXX`); los largos son base64 y no se tocan. */
  function normalized(raw: string): string {
    const c = raw.trim();
    return /^(obs|nbl)-[a-z0-9]{6}$/i.test(c) ? c.toUpperCase() : c;
  }

  /** ¿El texto ya parece un código entero? Así se busca solo al pegarlo o terminar de escribirlo. */
  function looksComplete(c: string): boolean {
    return /^(OBS|NBL)-[A-Z0-9]{6}$/.test(c) || (c.startsWith("OBS1-") && c.length > 24);
  }

  let lastLookup = "";

  async function lookup() {
    const c = normalized(code);
    if (!c) return;
    lastLookup = c;
    lookupError = "";
    manifest = null;
    looking = true;
    try {
      const found = await api.fetchShareManifest(c);
      // Si mientras tanto cambió el código, esta respuesta ya no vale.
      if (normalized(code) === c) manifest = found;
    } catch (e) {
      if (normalized(code) === c) lookupError = e instanceof Error ? e.message : String(e);
    } finally {
      looking = false;
    }
  }

  // Busca solo en cuanto el código está completo (pegado o tecleado), con una
  // pequeña espera para no buscar a cada tecla.
  $effect(() => {
    const c = normalized(code);
    if (!looksComplete(c) || c === lastLookup || app.importBusy) return;
    const t = setTimeout(() => {
      if (normalized(code) !== lastLookup) void lookup();
    }, 250);
    return () => clearTimeout(t);
  });

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
  // Los mods que no vienen de Modrinth los subió quien compartió el pack y
  // nadie los ha revisado: un .jar ejecuta código en este PC.
  const unverifiedMods = $derived(
    manifest ? manifest.mods.filter((m) => !m.url.startsWith("https://cdn.modrinth.com/")).length : 0,
  );
  const javaOk = $derived(!manifest?.java || app.java.some((j) => j.major >= manifest!.java!.min_major));
</script>

<Modal title="Importar por código" subtitle="Pega el código que te pasaron y se instala todo: mods, RAM, imagen…" width={480} busy={app.importBusy} {onclose}>
  {#if app.importedId}
    <div class="done">
      <span class="done-icon facet"><Icon name="check" size={22} strokeWidth={2.4} /></span>
      <strong class="display">¡Instancia instalada!</strong>
      <p class="muted">Ya la tienes disponible en Instancias.</p>
    </div>
  {:else}
    <div class="code-row">
      <textarea
        class="input mono"
        class:pack={isPackCode}
        rows={isPackCode ? 4 : 1}
        bind:value={code}
        disabled={app.importBusy}
        placeholder="Pega aquí el código (OBS-K7P2X9 u OBS1-…)"
        aria-label="Código"
        onkeydown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            lookup();
          }
        }}
      ></textarea>
      <button class="btn" onclick={lookup} disabled={looking || !code.trim() || app.importBusy}>
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
          {#if manifest.memory}<span class="tag">RAM {formatMemory(manifest.memory.min_mb)}–{formatMemory(manifest.memory.max_mb)}</span>{/if}
          <span class="tag">{fileCount} archivos</span>
        </div>
        {#if manifest.java && !javaOk}
          <p class="hint">Requiere Java {manifest.java.min_major}+; se instalará automáticamente al importar.</p>
        {/if}
        {#if manifest.background_url}<p class="hint">También se descargará el fondo de la instancia.</p>{/if}
        {#if unverifiedMods > 0}
          <div class="alert warn">
            <Icon name="info" size={16} />
            <span>
              {unverifiedMods} {unverifiedMods === 1 ? "mod no viene" : "mods no vienen"} de Modrinth: los subió quien
              compartió el pack. Instálalo solo si confías en esa persona.
            </span>
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
  .code-row {
    align-items: flex-start;
  }
  .code-row textarea {
    flex: 1;
    resize: none;
    letter-spacing: 0.08em;
  }
  .code-row textarea.pack {
    letter-spacing: normal;
    word-break: break-all;
    font-size: 12px;
  }
  .alert {
    margin: 10px 0;
  }
  .alert:first-child {
    margin-top: 0;
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
