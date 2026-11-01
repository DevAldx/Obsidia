<script lang="ts">
  import { fly } from "svelte/transition";
  import { toasts } from "../lib/toast.svelte";
  import { app } from "../lib/app.svelte";
  import Icon from "./Icon.svelte";

  const ms = $derived(app.settings?.ui.animations === false ? 0 : 180);
</script>

<div class="toasts" aria-live="polite">
  {#each toasts.list as t (t.id)}
    <div class="toast {t.kind}" transition:fly={{ x: 24, duration: ms }}>
      <Icon name={t.kind === "error" ? "alert" : t.kind === "success" ? "check" : "info"} size={16} />
      <span>{t.text}</span>
      <button onclick={() => toasts.dismiss(t.id)} aria-label="Descartar"><Icon name="close" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 90;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(360px, calc(100vw - 36px));
    pointer-events: none;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 11px 12px;
    border-radius: var(--radius);
    background: rgb(var(--surface-2) / 1);
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--accent);
    box-shadow: 0 18px 40px -16px rgb(0 0 0 / 0.7);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .toast :global(svg) { flex: none; margin-top: 2px; }
  .toast.success { border-left-color: var(--success); }
  .toast.success > :global(svg) { color: var(--success); }
  .toast.error { border-left-color: var(--danger); }
  .toast.error > :global(svg) { color: var(--danger); }
  span { flex: 1; }
  button {
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    padding: 2px;
  }
  button:hover { color: var(--text); }
</style>
