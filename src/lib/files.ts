/** Lectura de archivos elegidos por el usuario para enviarlos a Rust en base64. */

export const BACKGROUND_TYPES = ["image/png", "image/jpeg", "image/webp", "image/gif"];
export const BACKGROUND_ACCEPT = "image/png,image/jpeg,image/webp,image/gif,.png,.jpg,.jpeg,.webp,.gif";
export const MAX_BACKGROUND_BYTES = 15 * 1024 * 1024;

export function mimeOf(file: File): string {
  if (file.type) return file.type;
  const ext = file.name.split(".").pop()?.toLowerCase();
  if (ext === "png") return "image/png";
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (ext === "webp") return "image/webp";
  if (ext === "gif") return "image/gif";
  return "";
}

export function readAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result ?? ""));
    reader.onerror = () => reject(reader.error ?? new Error("no se pudo leer el archivo"));
    reader.readAsDataURL(file);
  });
}

export const base64Of = (dataUrl: string): string => dataUrl.slice(dataUrl.indexOf(",") + 1);

export async function readAsBase64(file: File): Promise<string> {
  return base64Of(await readAsDataUrl(file));
}
