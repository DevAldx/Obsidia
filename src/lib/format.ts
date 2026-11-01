import type { Instance, JavaRuntime } from "./types";

export const loaderLabel = (i: Pick<Instance, "loader" | "loader_version">): string =>
  i.loader === "fabric" ? `Fabric ${i.loader_version ?? ""}`.trim() : "Vanilla";

export const loaderName = (i: Pick<Instance, "loader">): string =>
  i.loader === "fabric" ? "Fabric" : "Vanilla";

export function javaLabel(i: Instance, runtimes: JavaRuntime[]): string {
  const java = i.java;
  if (java.kind === "auto") return "Java automático";
  return `Java ${runtimes.find((j) => j.path === java.path)?.major ?? "?"}`;
}

export function formatLastPlayed(ts: number | null): string {
  if (!ts) return "Nunca jugada";
  const days = Math.floor((Date.now() / 1000 - ts) / 86400);
  if (days <= 0) return "Hoy";
  if (days === 1) return "Ayer";
  if (days < 30) return `Hace ${days} días`;
  return new Date(ts * 1000).toLocaleDateString("es-ES", { day: "numeric", month: "short", year: "numeric" });
}

export function formatPlaytime(secs: number): string {
  if (secs < 60) return "0 min";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h} h ${m} min` : `${m} min`;
}

/** `AAAA-MM-DD` → «30 oct 2026». */
export function formatDay(day: string): string {
  const d = new Date(`${day}T12:00:00`);
  if (Number.isNaN(d.getTime())) return day;
  return d.toLocaleDateString("es-ES", { day: "numeric", month: "short", year: "numeric" });
}

export function formatMemory(mb: number): string {
  return mb >= 1024 ? `${(mb / 1024).toFixed(mb % 1024 === 0 ? 0 : 1)} GB` : `${mb} MB`;
}

export const errorText = (e: unknown): string => (e instanceof Error ? e.message : String(e));
