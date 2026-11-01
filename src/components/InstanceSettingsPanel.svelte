<script lang="ts">
  import { untrack } from "svelte";
  import { api, joinArgs, splitArgs } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { nav } from "../lib/nav.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { errorText, formatMemory } from "../lib/format";
  import { BACKGROUND_ACCEPT, BACKGROUND_TYPES, MAX_BACKGROUND_BYTES, mimeOf, readAsBase64 } from "../lib/files";
  import type { Instance, JavaChoice } from "../lib/types";
  import Icon from "./Icon.svelte";

  // El padre lo recrea con {#key} al cambiar de instancia.
  let { instance }: { instance: Instance } = $props();

  const init = untrack(() => instance);
  let name = $state(init.name);
  let minRam = $state(init.memory.min_mb);
  let maxRam = $state(init.memory.max_mb);
  let jvm = $state(joinArgs(init.jvm_args));
  let javaPath = $state(init.java.kind === "path" ? init.java.path : "");
  let busy = $state(false);
  let error = $state("");
  let confirmDelete = $state(false);

  // Si el Java manual guardado ya no existe en este PC, se avisa.
  const javaMissing = $derived(javaPath !== "" && !app.java.some((j) => j.path === javaPath) && !app.javaLoading);

  const dirty = $derived(
    name !== instance.name ||
      Number(minRam) !== instance.memory.min_mb ||
      Number(maxRam) !== instance.memory.max_mb ||
      jvm.trim() !== joinArgs(instance.jvm_args) ||
      javaPath !== (instance.java.kind === "path" ? instance.java.path : ""),
  );

  async function save() {
    busy = true;
    error = "";
    try {
      const java: JavaChoice = javaPath ? { kind: "path", path: javaPath } : { kind: "auto" };
      await api.updateInstance(instance.id, {
        name: name.trim() || instance.name,
        memory: { min_mb: Number(minRam), max_mb: Number(maxRam) },
        jvm_args: splitArgs(jvm),
        java,
      });
      await app.refreshInstances();
      toasts.success("Ajustes de la instancia guardados");
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    busy = true;
    try {
      app.settings = await api.deleteInstance(instance.id);
      await app.refreshInstances();
      nav.instanceId = null;
      toasts.success(`«${instance.name}» eliminada`);
    } catch (e) {
      error = errorText(e);
      busy = false;
    }
  }

  // ── Fondo de la instancia ──
  let bgInput = $state<HTMLInputElement | null>(null);
  let bgBusy = $state(false);
  const bg = $derived(app.backgrounds[instance.id]);

  async function onBackgroundChosen(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    const mime = mimeOf(file);
    if (!BACKGROUND_TYPES.includes(mime)) return toasts.push("error", "Usa una imagen PNG, JPG, WEBP o GIF.");
    if (file.size > MAX_BACKGROUND_BYTES) return toasts.push("error", "La imagen supera los 15 MB.");
    bgBusy = true;
    try {
      await api.setInstanceBackground(instance.id, mime, await readAsBase64(file));
      app.invalidateBackground(instance.id);
      await app.ensureBackground(instance.id);
    } catch (e) {
      toasts.error(e, "No se pudo guardar el fondo");
    } finally {
      bgBusy = false;
    }
  }

  async function clearBackground() {
    bgBusy = true;
    try {
      await api.clearInstanceBackground(instance.id);
      app.invalidateBackground(instance.id);
      await app.ensureBackground(instance.id);
    } catch (e) {
      toasts.error(e);
    } finally {
      bgBusy = false;
    }
  }
</script>

<div class="grid">
  <section class="block">
    <h2>General</h2>
    <label class="field"><span>Nombre</span><input bind:value={name} maxlength="48" /></label>
    <div class="row">
      <label class="field"><span>Versión de Minecraft</span><input value={instance.minecraft_version} disabled /></label>
      <label class="field"><span>Loader</span><input value={instance.loader === "fabric" ? `Fabric ${instance.loader_version ?? ""}` : "Vanilla"} disabled /></label>
    </div>
    <p class="hint">La versión y el loader quedan fijados al crear la instancia.</p>
  </section>

  <section class="block">
    <h2>Memoria</h2>
    <div class="mem">
      <strong class="display">{formatMemory(Number(maxRam) || 0)}</strong>
      <span class="hint">máximo para esta instancia</span>
    </div>
    <input type="range" min="1024" max="16384" step="512" bind:value={maxRam} aria-label="RAM máxima" />
    <div class="row">
      <label class="field"><span>Mínima (MB)</span><input type="number" step="256" min="256" bind:value={minRam} /></label>
      <label class="field"><span>Máxima (MB)</span><input type="number" step="256" min="512" bind:value={maxRam} /></label>
    </div>
  </section>

  <section class="block">
    <h2>Java</h2>
    <label class="field">
      <span>Entorno de Java</span>
      <select bind:value={javaPath}>
        <option value="">Automático (el que pida la versión)</option>
        {#each app.java as j (j.path)}<option value={j.path}>Java {j.major} · {j.path}</option>{/each}
        {#if javaMissing}<option value={javaPath}>No encontrado · {javaPath}</option>{/if}
      </select>
    </label>
    {#if javaMissing}
      <div class="alert warn"><Icon name="alert" size={16} /><span>Ese Java no existe en este PC. Elige «Automático» y Obsidia descargará el que haga falta.</span></div>
    {/if}
    <label class="field">
      <span>Argumentos JVM</span>
      <textarea rows="2" bind:value={jvm} placeholder="-XX:+UseG1GC"></textarea>
    </label>
  </section>

  <section class="block">
    <h2>Fondo</h2>
    <div class="bg-preview" class:obsidia-bg={!bg} style:background-image={bg ? `url(${bg})` : undefined}>
      <span>{instance.name}</span>
    </div>
    <input bind:this={bgInput} type="file" accept={BACKGROUND_ACCEPT} hidden onchange={onBackgroundChosen} />
    <div class="bg-actions">
      <button class="btn" onclick={() => bgInput?.click()} disabled={bgBusy}>{bgBusy ? "Guardando…" : "Elegir imagen…"}</button>
      {#if bg}<button class="btn ghost" onclick={clearBackground} disabled={bgBusy}>Quitar</button>{/if}
    </div>
    <p class="hint">PNG, JPG, WEBP o GIF animado, hasta 15 MB. Se usa en Inicio y en la biblioteca.</p>
  </section>
</div>

{#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}

<div class="savebar" class:visible={dirty}>
  <span class="hint">Cambios sin guardar</span>
  <button class="btn primary" onclick={save} disabled={busy || !dirty}>{busy ? "Guardando…" : "Guardar cambios"}</button>
</div>

<section class="danger-zone">
  <div>
    <strong>Eliminar instancia</strong>
    <p class="hint">Borra la carpeta <span class="mono">{instance.id}</span> con sus mundos, mods y ajustes. No se puede deshacer.</p>
  </div>
  {#if confirmDelete}
    <button class="btn ghost" onclick={() => (confirmDelete = false)} disabled={busy}>Cancelar</button>
    <button class="btn danger" onclick={remove} disabled={busy}>Sí, borrar todo</button>
  {:else}
    <button class="btn danger-soft" onclick={() => (confirmDelete = true)}><Icon name="trash" size={15} /> Eliminar</button>
  {/if}
</section>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }
  .block {
    padding: 16px 18px 4px;
    border-radius: var(--radius);
    background: rgb(var(--surface) / var(--surface-alpha));
    border: 1px solid var(--border);
  }
  .mem {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 6px;
  }
  .mem strong {
    font-size: 26px;
  }
  .mem + input {
    margin-bottom: 14px;
  }
  .bg-preview {
    display: flex;
    align-items: flex-end;
    height: 92px;
    margin-bottom: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background-size: cover;
    background-position: center;
    border: 1px solid var(--border);
    font-family: var(--font-display);
    font-weight: 600;
    text-shadow: 0 1px 8px rgb(0 0 0 / 0.8);
  }
  .bg-actions {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  .alert {
    margin: 0 0 14px;
  }
  .savebar {
    position: sticky;
    bottom: 12px;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 14px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: rgb(var(--surface-2) / 1);
    border: 1px solid var(--border-strong);
    opacity: 0;
    transform: translateY(8px);
    pointer-events: none;
    transition: opacity var(--ease), transform var(--ease);
  }
  .savebar.visible {
    opacity: 1;
    transform: none;
    pointer-events: auto;
  }
  .danger-zone {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 14px;
    padding: 14px 18px;
    border-radius: var(--radius);
    border: 1px solid rgb(255 92 114 / 0.25);
    background: rgb(255 92 114 / 0.04);
  }
  .danger-zone > div {
    flex: 1;
  }
  @media (max-width: 1100px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
