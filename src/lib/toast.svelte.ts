/** Avisos breves en la esquina inferior: confirmaciones y errores de acciones. */

export type ToastKind = "success" | "error" | "info";
export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
}

const MAX_TOASTS = 4;

class ToastState {
  list = $state<Toast[]>([]);
  private next = 1;

  push(kind: ToastKind, text: string, ms = kind === "error" ? 6500 : 3500) {
    const id = this.next++;
    this.list = [...this.list.slice(-(MAX_TOASTS - 1)), { id, kind, text }];
    setTimeout(() => this.dismiss(id), ms);
  }

  success(text: string) {
    this.push("success", text);
  }

  info(text: string) {
    this.push("info", text);
  }

  /** Acepta cualquier cosa lanzada por `api` y la muestra como error. */
  error(e: unknown, prefix?: string) {
    const msg = e instanceof Error ? e.message : String(e);
    this.push("error", prefix ? `${prefix}: ${msg}` : msg);
  }

  dismiss(id: number) {
    this.list = this.list.filter((t) => t.id !== id);
  }
}

export const toasts = new ToastState();
