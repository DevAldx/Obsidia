/** Navegación global: cualquier componente puede llevar a otra sección. */

export type Page = "home" | "instances" | "discover" | "settings";
export type InstanceTab = "overview" | "mods" | "settings" | "logs";
export type SettingsSection =
  | "general"
  | "account"
  | "minecraft"
  | "java"
  | "appearance"
  | "sharing"
  | "updates"
  | "about";

class NavState {
  page = $state<Page>("home");
  /** Instancia abierta en el panel de detalle de «Instancias». */
  instanceId = $state<string | null>(null);
  instanceTab = $state<InstanceTab>("overview");
  settingsSection = $state<SettingsSection>("general");
  /** Petición pendiente de abrir el asistente de creación al entrar en Instancias. */
  createRequested = $state(false);

  go(page: Page) {
    this.page = page;
  }

  openInstance(id: string, tab: InstanceTab = "overview") {
    this.instanceId = id;
    this.instanceTab = tab;
    this.page = "instances";
  }

  openSettings(section: SettingsSection) {
    this.settingsSection = section;
    this.page = "settings";
  }

  requestCreate() {
    this.createRequested = true;
    this.page = "instances";
  }
}

export const nav = new NavState();
