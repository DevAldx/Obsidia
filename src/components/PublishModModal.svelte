<script lang="ts">
  import { untrack } from "svelte";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { errorText } from "../lib/format";
  import type { CatalogEntry, Instance, ModEntry } from "../lib/types";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  let { instance, mod, onclose }: { instance: Instance; mod: ModEntry; onclose: () => void } = $props();

  /** «sodium-fabric-0.6.0+mc1.21.1.jar» → «sodium fabric» como nombre de partida. */
  function guessName(file: string): string {
    const base = file.replace(/\.disabled$/, "").replace(/\.jar$/i, "");
    const words = base.split(/[-_+ ]+/).filter((w) => w && !/^(v?\d|mc\d|fabric$)/i.test(w));
    const name = words.join(" ") || base;
    return name.charAt(0).toUpperCase() + name.slice(1, 48);
  }

  let name = $state(untrack(() => guessName(mod.file_name)));
  let description = $state("");
  let rights = $state(false);
  let busy = $state(false);
  let error = $state("");
  let published = $state<CatalogEntry | null>(null);

  const isMicrosoft = $derived(app.account?.kind === "microsoft");

  async function publish() {
    busy = true;
    error = "";
    try {
      published = await api.catalogPublishMod(instance.id, mod.file_name, name.trim(), description.trim() || null);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title="Publicar mod en la comunidad" subtitle="Cualquier jugador de Obsidia podrá instalarlo desde «Añadir mods › Comunidad»." width={480} {busy} {onclose}>
  {#if published}
    <div class="done">
      <span class="done-icon facet"><Icon name="check" size={22} strokeWidth={2.4} /></span>
      <strong class="display">¡Publicado!</strong>
      <p class="muted">{published.name} ya aparece en la comunidad para Minecraft {published.minecraft_version}.</p>
    </div>
  {:else}
    <p class="hint file mono">{mod.file_name}</p>
    {#if !isMicrosoft}
      <div class="alert warn"><Icon name="info" size={16} /><span>Para publicar necesitas iniciar sesión con una cuenta de Microsoft.</span></div>
    {/if}
    <label class="field">
      <span>Nombre</span>
      <input class="input" bind:value={name} maxlength="48" disabled={busy} />
    </label>
    <label class="field">
      <span>Descripción (opcional)</span>
      <textarea class="input" rows="3" bind:value={description} maxlength="300" disabled={busy} placeholder="Qué hace y si necesita algún otro mod"></textarea>
    </label>
    <p class="hint">
      Se publica para Minecraft {instance.minecraft_version} con Fabric. Si el mod ya está en Modrinth no hace falta subirlo:
      los demás lo encontrarán allí.
    </p>
    <label class="check rights">
      <input type="checkbox" bind:checked={rights} disabled={busy} />
      <span>Es mío o tengo permiso de su autor para compartirlo.</span>
    </label>
    {#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}
  {/if}

  {#snippet footer()}
    {#if published}
      <button class="btn primary" onclick={onclose}>Listo</button>
    {:else}
      <button class="btn ghost" onclick={onclose} disabled={busy}>Cancelar</button>
      <button class="btn primary" onclick={publish} disabled={busy || !rights || !name.trim() || !isMicrosoft}>
        <Icon name="upload" size={15} /> {busy ? "Subiendo…" : "Publicar"}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .file {
    margin: 0 0 12px;
    word-break: break-all;
  }
  .field {
    margin-bottom: 12px;
  }
  .field textarea {
    resize: vertical;
  }
  .rights {
    margin: 10px 0;
  }
  .alert {
    margin: 10px 0;
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
