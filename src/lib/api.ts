import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  Instance,
  JavaRuntime,
  InstanceList,
  ModEntry,
  UpdateCheck,
  BackgroundView,
  ExportedCode,
  UploadedPack,
  InstanceManifest,
  NewInstance,
  Settings,
  StartupProgress,
  UpdateInstance,
  VersionSummary,
  Account,
  SignInCodeEvent,
  NewsFeed,
  SkinProfile,
  SkinVariant,
  LibrarySkin,
} from "./types";

/** Los comandos de Rust fallan con `{ message }`; lo normalizamos a Error. */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const msg =
      typeof e === "object" && e !== null && "message" in e
        ? String((e as { message: unknown }).message)
        : String(e);
    throw new Error(msg);
  }
}

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  getStartupProgress: () => call<StartupProgress>("get_startup_progress"),
  splashReady: () => call<void>("splash_ready"),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  listInstances: () => call<InstanceList>("list_instances"),
  createInstance: (input: NewInstance) => call<Instance>("create_instance", { input }),
  updateInstance: (id: string, patch: UpdateInstance) =>
    call<Instance>("update_instance", { id, patch }),
  deleteInstance: (id: string) => call<Settings>("delete_instance", { id }),
  getAccount: () => call<Account | null>("get_account"),
  getSignInCode: () => call<SignInCodeEvent | null>("get_sign_in_code"),
  beginSignIn: () => call<SignInCodeEvent>("begin_sign_in"),
  beginLegacySignIn: () => call<SignInCodeEvent>("begin_legacy_sign_in"),
  signInOffline: (username: string) => call<Account>("sign_in_offline", { username }),
  cancelSignIn: () => call<void>("cancel_sign_in"),
  signOut: () => call<void>("sign_out"),
  listJava: () => call<JavaRuntime[]>("list_java_runtimes"),
  listVersions: () => call<VersionSummary[]>("list_minecraft_versions"),
  play: (id: string) => call<void>("play_instance", { id }),
  stop: (id: string) => call<void>("stop_instance", { id }),
  gameStates: () => call<Record<string, string>>("game_states"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  openHttpsUrl: (url: string) => call<void>("open_https_url", { url }),
  checkForUpdates: () => call<UpdateCheck>("check_for_updates"),
  getInstanceBackground: (instanceId: string) => call<BackgroundView | null>("get_instance_background", { instanceId }),
  setInstanceBackground: (instanceId: string, mime: string, dataBase64: string) =>
    call<void>("set_instance_background", { instanceId, mime, dataBase64 }),
  clearInstanceBackground: (instanceId: string) => call<void>("clear_instance_background", { instanceId }),
  getLauncherBackground: () => call<BackgroundView | null>("get_launcher_background"),
  setLauncherBackground: (mime: string, dataBase64: string) =>
    call<void>("set_launcher_background", { mime, dataBase64 }),
  clearLauncherBackground: () => call<void>("clear_launcher_background"),
  getLauncherIcon: () => call<BackgroundView | null>("get_launcher_icon"),
  setLauncherIcon: (mime: string, dataBase64: string) =>
    call<void>("set_launcher_icon", { mime, dataBase64 }),
  clearLauncherIcon: () => call<void>("clear_launcher_icon"),
  fetchShareManifest: (code: string) => call<InstanceManifest>("fetch_share_manifest", { code }),
  importFromCode: (manifest: InstanceManifest) =>
    call<void>("import_from_code", { code: manifest.code, manifest }),
  exportManifestTemplate: (instanceId: string) => call<string>("export_manifest_template", { instanceId }),
  exportShareCode: (instanceId: string, description: string | null, backgroundUrl: string | null) =>
    call<ExportedCode>("export_share_code", { instanceId, description, backgroundUrl }),
  uploadSharePack: (instanceId: string, description: string | null, includeConfig: boolean) =>
    call<UploadedPack>("upload_share_pack", { instanceId, description, includeConfig }),
  listMods: (instanceId: string) => call<ModEntry[]>("list_mods", { instanceId }),
  setModEnabled: (instanceId: string, fileName: string, enabled: boolean) =>
    call<string>("set_mod_enabled", { instanceId, fileName, enabled }),
  deleteMod: (instanceId: string, fileName: string) =>
    call<void>("delete_mod", { instanceId, fileName }),
  importMods: (instanceId: string, files: { name: string; data_base64: string }[]) =>
    call<string[]>("import_mods", { instanceId, files }),
  openInstanceFolder: (instanceId: string, sub: "" | "mods" | "config") =>
    call<void>("open_instance_folder", { instanceId, sub }),
  selectInstance: (id: string | null) => call<Settings>("select_instance", { id }),
  checkAndPrepareUpdate: () => call<ObsidiaUpdateStatus>("check_and_prepare_update"),
  installUpdate: () => call<void>("install_update"),
  fetchNews: (force = false) => call<NewsFeed>("fetch_news", { force }),
  getSkinProfile: () => call<SkinProfile>("get_skin_profile"),
  uploadSkin: (name: string, variant: SkinVariant, dataBase64: string) =>
    call<SkinProfile>("upload_skin", { name, variant, dataBase64 }),
  applyLibrarySkin: (id: string) => call<SkinProfile>("apply_library_skin", { id }),
  resetSkin: () => call<SkinProfile>("reset_skin"),
  setCape: (capeId: string | null) => call<SkinProfile>("set_cape", { capeId }),
  listSkinLibrary: () => call<LibrarySkin[]>("list_skin_library"),
  deleteLibrarySkin: (id: string) => call<void>("delete_library_skin", { id }),
};

/** "-Xmx2G -XX:+UseG1GC" → ["-Xmx2G", "-XX:+UseG1GC"] */
export const splitArgs = (s: string): string[] => s.split(/\s+/).filter(Boolean);
export const joinArgs = (a: string[]): string => a.join(" ");

export type ObsidiaUpdateInfo = {
  version: string;
  current_version: string;
  notes?: string | null;
};

export type ObsidiaUpdateStatus = {
  available: boolean;
  update: ObsidiaUpdateInfo | null;
};

/** Servidor oficial para compartir. Debe coincidir con `DEFAULT_SHARE_URL` de `obsidia-core`. */
export const DEFAULT_SHARE_URL = "https://obsidia-share.obsidia-share.workers.dev";
