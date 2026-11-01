<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { app, applyUi } from "./lib/app.svelte";
  import { nav } from "./lib/nav.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import Logo from "./components/Logo.svelte";
  import Home from "./pages/Home.svelte";
  import Instances from "./pages/Instances.svelte";
  import Discover from "./pages/Discover.svelte";
  import SettingsPage from "./pages/Settings.svelte";
  import SignInDialog from "./components/SignInDialog.svelte";
  import LocalSignInDialog from "./components/LocalSignInDialog.svelte";
  import UpdateDialog from "./components/UpdateDialog.svelte";

  onMount(() => app.init());
  $effect(() => {
    if (app.settings) applyUi(app.settings.ui);
  });
  const ms = $derived(app.settings?.ui.animations === false ? 0 : 200);
  const ui = $derived(app.settings?.ui);
</script>

{#if app.launcherBackground && ui}
  <div
    class="launcher-bg"
    style:background-image={`url("${app.launcherBackground}")`}
    style:background-size={ui.background_fit === "cover"
      ? "cover"
      : ui.background_fit === "contain"
        ? "contain"
        : "auto"}
    style:background-repeat={ui.background_fit === "tile" ? "repeat" : "no-repeat"}
    style:filter={ui.background_blur > 0 ? `blur(${ui.background_blur}px)` : undefined}
  ></div>
  <div class="launcher-bg-shade" style:opacity={ui.background_dim / 100}></div>
{/if}

<div class="shell">
  <TopBar />
  <main>
    {#if app.fatal}
      <div class="state">
        <Logo size={56} />
        <h1>No se pudo iniciar Obsidia</h1>
        <p class="alert">{app.fatal}</p>
      </div>
    {:else if !app.ready}
      <div class="state" aria-busy="true">
        <div class="skeleton hero-skel"></div>
        <div class="skel-row">
          <div class="skeleton"></div>
          <div class="skeleton"></div>
          <div class="skeleton"></div>
        </div>
      </div>
    {:else}
      {#key nav.page}
        <div class="page page-{nav.page}" in:fly={{ y: 8, duration: ms }}>
          {#if nav.page === "home"}
            <Home />
          {:else if nav.page === "instances"}
            <Instances />
          {:else if nav.page === "discover"}
            <Discover />
          {:else}
            <SettingsPage />
          {/if}
        </div>
      {/key}
    {/if}
  </main>
</div>

<SignInDialog />
<LocalSignInDialog />
<UpdateDialog />
<Toasts />

<style>
  .launcher-bg {
    position: fixed;
    inset: -64px;
    z-index: 0;
    background-position: center;
    background-color: var(--background);
    pointer-events: none;
  }
  .launcher-bg-shade {
    position: fixed;
    inset: 0;
    z-index: 0;
    background: #050409;
    pointer-events: none;
  }
  .shell {
    position: relative;
    z-index: 1;
    display: grid;
    grid-template-rows: var(--topbar-h) 1fr;
    height: 100%;
  }
  main {
    position: relative;
    min-height: 0;
    overflow: hidden;
  }
  .page {
    height: 100%;
    overflow-y: auto;
  }
  /* Instancias y Ajustes gestionan su propio scroll interno por columnas. */
  .page-instances,
  .page-settings {
    overflow: hidden;
  }
  .state {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 28px;
    height: 100%;
  }
  .hero-skel {
    height: 52%;
    border-radius: var(--radius-lg);
  }
  .skel-row {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr;
    gap: 16px;
    flex: 1;
    max-height: 200px;
  }
</style>
