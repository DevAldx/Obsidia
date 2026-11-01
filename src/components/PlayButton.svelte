<script lang="ts">
  import { app } from "../lib/app.svelte";
  import Icon from "./Icon.svelte";

  let { id, big = false }: { id: string; big?: boolean } = $props();

  const state = $derived(app.games[id]);
  const prog = $derived(app.progress[id]);
  const pct = $derived(prog && prog.total > 0 ? Math.min(100, Math.round((prog.done / prog.total) * 100)) : null);
</script>

{#if state === "running"}
  <button class="play facet stop" class:big onclick={() => app.stop(id)}>
    <Icon name="stop" size={big ? 18 : 14} strokeWidth={2.4} />
    <span>Detener</span>
  </button>
{:else if state === "preparing"}
  <div class="play facet prep" class:big role="status" aria-live="polite">
    <div class="prep-fill" class:indeterminate={pct === null} style:width={pct === null ? undefined : `${pct}%`}></div>
    <span class="prep-text">
      <span class="phase">{prog?.phase ?? "Preparando…"}</span>
      {#if pct !== null}<span class="pct">{pct}%</span>{/if}
    </span>
  </div>
{:else}
  <button class="play facet go" class:big onclick={() => app.play(id)}>
    <Icon name="play" size={big ? 18 : 14} strokeWidth={2.4} />
    <span>Jugar</span>
  </button>
{/if}

<style>
  .play {
    --facet: 8px;
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    height: 36px;
    min-width: 112px;
    padding: 0 18px;
    border: none;
    cursor: pointer;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 13px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    overflow: hidden;
    transition: background var(--ease), transform var(--ease), filter var(--ease);
  }
  .play.big {
    --facet: 12px;
    height: 56px;
    min-width: 220px;
    padding: 0 34px;
    font-size: 17px;
    letter-spacing: 0.2em;
  }
  .go {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .go:hover {
    background: var(--accent-hover);
  }
  .go:active,
  .stop:active {
    transform: translateY(1px);
  }
  .go :global(svg) {
    fill: currentColor;
  }
  .stop {
    background: var(--danger);
    color: #fff;
  }
  .stop:hover {
    filter: brightness(1.1);
  }
  .stop :global(svg) {
    fill: currentColor;
  }
  .prep {
    cursor: default;
    background: rgb(255 255 255 / 0.08);
    color: var(--text);
    min-width: 220px;
  }
  .prep.big {
    min-width: 300px;
  }
  .prep-fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: color-mix(in srgb, var(--accent) 45%, transparent);
    transition: width var(--ease-slow);
  }
  .prep-fill.indeterminate {
    width: 35%;
    animation: sweep 1.2s ease-in-out infinite alternate;
  }
  .prep-text {
    position: relative;
    display: flex;
    gap: 10px;
    align-items: baseline;
    font-size: 11px;
    letter-spacing: 0.1em;
  }
  .big .prep-text {
    font-size: 12px;
  }
  .phase {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pct {
    font-variant-numeric: tabular-nums;
    color: var(--accent-hover);
  }
  @keyframes sweep {
    from { left: 0; }
    to { left: 65%; }
  }
</style>
