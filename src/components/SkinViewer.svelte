<script lang="ts" module>
  /** Carga una imagen (URL de textures.minecraft.net o data:) para dibujarla. */
  export function loadImage(src: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error("no se pudo cargar la textura"));
      img.src = src;
    });
  }
</script>

<script lang="ts">
  import type { SkinVariant } from "../lib/types";

  /**
   * Render 2D (frente o espalda) de una skin de Minecraft sobre canvas, sin
   * motores 3D: suficiente para previsualizar y muy barato en equipos modestos.
   * Unidades en «píxeles de skin»: el personaje mide 16×32.
   */
  let {
    skin,
    variant = "classic",
    cape = null,
    view = "front",
    height = 240,
  }: {
    skin: string | null;
    variant?: SkinVariant;
    cape?: string | null;
    view?: "front" | "back";
    height?: number;
  } = $props();

  const SCALE = 8; // resolución interna: 16×32 → 128×256
  let canvas = $state<HTMLCanvasElement | null>(null);
  let failed = $state(false);

  type Rect = [sx: number, sy: number, sw: number, sh: number, dx: number, dy: number];

  function partsFor(v: "front" | "back", aw: number, legacy: boolean): { base: (Rect | [Rect, true])[]; overlay: Rect[] } {
    // Para skins antiguas (64×32) el brazo/pierna izquierdos son el derecho reflejado.
    if (v === "front") {
      const base: (Rect | [Rect, true])[] = [
        [8, 8, 8, 8, 4, 0],
        [20, 20, 8, 12, 4, 8],
        [44, 20, aw, 12, 4 - aw, 8],
        legacy ? [[44, 20, aw, 12, 12, 8], true] : [36, 52, aw, 12, 12, 8],
        [4, 20, 4, 12, 4, 20],
        legacy ? [[4, 20, 4, 12, 8, 20], true] : [20, 52, 4, 12, 8, 20],
      ];
      const overlay: Rect[] = [[40, 8, 8, 8, 4, 0]];
      if (!legacy) {
        overlay.push(
          [20, 36, 8, 12, 4, 8],
          [44, 36, aw, 12, 4 - aw, 8],
          [52, 52, aw, 12, 12, 8],
          [4, 36, 4, 12, 4, 20],
          [4, 52, 4, 12, 8, 20],
        );
      }
      return { base, overlay };
    }
    const base: (Rect | [Rect, true])[] = [
      [24, 8, 8, 8, 4, 0],
      [32, 20, 8, 12, 4, 8],
      [48 + aw, 20, aw, 12, 12, 8],
      legacy ? [[48 + aw, 20, aw, 12, 4 - aw, 8], true] : [40 + aw, 52, aw, 12, 4 - aw, 8],
      [12, 20, 4, 12, 8, 20],
      legacy ? [[12, 20, 4, 12, 4, 20], true] : [28, 52, 4, 12, 4, 20],
    ];
    const overlay: Rect[] = [[56, 8, 8, 8, 4, 0]];
    if (!legacy) {
      overlay.push(
        [32, 36, 8, 12, 4, 8],
        [48 + aw, 36, aw, 12, 12, 8],
        [56 + aw, 52, aw, 12, 4 - aw, 8],
        [12, 36, 4, 12, 8, 20],
        [12, 52, 4, 12, 4, 20],
      );
    }
    return { base, overlay };
  }

  function draw(ctx: CanvasRenderingContext2D, img: HTMLImageElement, r: Rect, mirror = false) {
    const k = img.width / 64; // tolera texturas HD
    const [sx, sy, sw, sh, dx, dy] = r;
    if (mirror) {
      ctx.save();
      ctx.scale(-1, 1);
      ctx.drawImage(img, sx * k, sy * k, sw * k, sh * k, -(dx + sw) * SCALE, dy * SCALE, sw * SCALE, sh * SCALE);
      ctx.restore();
    } else {
      ctx.drawImage(img, sx * k, sy * k, sw * k, sh * k, dx * SCALE, dy * SCALE, sw * SCALE, sh * SCALE);
    }
  }

  $effect(() => {
    const target = canvas;
    const src = skin;
    const capeSrc = cape;
    const v = view;
    const aw = variant === "slim" ? 3 : 4;
    if (!target) return;
    let alive = true;
    failed = false;
    void (async () => {
      const ctx = target.getContext("2d");
      if (!ctx) return;
      ctx.clearRect(0, 0, target.width, target.height);
      if (!src) return;
      try {
        const [img, capeImg] = await Promise.all([
          loadImage(src),
          v === "back" && capeSrc ? loadImage(capeSrc).catch(() => null) : Promise.resolve(null),
        ]);
        if (!alive) return;
        ctx.imageSmoothingEnabled = false;
        ctx.clearRect(0, 0, target.width, target.height);
        const legacy = img.height * 2 === img.width;
        const { base, overlay } = partsFor(v, aw, legacy);
        for (const part of base) {
          if (part.length === 2) draw(ctx, img, part[0], true);
          else draw(ctx, img, part);
        }
        for (const part of overlay) draw(ctx, img, part);
        if (capeImg) {
          const k = capeImg.width / 64;
          ctx.drawImage(capeImg, 1 * k, 1 * k, 10 * k, 16 * k, 3 * SCALE, 8 * SCALE, 10 * SCALE, 16 * SCALE);
        }
      } catch {
        if (alive) failed = true;
      }
    })();
    return () => {
      alive = false;
    };
  });
</script>

<div class="viewer" style:height={`${height}px`} style:width={`${height / 2}px`}>
  <canvas bind:this={canvas} width={16 * SCALE} height={32 * SCALE} aria-label={view === "front" ? "Skin de frente" : "Skin de espaldas"}></canvas>
  {#if failed}<span class="fail">Sin vista previa</span>{/if}
</div>

<style>
  .viewer {
    position: relative;
    flex: none;
  }
  canvas {
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
    filter: drop-shadow(0 10px 18px rgb(0 0 0 / 0.55));
  }
  .fail {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 12px;
    color: var(--muted);
    text-align: center;
  }
</style>
