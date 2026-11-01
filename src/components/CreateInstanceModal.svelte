<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, splitArgs } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { errorText } from "../lib/format";
  import type { Loader, VersionSummary } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  let {
    preset = null,
    onclose,
  }: {
    /** Valores sugeridos (por ejemplo, desde Descubrir). */
    preset?: { version?: string; loader?: Loader; snapshots?: boolean } | null;
    /** Recibe el ID creado, o nada si se canceló. */
    onclose: (createdId?: string) => void;
  } = $props();

  const init = untrack(() => preset);
  const d = app.settings!.minecraft;
  let name = $state("");
  let version = $state(init?.version ?? "");
  let loader = $state<Loader>(init?.loader ?? "fabric");
  let minRam = $state(d.min_ram_mb);
  let maxRam = $state(d.max_ram_mb);
  let jvm = $state(d.jvm_args.join(" "));
  let versions = $state<VersionSummary[]>([]);
  let showSnapshots = $state(init?.snapshots ?? false);
  let loadingVersions = $state(true);
  let versionsError = $state("");
  let busy = $state(false);
  let error = $state("");

  const shown = $derived(versions.filter((v) => showSnapshots || v.kind === "release" || v.id === version));
  const latestRelease = $derived(versions.find((v) => v.kind === "release")?.id ?? "");
  const placeholder = $derived(version ? `${loader === "fabric" ? "Fabric" : "Vanilla"} ${version}` : "Mi instancia");

  onMount(async () => {
    try {
      versions = await api.listVersions();
      if (!version || !versions.some((v) => v.id === version)) version = latestRelease;
    } catch (e) {
      versionsError = errorText(e);
    } finally {
      loadingVersions = false;
    }
  });

  async function submit() {
    if (!version) return;
    busy = true;
    error = "";
    try {
      const created = await api.createInstance({
        name: name.trim() || placeholder,
        minecraft_version: version,
        loader,
        memory: { min_mb: Number(minRam), max_mb: Number(maxRam) },
        jvm_args: splitArgs(jvm),
      });
      if (!app.settings?.selected_instance) await app.select(created.id);
      await app.refreshInstances();
      toasts.success(`Instancia «${created.name}» creada`);
      onclose(created.id);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  const loaders: { id: Loader; title: string; text: string }[] = [
    { id: "fabric", title: "Fabric", text: "Mods ligeros y de rendimiento" },
    { id: "vanilla", title: "Vanilla", text: "Minecraft tal cual, sin mods" },
  ];
</script>

<Modal title="Nueva instancia" subtitle="Obsidia descarga Minecraft, el loader y Java al jugar por primera vez." width={540} {busy} onclose={() => onclose()}>
  <p class="step"><span>1</span> Loader</p>
  <div class="loaders">
    {#each loaders as l (l.id)}
      <button class="loader facet" class:active={loader === l.id} onclick={() => (loader = l.id)} aria-pressed={loader === l.id}>
        <span class="l-icon"><Icon name={l.id === "fabric" ? "puzzle" : "layers"} size={20} /></span>
        <strong>{l.title}</strong>
        <span class="hint">{l.text}</span>
      </button>
    {/each}
  </div>

  <p class="step"><span>2</span> Versión de Minecraft</p>
  {#if loadingVersions}
    <div class="skeleton" style="height: 38px"></div>
  {:else if versionsError}
    <div class="alert"><Icon name="alert" size={16} /><span>No se pudo cargar la lista de versiones: {versionsError}</span></div>
  {:else}
    <div class="version-row">
      <select class="input" bind:value={version} aria-label="Versión de Minecraft">
        {#each shown as v (v.id)}
          <option value={v.id}>{v.id}{v.id === latestRelease ? " · última" : v.kind !== "release" ? ` · ${v.kind}` : ""}</option>
        {/each}
      </select>
      <label class="field check inline">
        <input type="checkbox" bind:checked={showSnapshots} /><span>Snapshots y antiguas</span>
      </label>
    </div>
  {/if}

  <p class="step"><span>3</span> Nombre</p>
  <input class="input full" bind:value={name} maxlength="48" placeholder={placeholder} />

  <details class="advanced">
    <summary>Memoria y argumentos de Java</summary>
    <div class="row">
      <label class="field"><span>RAM mínima (MB)</span><input type="number" step="256" min="256" bind:value={minRam} /></label>
      <label class="field"><span>RAM máxima (MB)</span><input type="number" step="256" min="512" bind:value={maxRam} /></label>
    </div>
    <label class="field">
      <span>Argumentos JVM</span>
      <textarea rows="2" bind:value={jvm} placeholder="-XX:+UseG1GC"></textarea>
    </label>
  </details>

  {#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}

  {#snippet footer()}
    <button class="btn ghost" onclick={() => onclose()} disabled={busy}>Cancelar</button>
    <button class="btn primary" onclick={submit} disabled={busy || !version}>
      {busy ? "Creando…" : "Crear instancia"}
    </button>
  {/snippet}
</Modal>

<style>
  .step {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 14px 0 8px;
    font-size: 12px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-2);
  }
  .step:first-child {
    margin-top: 6px;
  }
  .step span {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    background: rgb(255 255 255 / 0.08);
    font-size: 11px;
    letter-spacing: 0;
  }
  .loaders {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .loader {
    --facet: 12px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 14px;
    border: none;
    background: rgb(255 255 255 / 0.04);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    color: var(--text);
    cursor: pointer;
    text-align: left;
    transition: background var(--ease), box-shadow var(--ease);
  }
  .loader:hover {
    background: rgb(255 255 255 / 0.07);
  }
  .loader.active {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--accent);
  }
  .l-icon {
    color: var(--accent-hover);
    margin-bottom: 6px;
  }
  .version-row {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .version-row select {
    flex: 1;
  }
  .inline {
    margin: 0;
    white-space: nowrap;
  }
  .inline span {
    font-size: 12px;
    color: var(--text-2);
  }
  .full {
    width: 100%;
  }
  .advanced {
    margin-top: 16px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
  .advanced summary {
    cursor: pointer;
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 10px;
  }
  .alert {
    margin-top: 12px;
  }
</style>
