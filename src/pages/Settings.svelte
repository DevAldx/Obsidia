<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { api, joinArgs, splitArgs } from "../lib/api";
  import { nav, type SettingsSection } from "../lib/nav.svelte";
  import { updates } from "../lib/updates.svelte";
  import { toasts } from "../lib/toast.svelte";
  import { errorText, formatMemory } from "../lib/format";
  import type { Settings, UpdateCheck } from "../lib/types";
  import Icon, { type IconName } from "../components/Icon.svelte";
  import SkinFace from "../components/SkinFace.svelte";
  import Appearance from "./Appearance.svelte";
  import About from "./About.svelte";
  import Skins from "./Skins.svelte";

  const sections: { id: SettingsSection; label: string; icon: IconName; group: string }[] = [
    { id: "general", label: "General", icon: "settings", group: "Launcher" },
    { id: "account", label: "Cuenta", icon: "user", group: "Launcher" },
    { id: "skin", label: "Skin y capa", icon: "shirt", group: "Launcher" },
    { id: "appearance", label: "Apariencia", icon: "palette", group: "Launcher" },
    { id: "updates", label: "Actualizaciones", icon: "refresh", group: "Launcher" },
    { id: "minecraft", label: "Minecraft", icon: "monitor", group: "Juego" },
    { id: "java", label: "Java y memoria", icon: "coffee", group: "Juego" },
    { id: "sharing", label: "Compartir", icon: "share", group: "Juego" },
    { id: "about", label: "Acerca de", icon: "info", group: "Obsidia" },
  ];
  const groups = ["Launcher", "Juego", "Obsidia"];

  // ── Borrador de los ajustes generales (se guardan con la barra inferior) ──
  type Draft = {
    minecraft: Settings["minecraft"];
    jvm: string;
    dataDir: string;
    clientId: string;
    manifestUrl: string;
    sharingUrl: string;
  };
  function fromSettings(s: Settings): Draft {
    return {
      minecraft: structuredClone($state.snapshot(s.minecraft)),
      jvm: joinArgs(s.minecraft.jvm_args),
      dataDir: s.data_dir ?? "",
      clientId: s.account.microsoft_client_id ?? "",
      manifestUrl: s.updates.manifest_url ?? "",
      sharingUrl: s.sharing.manifest_base_url ?? "",
    };
  }
  const normalize = (d: Draft) =>
    JSON.stringify({
      ...d,
      minecraft: { ...d.minecraft, jvm_args: [] },
      jvm: splitArgs(d.jvm).join(" "),
      dataDir: d.dataDir.trim(),
      clientId: d.clientId.trim(),
      manifestUrl: d.manifestUrl.trim(),
      sharingUrl: d.sharingUrl.trim(),
    });

  let draft = $state<Draft>(fromSettings(app.settings!));
  let saving = $state(false);
  let error = $state("");
  const dirty = $derived(app.settings ? normalize(draft) !== normalize(fromSettings(app.settings)) : false);

  async function save() {
    if (!app.settings) return;
    error = "";
    saving = true;
    try {
      // Se parte de los ajustes vigentes para no pisar la apariencia ni la
      // instancia seleccionada, que se guardan por otras vías.
      const next = $state.snapshot(app.settings) as Settings;
      next.minecraft = { ...$state.snapshot(draft.minecraft), jvm_args: splitArgs(draft.jvm) };
      next.data_dir = draft.dataDir.trim() || null;
      next.account.microsoft_client_id = draft.clientId.trim() || null;
      next.updates.manifest_url = draft.manifestUrl.trim() || null;
      next.sharing.manifest_base_url = draft.sharingUrl.trim() || null;
      await app.saveSettings(next);
      draft = fromSettings(app.settings);
      toasts.success("Ajustes guardados");
    } catch (e) {
      error = errorText(e);
    } finally {
      saving = false;
    }
  }

  function discard() {
    if (app.settings) draft = fromSettings(app.settings);
    error = "";
  }

  // ── Manifiesto manual de actualización ──
  let checking = $state(false);
  let checkResult = $state<UpdateCheck | null>(null);
  let checkError = $state("");
  async function checkManual() {
    checking = true;
    checkError = "";
    try {
      checkResult = await api.checkForUpdates();
    } catch (e) {
      checkError = errorText(e);
    } finally {
      checking = false;
    }
  }
  async function openDownload(url: string) {
    try {
      await api.openHttpsUrl(url);
    } catch (e) {
      toasts.error(e);
    }
  }

  const resolutions: [number, number][] = [
    [854, 480],
    [1280, 720],
    [1600, 900],
    [1920, 1080],
  ];
