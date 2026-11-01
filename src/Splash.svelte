<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { api } from "./lib/api";
  import { applyUi } from "./lib/app.svelte";
  import type { StartupProgress } from "./lib/types";
  import Logo from "./components/Logo.svelte";

  const stepNames = ["Configuración", "Actualización", "Java", "Instancias", "Listo"];

  let progress = $state<StartupProgress>({
    step: 0,
    total: stepNames.length,
    percent: 4,
    label: "Iniciando Obsidia",
    detail: "Preparando la ventana de bienvenida…",
    done: false,
    version: "",
  });
  let icon = $state<string | null>(null);

  onMount(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    void (async () => {
      // Primero escuchamos y después pedimos el estado: si el arranque ya
      // avanzó, `getStartupProgress` devuelve el último paso conocido.
      try {
        unlisten = await listen<StartupProgress>("startup-progress", (event) => {
          if (!disposed) progress = event.payload;
        });
        if (disposed) {
          unlisten();
          return;
        }
        const current = await api.getStartupProgress();
        if (!disposed && current.step >= progress.step) progress = current;
      } catch {
        // La animación sigue con el estado local; el fallback de Rust abrirá
        // la ventana principal aunque este comando falle.
      }

      try {
        const settings = await api.getSettings();
        if (!disposed) applyUi(settings.ui);
      } catch {
        // Tema predeterminado.
      }

      try {
        const custom = await api.getLauncherIcon();
        if (!disposed && custom) {
          icon = `data:${custom.mime};base64,${custom.data_base64}`;
        }
      } catch {
        // Icono predeterminado.
      }
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<div class="splash">
  <div class="veins" aria-hidden="true"></div>

  <div class="stage">
    <div class="mark">
      <Logo size={76} custom={icon} />
    </div>

    <h1>OBSIDIA</h1>
    <p class="tagline">Forjado para jugar</p>

    <div class="status" aria-live="polite">
      <div class="status-line">
        <span class="label">{progress.label}</span>
        <span class="percent">{Math.round(progress.percent)}%</span>
      </div>
      <div
        class="track"
        role="progressbar"
        aria-label="Progreso de inicio"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={Math.round(progress.percent)}
      >
        <div class="fill" style:width={`${progress.percent}%`}></div>
      </div>
      <div class="steps" aria-hidden="true">
        {#each stepNames as name, index}
          <span
            class="step"
            class:done={progress.step > index + 1}
            class:current={progress.step === index + 1}
            title={name}
          ></span>
        {/each}
        <span class="detail">{progress.detail ?? "Preparando Obsidia…"}</span>
      </div>
    </div>
  </div>

  <footer>
    <span>v{progress.version || "…"}</span>
    <span>© 2026 Obsidia · No afiliado a Mojang Studios</span>
  </footer>
</div>

<style>
  .splash {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background:
      radial-gradient(420px 260px at 50% 38%, rgb(160 107 255 / 0.2), transparent 70%),
      linear-gradient(170deg, #110d1a 0%, #09080d 60%, #060509 100%);
    color: var(--text);
    user-select: none;
    border: 1px solid rgb(255 255 255 / 0.06);
  }
  /* Vetas isométricas fijas: sin animación ni desenfoque. */
  .veins {
    position: absolute;
    inset: 0;
    opacity: 0.6;
    background-image:
      repeating-linear-gradient(30deg, rgb(255 255 255 / 0.025) 0 1px, transparent 1px 38px),
      repeating-linear-gradient(150deg, rgb(255 255 255 / 0.025) 0 1px, transparent 1px 38px);
    mask-image: radial-gradient(circle at 50% 45%, #000 0%, transparent 70%);
    pointer-events: none;
  }
  .stage {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 0 48px;
    animation: rise 520ms cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  .mark {
    margin-bottom: 16px;
    animation: hover 3.6s ease-in-out infinite;
  }
  h1 {
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 700;
    letter-spacing: 0.32em;
    margin-right: -0.32em;
  }
  .tagline {
    margin-top: 4px;
    font-size: 11px;
    letter-spacing: 0.26em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .status {
    width: min(400px, 100%);
    margin-top: 30px;
  }
  .status-line {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 8px;
    font-size: 12px;
  }
  .label {
    font-weight: 600;
  }
  .percent {
    font-variant-numeric: tabular-nums;
    color: var(--accent-hover);
    font-weight: 600;
  }
  .track {
    height: 4px;
    background: rgb(255 255 255 / 0.08);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 360ms cubic-bezier(0.2, 0.7, 0.2, 1);
  }
  .steps {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 10px;
  }
  .step {
    width: 6px;
    height: 6px;
    background: rgb(255 255 255 / 0.15);
    transition: background 200ms ease;
  }
  .step.done {
    background: var(--accent);
  }
  .step.current {
    background: var(--accent-hover);
    animation: blink 900ms ease-in-out infinite;
  }
  .detail {
    margin-left: 8px;
    font-size: 11px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  footer {
    position: relative;
    display: flex;
    justify-content: space-between;
    padding: 12px 18px;
    font-size: 10px;
    letter-spacing: 0.06em;
    color: rgb(255 255 255 / 0.35);
  }
  @keyframes rise {
    from { opacity: 0; transform: translateY(8px); }
    to { opacity: 1; transform: none; }
  }
  @keyframes hover {
    50% { transform: translateY(-4px); }
  }
  @keyframes blink {
    50% { opacity: 0.4; }
  }
</style>
