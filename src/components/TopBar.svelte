<script lang="ts">
  import { getCurrentWindow, type Window } from "@tauri-apps/api/window";
  import { app } from "../lib/app.svelte";
  import { nav, type Page } from "../lib/nav.svelte";
  import Icon, { type IconName } from "./Icon.svelte";
  import Logo from "./Logo.svelte";
  import AccountMenu from "./AccountMenu.svelte";

  const items: { id: Page; label: string; icon: IconName }[] = [
    { id: "home", label: "Inicio", icon: "home" },
    { id: "instances", label: "Instancias", icon: "library" },
    { id: "discover", label: "Descubrir", icon: "compass" },
    { id: "settings", label: "Ajustes", icon: "settings" },
  ];

  // En un navegador normal (vite dev sin Tauri) no hay ventana que controlar.
  const win: Window | null = (() => {
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  })();

  /** Primer juego activo: se muestra como indicador en la barra. */
  const activity = $derived.by(() => {
    const entry = Object.entries(app.games)[0];
    if (!entry) return null;
    const [id, state] = entry;
    const inst = app.list.instances.find((i) => i.id === id);
    const p = app.progress[id];
    const pct = p && p.total > 0 ? Math.min(100, Math.round((p.done / p.total) * 100)) : null;
    return { id, state, name: inst?.name ?? id, phase: p?.phase ?? "Preparando", pct };
  });
</script>

<header class="topbar" data-tauri-drag-region>
  <button class="brand" onclick={() => nav.go("home")} title="Obsidia">
    <Logo size={26} custom={app.launcherIcon} />
    <span class="wordmark">OBSIDIA</span>
  </button>

  <nav aria-label="Secciones">
    {#each items as item (item.id)}
      <button class:active={nav.page === item.id} onclick={() => nav.go(item.id)} aria-current={nav.page === item.id ? "page" : undefined}>
        <Icon name={item.icon} size={16} />
        <span>{item.label}</span>
      </button>
    {/each}
  </nav>

  <div class="drag" data-tauri-drag-region></div>

  {#if activity}
    <button class="activity" onclick={() => nav.openInstance(activity.id, "logs")} title="Ver registros">
      <span class="dot" class:live={activity.state === "running"} class:busy={activity.state !== "running"}></span>
      <span class="act-text">
        {#if activity.state === "running"}
          Jugando · <strong>{activity.name}</strong>
        {:else}
          {activity.phase}{activity.pct !== null ? ` · ${activity.pct}%` : "…"}
        {/if}
      </span>
      <Icon name="terminal" size={14} />
    </button>
  {/if}

  {#if app.account?.kind === "microsoft"}
    <button
      class="skin-btn"
      class:active={nav.page === "settings" && nav.settingsSection === "skin"}
      onclick={() => nav.openSettings("skin")}
      title="Skin y capa"
    >
      <Icon name="shirt" size={16} />
      <span>Skin</span>
    </button>
  {/if}
  <AccountMenu />

  {#if win}
    <div class="window-controls">
      <button onclick={() => win?.minimize()} aria-label="Minimizar"><Icon name="minus" size={15} /></button>
      <button onclick={() => win?.toggleMaximize()} aria-label="Maximizar"><Icon name="square" size={12} /></button>
      <button class="close" onclick={() => win?.close()} aria-label="Cerrar"><Icon name="close" size={15} /></button>
    </div>
  {/if}
</header>

<style>
  .topbar {
    position: relative;
    z-index: 20;
    display: flex;
    align-items: center;
    gap: 6px;
    height: var(--topbar-h);
    padding: 0 0 0 14px;
    background: rgb(var(--surface) / 0.86);
    border-bottom: 1px solid var(--border);
    user-select: none;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px 6px 4px;
    margin-right: 14px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text);
  }
  .wordmark {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 15px;
    letter-spacing: 0.24em;
  }
  nav {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 100%;
  }
  nav button {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    padding: 0 14px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
    transition: color var(--ease);
  }
  nav button:hover {
    color: var(--text);
  }
  nav button.active {
    color: var(--text);
  }
  nav button::after {
    content: "";
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: -1px;
    height: 3px;
    background: var(--accent);
    clip-path: polygon(3px 0, 100% 0, calc(100% - 3px) 100%, 0 100%);
    transform: scaleX(0);
    transition: transform var(--ease);
  }
  nav button.active::after {
    transform: scaleX(1);
  }
  .drag {
    flex: 1;
    align-self: stretch;
  }
  .skin-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 36px;
    padding: 0 12px;
    margin-right: 6px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: rgb(255 255 255 / 0.03);
    color: var(--text-2);
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
    transition: background var(--ease), border-color var(--ease), color var(--ease);
  }
  .skin-btn:hover,
  .skin-btn.active {
    color: var(--text);
    border-color: var(--border-strong);
    background: rgb(255 255 255 / 0.07);
  }
  .skin-btn.active :global(svg) {
    color: var(--accent-hover);
  }
  .activity {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 12px;
    margin-right: 6px;
    border-radius: 8px;
    border: 1px solid var(--border-strong);
    background: rgb(0 0 0 / 0.25);
    cursor: pointer;
    color: var(--text-2);
    font-size: 12px;
    max-width: 280px;
  }
  .activity:hover {
    color: var(--text);
    border-color: rgb(255 255 255 / 0.25);
  }
  .act-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .window-controls {
    display: flex;
    align-self: stretch;
    margin-left: 10px;
  }
  .window-controls button {
    width: 46px;
    display: grid;
    place-items: center;
    background: none;
    border: none;
    color: var(--muted);
    cursor: pointer;
    transition: background var(--ease), color var(--ease);
  }
  .window-controls button:hover {
    background: rgb(255 255 255 / 0.07);
    color: var(--text);
  }
  .window-controls .close:hover {
    background: #e5484d;
    color: #fff;
  }
  @media (max-width: 1120px) {
    .wordmark {
      display: none;
    }
    .brand {
      margin-right: 4px;
    }
    .activity {
      max-width: 190px;
    }
  }
  @media (max-width: 860px) {
    nav button span {
      display: none;
    }
  }
</style>