</script>

<div class="settings">
  <aside class="subnav">
    <h1>Ajustes</h1>
    {#each groups as g (g)}
      <p class="group">{g}</p>
      {#each sections.filter((s) => s.group === g) as s (s.id)}
        <button class:active={nav.settingsSection === s.id} onclick={() => (nav.settingsSection = s.id)}>
          <Icon name={s.icon} size={16} />
          {s.label}
        </button>
      {/each}
    {/each}
  </aside>

  <div class="content">
    <div class="inner">
      {#key nav.settingsSection}
        {@const meta = sections.find((s) => s.id === nav.settingsSection)}
        <header class="head">
          <h2 class="title display">{meta?.label}</h2>
        </header>

        {#if nav.settingsSection === "general"}
          <div class="group-block">
            <label class="field">
              <span>Directorio de datos (vacío = predeterminado)</span>
              <input class="mono" bind:value={draft.dataDir} placeholder={app.info?.data_dir} />
            </label>
            <p class="hint">
              En uso: <span class="mono">{app.info?.data_dir}</span>. El cambio se aplica al reiniciar Obsidia
              y no mueve las instancias existentes.
            </p>
          </div>
          <div class="group-block">
            <p class="label">Configuración</p>
            <p class="hint">Archivo de ajustes en <span class="mono">{app.info?.config_dir}</span>.</p>
          </div>
        {:else if nav.settingsSection === "account"}
          {#if app.account}
            <div class="account-card">
              <SkinFace url={app.account.skin_url} username={app.account.username} size={56} />
              <div class="ac-text">
                <strong class="display">{app.account.username}</strong>
                <span class="tag" class:accent={app.account.kind !== "offline"}>
                  {app.account.kind === "offline" ? "Cuenta local" : app.account.flow === "legacy" ? "Microsoft · heredado" : "Microsoft"}
                </span>
              </div>
              {#if app.account.kind === "offline"}
                <button class="btn" onclick={() => app.openLocalSignIn()}><Icon name="edit" size={15} /> Cambiar nombre</button>
              {/if}
              <button class="btn danger-soft" onclick={() => app.signOut()}><Icon name="logout" size={15} /> Cerrar sesión</button>
            </div>
            {#if app.account.kind === "offline"}
              <div class="alert info"><Icon name="info" size={16} /><span>Sin Microsoft ni Azure. Funciona en servidores con <span class="mono">online-mode=false</span>; los servidores oficiales la rechazan.</span></div>
            {:else if app.account.flow === "legacy"}
              <div class="alert info"><Icon name="info" size={16} /><span>Sesión iniciada con el flujo heredado del launcher oficial. Funciona sin Azure, pero no es una integración oficial: Microsoft podría bloquearla. Si falla, usa una app de Azure propia o la cuenta local.</span></div>
            {/if}
          {:else}
            <div class="signin-options">
              <button class="opt primary-opt" onclick={() => app.startLegacySignIn()} disabled={app.signInBusy}>
                <Icon name="user" size={22} />
                <strong>{app.signInBusy && app.signInLegacy ? "Esperando a Microsoft…" : "Microsoft"}</strong>
                <span class="hint">Para servidores oficiales. Flujo heredado: no necesitas Azure, pero no es oficial y podría bloquearse.</span>
              </button>
              <button class="opt" onclick={() => app.openLocalSignIn()}>
                <Icon name="edit" size={22} />
                <strong>Cuenta local</strong>
                <span class="hint">Solo un nombre. Para servidores con <span class="mono">online-mode=false</span>.</span>
              </button>
            </div>
            <details class="advanced">
              <summary>Usar mi propia app de Azure (avanzado)</summary>
              <label class="field">
                <span>Client ID de Microsoft</span>
                <input class="mono" bind:value={draft.clientId} placeholder="00000000-0000-0000-0000-000000000000" />
              </label>
              <p class="hint">GUID de tu aplicación registrada en portal.azure.com y aprobada para la API de Minecraft. Guarda los cambios antes de iniciar sesión. Pasos en el README.</p>
              <button class="btn" onclick={() => app.startSignIn()} disabled={app.signInBusy || dirty}>Iniciar sesión con Azure</button>
            </details>
          {/if}
          {#if app.signInError}<div class="alert"><Icon name="alert" size={16} /><span>{app.signInError}</span></div>{/if}
        {:else if nav.settingsSection === "skin"}
          <Skins />
        {:else if nav.settingsSection === "appearance"}
          <Appearance />
        {:else if nav.settingsSection === "updates"}
          <div class="update-hero">
            <div>
              <p class="label">Versión instalada</p>
              <strong class="display ver">v{app.info?.version}</strong>
            </div>
            <div class="update-actions">
              {#if updates.status && !updates.status.available}<span class="tag ok"><Icon name="check" size={13} /> Al día</span>{/if}
              <button class="btn primary" onclick={() => updates.check()} disabled={updates.checking || updates.installing}>
                <Icon name="refresh" size={15} /> {updates.checking ? "Buscando…" : "Buscar actualizaciones"}
              </button>
            </div>
          </div>
          {#if updates.error}<div class="alert"><Icon name="alert" size={16} /><span>{updates.error}</span></div>{/if}
          <p class="hint">Obsidia comprueba actualizaciones firmadas al iniciar. Si aceptas una, se descarga, se instala y la aplicación se reinicia.</p>
          <details class="advanced">
            <summary>Manifiesto manual (avanzado)</summary>
            <label class="field">
              <span>URL del manifiesto de actualización</span>
              <input class="mono" bind:value={draft.manifestUrl} placeholder="https://ejemplo.com/obsidia/latest.json" />
            </label>
            <p class="hint">Fuente alternativa para comprobar a mano un <span class="mono">latest.json</span> propio y abrir su descarga en el navegador. Guarda antes de comprobar.</p>
            <button class="btn" onclick={checkManual} disabled={checking || dirty}>{checking ? "Buscando…" : "Comprobar manifiesto manual"}</button>
            {#if checkError}<div class="alert"><Icon name="alert" size={16} /><span>{checkError}</span></div>{/if}
            {#if checkResult}
              {#if checkResult.available}
                <p class="hint">Hay una versión nueva: <strong>v{checkResult.available.version}</strong>{#if checkResult.available.notes} — {checkResult.available.notes}{/if}</p>
                <button class="btn" onclick={() => openDownload(checkResult!.available!.url)}>Abrir descarga</button>
              {:else}
                <p class="hint">Ya tienes la última versión.</p>
              {/if}
            {/if}
          </details>
        {:else if nav.settingsSection === "minecraft"}
          <p class="hint intro">Valores por defecto para las instancias nuevas y opciones de ventana del juego.</p>
          <div class="group-block">
            <p class="label">Resolución de la ventana</p>
            <div class="chips">
              {#each resolutions as [w, h] (w)}
                <button class="chip" class:active={draft.minecraft.width === w && draft.minecraft.height === h} onclick={() => { draft.minecraft.width = w; draft.minecraft.height = h; }}>{w}×{h}</button>
              {/each}
            </div>
            <div class="row">
              <label class="field"><span>Ancho</span><input type="number" min="320" bind:value={draft.minecraft.width} /></label>
              <label class="field"><span>Alto</span><input type="number" min="240" bind:value={draft.minecraft.height} /></label>
            </div>
            <label class="field check"><input type="checkbox" bind:checked={draft.minecraft.fullscreen} /><span>Iniciar en pantalla completa</span></label>
          </div>
        {:else if nav.settingsSection === "java"}
          <div class="group-block">
            <p class="label">Memoria por defecto para instancias nuevas</p>
            <div class="mem">
              <strong class="display">{formatMemory(Number(draft.minecraft.max_ram_mb) || 0)}</strong>
              <span class="hint">máximo</span>
            </div>
            <input type="range" min="1024" max="16384" step="512" bind:value={draft.minecraft.max_ram_mb} aria-label="RAM máxima" />
            <div class="row">
              <label class="field"><span>RAM mínima (MB)</span><input type="number" step="256" bind:value={draft.minecraft.min_ram_mb} /></label>
              <label class="field"><span>RAM máxima (MB)</span><input type="number" step="256" bind:value={draft.minecraft.max_ram_mb} /></label>
            </div>
            <label class="field"><span>Argumentos JVM por defecto</span><textarea class="mono" rows="2" bind:value={draft.jvm}></textarea></label>
          </div>
          <div class="group-block">
            <div class="java-head">
              <p class="label">Java detectado en este equipo</p>
              <button class="btn sm" onclick={() => app.detectJava()} disabled={app.javaLoading}>
                <Icon name="refresh" size={14} /> {app.javaLoading ? "Buscando…" : "Detectar de nuevo"}
              </button>
            </div>
            {#if app.javaLoading && app.java.length === 0}
              <div class="skeleton" style="height: 52px"></div>
            {:else if app.java.length === 0}
              <div class="alert info"><Icon name="info" size={16} /><span>No se encontró Java en el sistema. No pasa nada: Obsidia descarga de Adoptium el JRE que pida cada versión. En Arch/CachyOS también puedes instalarlo con <span class="mono">sudo pacman -S jre21-openjdk</span>.</span></div>
            {/if}
            <div class="java-list">
              {#each app.java as j (j.path)}
                <div class="java">
                  <span class="java-major display">{j.major}</span>
                  <div class="java-text">
                    <strong>Java {j.major}</strong>
                    <span class="mono hint">{j.version} · {j.path}</span>
                  </div>
                </div>
              {/each}
            </div>
            <p class="hint">Cada instancia elige su Java en sus ajustes. En automático se usa el que pide su versión de Minecraft y, si falta, se descarga solo.</p>
          </div>
        {:else if nav.settingsSection === "sharing"}
          <div class="group-block">
            <label class="field">
              <span>URL base del servicio de manifiestos</span>
              <input class="mono" bind:value={draft.sharingUrl} placeholder="https://ejemplo.com/manifests" />
            </label>
            <p class="hint">
              Un código como <span class="mono">OBS-K7P2X9</span> se busca en
              <span class="mono">{"<esta URL>"}/OBS-K7P2X9.json</span>. Obsidia no aloja nada: necesitas tu propio
              sitio (o uno de confianza) con esos JSON. Más detalles en el README.
            </p>
          </div>
        {:else}
          <About />
        {/if}
      {/key}

      {#if error}<div class="alert"><Icon name="alert" size={16} /><span>{error}</span></div>{/if}
    </div>

    <div class="savebar" class:visible={dirty}>
      <Icon name="info" size={16} />
      <span>Tienes cambios sin guardar</span>
      <span class="spacer"></span>
      <button class="btn ghost" onclick={discard} disabled={saving}>Descartar</button>
      <button class="btn primary" onclick={save} disabled={saving}>{saving ? "Guardando…" : "Guardar cambios"}</button>
    </div>
  </div>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr);
    height: 100%;
  }
  .subnav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 22px 12px;
    border-right: 1px solid var(--border);
    background: rgb(var(--surface) / 0.55);
    overflow-y: auto;
  }
  .subnav h1 {
    font-size: 20px;
    padding: 0 10px 8px;
  }
  .group {
    padding: 14px 10px 6px;
    font-size: 10px;
    font-weight: 650;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--muted);
  }
  .subnav button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border-radius: 8px;
    border: none;
    background: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 550;
    transition: background var(--ease), color var(--ease);
  }
  .subnav button:hover {
    background: rgb(255 255 255 / 0.05);
    color: var(--text);
  }
  .subnav button.active {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
    color: var(--text);
  }
  .subnav button.active :global(svg) {
    color: var(--accent-hover);
  }
  .content {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .inner {
    flex: 1;
    overflow-y: auto;
    padding: 26px 32px 90px;
  }
  .inner > :global(*) {
    max-width: 920px;
  }
  .head {
    margin-bottom: 18px;
  }
  .title {
    font-size: 24px;
    font-weight: 700;
    color: var(--text);
    text-transform: none;
    letter-spacing: -0.02em;
    margin: 0;
  }
  .intro {
    margin: -8px 0 16px;
  }
  .group-block {
    padding: 4px 0 18px;
    margin-bottom: 18px;
    border-bottom: 1px solid var(--border);
  }
  .label {
    font-size: 13px;
    font-weight: 600;
    margin-bottom: 10px;
  }
  .alert {
    margin: 12px 0;
  }
  .advanced {
    margin-top: 18px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
  }
  .advanced summary {
    cursor: pointer;
    font-size: 13px;
    color: var(--muted);
    font-weight: 600;
  }
  .advanced[open] summary {
    margin-bottom: 14px;
  }

  .account-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 18px;
    border-radius: var(--radius-lg);
    background:
      radial-gradient(circle at 0% 0%, color-mix(in srgb, var(--accent) 18%, transparent), transparent 60%),
      rgb(var(--surface-2) / 1);
    border: 1px solid var(--border);
  }
  .ac-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .ac-text strong {
    font-size: 20px;
  }
  .signin-options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .opt {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 18px;
    border-radius: var(--radius);
    border: 1px solid var(--border-strong);
    background: rgb(255 255 255 / 0.03);
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }
  .opt:hover:not(:disabled) {
    background: rgb(255 255 255 / 0.06);
  }
  .opt strong {
    font-size: 15px;
  }
  .opt :global(svg) {
    color: var(--accent-hover);
  }
  .primary-opt {
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .update-hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    margin-bottom: 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: rgb(var(--surface) / var(--surface-alpha));
  }
  .update-hero .label {
    margin-bottom: 0;
    color: var(--muted);
    font-weight: 500;
  }
  .ver {
    font-size: 30px;
  }
  .update-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 14px;
  }
  .chip {
    height: 30px;
    padding: 0 12px;
    border-radius: 8px;
    border: 1px solid var(--border-strong);
    background: none;
    color: var(--text-2);
    cursor: pointer;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .chip.active {
    border-color: var(--accent);
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .mem {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
  }
  .mem strong {
    font-size: 30px;
  }
  .mem + input {
    margin-bottom: 16px;
  }
  .java-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  .java-head .label {
    margin: 0;
  }
  .java-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 10px;
  }
  .java {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: rgb(255 255 255 / 0.03);
    border: 1px solid var(--border);
  }
  .java-major {
    width: 40px;
    height: 40px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: rgb(255 138 76 / 0.12);
    color: var(--ember);
    font-weight: 700;
    font-size: 16px;
  }
  .java-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .java-text .mono {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .savebar {
    position: absolute;
    left: 24px;
    right: 24px;
    bottom: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px 10px 16px;
    border-radius: var(--radius);
    background: rgb(var(--surface-3) / 1);
    border: 1px solid var(--border-strong);
    box-shadow: 0 18px 40px -16px rgb(0 0 0 / 0.8);
    font-size: 13px;
    opacity: 0;
    transform: translateY(12px);
    pointer-events: none;
    transition: opacity var(--ease), transform var(--ease);
  }
  .savebar.visible {
    opacity: 1;
    transform: none;
    pointer-events: auto;
  }
  .savebar :global(svg) {
    color: var(--accent-hover);
  }
  .spacer {
    flex: 1;
  }
</style>
