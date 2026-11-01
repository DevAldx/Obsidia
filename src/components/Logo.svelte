<script lang="ts">
  /** Marca de Obsidia en línea (sin petición de imagen). Si el usuario eligió
   *  un icono propio en Apariencia, se muestra ese. */
  let { size = 30, custom = null }: { size?: number; custom?: string | null } = $props();
</script>

{#if custom}
  <img class="custom" src={custom} alt="" width={size} height={size} />
{:else}
  <svg width={size} height={size} viewBox="220 180 584 664" aria-hidden="true">
    <defs>
      <linearGradient id="ob-top" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#7a52cc" />
        <stop offset="1" stop-color="#3a2468" />
      </linearGradient>
    </defs>
    <polygon points="512,196 786,354 512,512 238,354" fill="url(#ob-top)" />
    <polygon points="238,354 512,512 512,828 238,670" fill="#1b1230" />
    <polygon points="512,512 786,354 786,670 512,828" fill="#2c1b4c" />
    <polygon points="512,268 660,354 512,440 364,354" fill="#9a74e6" opacity="0.55" />
    <path d="M300 430 L344 455 L344 505 L388 530 L388 600 L432 625" fill="none" stroke="var(--accent-hover)" stroke-width="26" stroke-linecap="square" />
    <path d="M556 700 L600 675 L600 625 L644 600 L644 545 L688 520 L688 470 L736 443" fill="none" stroke="var(--accent)" stroke-width="26" stroke-linecap="square" />
    <polygon points="512,196 786,354 786,670 512,828 238,670 238,354" fill="none" stroke="#d9c2ff" stroke-opacity="0.35" stroke-width="10" stroke-linejoin="round" />
  </svg>
{/if}

<style>
  svg, .custom { display: block; flex: none; }
  .custom { border-radius: 8px; object-fit: cover; }
</style>
