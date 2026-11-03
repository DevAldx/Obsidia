<script lang="ts">
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { readAsBase64 } from "../lib/files";
  import { errorText } from "../lib/format";
  import { toasts } from "../lib/toast.svelte";
  import type { Instance, ModEntry } from "../lib/types";
  import Icon from "./Icon.svelte";
  import ModBrowserModal from "./ModBrowserModal.svelte";
  import PublishModModal from "./PublishModModal.svelte";

  let { instance }: { instance: Instance } = $props();

  let mods = $state<ModEntry[]>([]);
  let loading = $state(true);
  let error = $state("");
  let busy = $state<string | null>(null); // file_name en curso
  let query = $state("");
  let fileInput = $state<HTMLInputElement | null>(null);
  let confirmDelete = $state<string | null>(null);
  let browsing = $state(false);
  let publishing = $state<ModEntry | null>(null);

  async function refresh() {
    error = "";
    try {
      mods = await api.listMods(instance.id);
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  // Recarga al cambiar de instancia.
  $effect(() => {
    instance.id;
    loading = true;
    mods = [];
    void refresh();
  });

  const displayName = (f: string) => (f.endsWith(".disabled") ? f.slice(0, -".disabled".length) : f);
  const formatSize = (b: number) =>
    b >= 1024 * 1024 ? `${(b / (1024 * 1024)).toFixed(1)} MB` : `${Math.max(1, Math.round(b / 1024))} KB`;

  const filtered = $derived(
    query.trim() ? mods.filter((m) => m.file_name.toLowerCase().includes(query.trim().toLowerCase())) : mods,
  );
  const enabledCount = $derived(mods.filter((m) => m.enabled).length);

  async function toggle(mod: ModEntry) {
    busy = mod.file_name;
    try {
      await api.setModEnabled(instance.id, mod.file_name, !mod.enabled);
      await refresh();
    } catch (e) {
      toasts.error(e);
    } finally {
      busy = null;
    }
  }

  async function remove(mod: ModEntry) {
    busy = mod.file_name;
    try {
      await api.deleteMod(instance.id, mod.file_name);
      confirmDelete = null;
      await refresh();
      void app.refreshInstances(); // actualiza el contador de mods
    } catch (e) {
      toasts.error(e);
    } finally {
      busy = null;
    }
  }

  async function onFilesChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const chosen = Array.from(input.files ?? []);
    input.value = "";
    if (chosen.length === 0) return;
    busy = "__import__";
    try {
      const files = await Promise.all(chosen.map(async (f) => ({ name: f.name, data_base64: await readAsBase64(f) })));
      const imported = await api.importMods(instance.id, files);
      await refresh();
      void app.refreshInstances();
      toasts.success(imported.length === 1 ? "Mod importado" : `${imported.length} mods importados`);
    } catch (e) {
      toasts.error(e, "No se pudieron importar los mods");
    } finally {
      busy = null;
    }
  }

  async function openFolder(sub: "mods" | "config") {
    try {
      await api.openInstanceFolder(instance.id, sub);
    } catch (e) {
      toasts.error(e, "No se pudo abrir la carpeta");
    }
  }
</script>

<div class="toolbar">
  <div class="search">
    <Icon name="search" size={15} />
    <input class="input" bind:value={query} placeholder="Buscar mods…" aria-label="Buscar mods" />
  </div>
  <span class="count">{enabledCount}/{mods.length} activos</span>
  <span class="spacer"></span>
  <input bind:this={fileInput} type="file" accept=".jar" multiple hidden onchange={onFilesChosen} />
  <button class="btn" onclick={() => openFolder("mods")}><Icon name="folder" size={15} /> mods</button>
  <button class="btn" onclick={() => openFolder("config")}><Icon name="folder" size={15} /> config</button>
  <button class="btn" onclick={() => fileInput?.click()} disabled={busy === "__import__"}>
    <Icon name="upload" size={15} /> {busy === "__import__" ? "Importando…" : "Importar .jar"}
  </button>
  <button class="btn primary" onclick={() => (browsing = true)}>
    <Icon name="plus" size={15} /> Añadir mods
  </button>
</div>

{#if instance.loader === "vanilla"}
  <div class="alert warn"><Icon name="info" size={16} /><span>Esta instancia es Vanilla: los mods no se cargarán sin un loader como Fabric.</span></div>
{/if}
{#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}

{#if loading}
  <div class="list">
    {#each [1, 2, 3, 4] as n (n)}<div class="skeleton" style="height: 46px"></div>{/each}
  </div>
{:else if mods.length === 0}
  <div class="empty">
    <Icon name="puzzle" size={30} />
    <strong>Sin mods todavía</strong>
    <p class="hint">Búscalos en Modrinth o en la comunidad, o importa tus archivos .jar.</p>
    <button class="btn primary" onclick={() => (browsing = true)}><Icon name="plus" size={15} /> Añadir mods</button>
  </div>
{:else}
  <div class="list">
    {#each filtered as mod (mod.file_name)}
      <div class="mod" class:off={!mod.enabled}>
        <button
          class="switch"
          role="switch"
          aria-checked={mod.enabled}
          aria-label={mod.enabled ? "Desactivar" : "Activar"}
          disabled={busy === mod.file_name}
          onclick={() => toggle(mod)}
        ><span></span></button>
        <span class="mod-name" title={mod.file_name}>{displayName(mod.file_name)}</span>
        <span class="size">{formatSize(mod.size_bytes)}</span>
        {#if confirmDelete === mod.file_name}
          <button class="btn sm danger" disabled={busy === mod.file_name} onclick={() => remove(mod)}>Borrar</button>
          <button class="btn sm ghost" onclick={() => (confirmDelete = null)}>No</button>
        {:else}
          {#if instance.loader === "fabric"}
            <button class="btn sm ghost icon" title="Publicar en la comunidad" onclick={() => (publishing = mod)}><Icon name="upload" size={15} /></button>
          {/if}
          <button class="btn sm ghost icon" title="Eliminar" onclick={() => (confirmDelete = mod.file_name)}><Icon name="trash" size={15} /></button>
        {/if}
      </div>
    {:else}
      <p class="hint">Ningún mod coincide con «{query}».</p>
    {/each}
  </div>
{/if}

{#if browsing}
  <ModBrowserModal
    {instance}
    onclose={() => (browsing = false)}
    onchanged={() => {
      void refresh();
      void app.refreshInstances();
    }}
  />
{/if}
{#if publishing}
  <PublishModModal {instance} mod={publishing} onclose={() => (publishing = null)} />
{/if}

<style>
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    width: 240px;
  }
  .search :global(svg) {
    position: absolute;
    left: 10px;
    color: var(--muted);
  }
  .search input {
    width: 100%;
    padding-left: 32px;
    height: 36px;
  }
  .count {
    font-size: 12px;
    color: var(--muted);
  }
  .spacer {
    flex: 1;
  }
  .alert {
    margin-bottom: 12px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .mod {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 46px;
    padding: 6px 8px 6px 12px;
    border-radius: var(--radius-sm);
    background: rgb(255 255 255 / 0.025);
    border: 1px solid transparent;
    transition: background var(--ease), border-color var(--ease);
  }
  .mod:hover {
    background: rgb(255 255 255 / 0.05);
    border-color: var(--border);
  }
  .mod-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }
  .off .mod-name {
    color: var(--muted);
    text-decoration: line-through;
  }
  .size {
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .switch {
    position: relative;
    width: 34px;
    height: 20px;
    flex: none;
    border-radius: 6px;
    border: none;
    background: rgb(255 255 255 / 0.14);
    cursor: pointer;
    transition: background var(--ease);
  }
  .switch span {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 4px;
    background: #fff;
    transition: transform var(--ease);
  }
  .switch[aria-checked="true"] {
    background: var(--accent);
  }
  .switch[aria-checked="true"] span {
    transform: translateX(14px);
    background: var(--accent-ink);
  }
  .switch:disabled {
    opacity: 0.5;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 40px 20px;
    color: var(--muted);
    text-align: center;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
  }
  .empty strong {
    color: var(--text);
  }
</style>
