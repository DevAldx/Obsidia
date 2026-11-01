<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import Icon from "./Icon.svelte";
  import { app } from "../lib/app.svelte";

  let {
    title,
    subtitle = null,
    width = 480,
    busy = false,
    onclose,
    children,
    footer,
  }: {
    title: string;
    subtitle?: string | null;
    width?: number;
    /** Mientras es `true` no se puede cerrar (operación en curso). */
    busy?: boolean;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  const ms = $derived(app.settings?.ui.animations === false ? 0 : 140);

  function close() {
    if (!busy) onclose();
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="modal-backdrop" onclick={close} transition:fade={{ duration: ms }}>
  <div
    class="modal-panel"
    style:width={`${width}px`}
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    transition:scale={{ duration: ms, start: 0.97 }}
  >
    <header>
      <div>
        <h3>{title}</h3>
        {#if subtitle}<p class="hint">{subtitle}</p>{/if}
      </div>
      <button class="btn ghost icon sm" onclick={close} disabled={busy} aria-label="Cerrar">
        <Icon name="close" size={16} />
      </button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgb(4 3 8 / 0.72);
    display: grid;
    place-items: center;
    padding: 20px;
    overflow: auto;
  }
  .modal-panel {
    max-width: 100%;
    max-height: calc(100vh - 40px);
    display: flex;
    flex-direction: column;
    background: rgb(var(--surface) / 1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: 0 30px 80px -20px rgb(0 0 0 / 0.7);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 18px 20px 12px;
  }
  .body {
    padding: 4px 20px 18px;
    overflow-y: auto;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
    background: rgb(0 0 0 / 0.15);
    border-radius: 0 0 var(--radius-lg) var(--radius-lg);
  }
</style>
