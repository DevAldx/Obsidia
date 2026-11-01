<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { toasts } from "../lib/toast.svelte";
  import Icon from "../components/Icon.svelte";
  import Logo from "../components/Logo.svelte";

  async function openRepository() {
    try {
      await api.openHttpsUrl("https://github.com/DevAldx/Obsidia");
    } catch (e) {
      toasts.error(e, "No se pudo abrir el navegador");
    }
  }
</script>

<div class="hero facet">
  <Logo size={64} />
  <div class="hero-copy">
    <strong class="display product">Obsidia</strong>
    <p class="muted">Versión {app.info?.version ?? "—"}</p>
    <p class="hint">
      Launcher independiente y de código abierto para Minecraft: Java Edition, en Linux y Windows.
      Gestiona instancias, mods y cuentas sin depender del launcher oficial.
    </p>
  </div>
  <button class="btn" onclick={openRepository}><Icon name="external" size={15} /> Repositorio</button>
</div>

<dl class="legal">
  <dt>Obsidia</dt>
  <dd>
    © 2026 Obsidia. El código fuente se distribuye bajo la licencia MIT. El nombre, el icono y la
    identidad visual de Obsidia pertenecen a sus autores.
  </dd>
  <dt>Minecraft</dt>
  <dd>
    Minecraft, el logotipo de Minecraft y los nombres relacionados son marcas registradas de Mojang
    Studios. © Mojang Studios. Todos los derechos reservados.
  </dd>
  <dt>Microsoft</dt>
  <dd>
    Microsoft, Xbox, Microsoft Account y los logotipos relacionados son marcas de Microsoft
    Corporation. El inicio de sesión con Microsoft se rige por los términos y la privacidad de
    Microsoft.
  </dd>
  <dt>No afiliación</dt>
  <dd>
    Obsidia es un proyecto independiente. No está afiliado, asociado, autorizado, respaldado ni
    patrocinado por Mojang Studios, Microsoft Corporation ni sus filiales. Obsidia no incluye ni
    distribuye Minecraft: descarga los archivos oficiales desde los servidores de Mojang usando tu
    propia cuenta. Las texturas, sonidos y demás recursos del juego pertenecen a Mojang Studios.
  </dd>
  <dt>Créditos</dt>
  <dd>
    Obsidia usa las APIs públicas de Mojang para versiones, noticias, assets y autenticación, Fabric
    para el soporte de mods y Adoptium para descargar Java cuando hace falta. Gracias a sus
    mantenedores.
  </dd>
</dl>

<style>
  .hero {
    --facet: 16px;
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 22px 24px;
    background:
      radial-gradient(circle at 0% 50%, color-mix(in srgb, var(--accent) 22%, transparent), transparent 55%),
      rgb(var(--surface-2) / 1);
  }
  .hero-copy {
    flex: 1;
    min-width: 0;
  }
  .product {
    font-size: 24px;
    letter-spacing: 0.02em;
  }
  .legal {
    display: grid;
    grid-template-columns: 140px 1fr;
    margin-top: 22px;
  }
  dt,
  dd {
    padding: 14px 0;
    border-top: 1px solid var(--border);
  }
  dt {
    font-weight: 600;
  }
  dd {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.65;
  }
</style>
