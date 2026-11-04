import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Tauri recompila src-tauri por su cuenta; si Vite vigila target/, en Windows
    // falla con EBUSY al tocar los .exe que cargo está escribiendo.
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: { target: "es2022", outDir: "dist" },
});
