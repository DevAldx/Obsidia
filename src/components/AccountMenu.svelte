<script lang="ts">
  import { fly } from "svelte/transition";
  import { app } from "../lib/app.svelte";
  import { nav } from "../lib/nav.svelte";
  import SkinFace from "./SkinFace.svelte";
  import Icon from "./Icon.svelte";

  let open = $state(false);
  let root = $state<HTMLDivElement | null>(null);
  const ms = $derived(app.settings?.ui.animations === false ? 0 : 140);

  function onWindowClick(e: MouseEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }

  function run(action: () => unknown) {
    open = false;
    void action();
  }

  const kindLabel = $derived(
    !app.account
      ? ""
      : app.account.kind === "offline"
        ? "Cuenta local"
        : app.account.flow === "legacy"
          ? "Microsoft · flujo heredado"
          : "Microsoft",
  );
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="account" bind:this={root}>
  <button class="trigger" class:open onclick={() => (open = !open)} aria-haspopup="menu" aria-expanded={open}>
    {#if app.account}
      <SkinFace url={app.account.skin_url} username={app.account.username} size={26} />
      <span class="name">{app.account.username}</span>
    {:else}
      <span class="anon"><Icon name="user" size={15} /></span>
      <span class="name">{app.signInBusy ? "Conectando…" : "Iniciar sesión"}</span>
      <span class="attention" aria-hidden="true"></span>
    {/if}
    <Icon name="chevronDown" size={14} />
  </button>

  {#if open}
    <div class="menu" role="menu" transition:fly={{ y: -6, duration: ms }}>
      {#if app.account}
        <div class="profile">
          <SkinFace url={app.account.skin_url} username={app.account.username} size={44} />
          <div>
            <strong>{app.account.username}</strong>
            <span class="hint">{kindLabel}</span>
          </div>
        </div>
        {#if app.account.kind === "offline"}
          <p class="hint note">Solo sirve en servidores con <span class="mono">online-mode=false</span>.</p>
          <button class="item" role="menuitem" onclick={() => run(() => app.openLocalSignIn())}>
            <Icon name="edit" size={16} /> Cambiar nombre
          </button>
        {/if}
        <button class="item" role="menuitem" onclick={() => run(() => nav.openSettings("account"))}>
          <Icon name="settings" size={16} /> Gestionar cuenta
        </button>
        <button class="item danger" role="menuitem" onclick={() => run(() => app.signOut())}>
          <Icon name="logout" size={16} /> Cerrar sesión
        </button>
      {:else}
        <div class="signin">
          <strong class="display">Entra para jugar</strong>
          <p class="hint">Con Microsoft accedes a servidores oficiales. La cuenta local no necesita contraseña.</p>
        </div>
        <button
          class="btn primary wide"
          onclick={() => run(() => app.startLegacySignIn())}
          disabled={app.signInBusy}
          title="Flujo heredado de Microsoft, sin Azure (no oficial)"
        >
          {app.signInBusy && app.signInLegacy ? "Esperando a Microsoft…" : "Iniciar sesión con Microsoft"}
        </button>
        <button class="btn wide" onclick={() => run(() => app.openLocalSignIn())}>
          <Icon name="user" size={15} /> Cuenta local
        </button>
        <button class="item subtle" role="menuitem" onclick={() => run(() => nav.openSettings("account"))}>
          Más opciones (Azure propia)…
        </button>
      {/if}
      {#if app.signInError}<p class="error">{app.signInError}</p>{/if}
    </div>
  {/if}
</div>

<style>
  .account {
    position: relative;
  }
  .trigger {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 10px 0 5px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: rgb(255 255 255 / 0.03);
    cursor: pointer;
    color: var(--text-2);
    transition: background var(--ease), border-color var(--ease);
  }
  .trigger:hover,
  .trigger.open {
    background: rgb(255 255 255 / 0.07);
    border-color: var(--border-strong);
    color: var(--text);
  }
  .name {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .anon {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    display: grid;
    place-items: center;
    background: rgb(255 255 255 / 0.08);
  }
  .attention {
    position: absolute;
    top: -3px;
    right: -3px;
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--background);
  }
  .menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 80;
    width: 290px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: rgb(var(--surface-2) / 1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 24px 60px -20px rgb(0 0 0 / 0.8);
  }
  .profile {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 6px 10px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .profile div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .profile strong {
    font-size: 15px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .note {
    padding: 0 6px 4px;
  }
  .signin {
    padding: 6px 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .signin strong {
    font-size: 16px;
  }
  .wide {
    width: 100%;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 8px;
    border: none;
    border-radius: 8px;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-2);
    font-size: 13px;
  }
  .item:hover {
    background: rgb(255 255 255 / 0.06);
    color: var(--text);
  }
  .item.danger:hover {
    color: var(--danger);
  }
  .item.subtle {
    justify-content: center;
    font-size: 12px;
    color: var(--muted);
  }
</style>
