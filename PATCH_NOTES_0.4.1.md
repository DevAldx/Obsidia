# Obsidia 0.4.1: notas de la versión

Fecha: 2026-11-01

## Cambios

Compartir una instancia ya no requiere preparar nada a mano. El botón
**Compartir** de cada instancia genera un código que lleva dentro toda su
configuración, y **Importar por código** la instala tal cual.

### Compartir por código

- **Código corto `OBS-XXXXXX`, con todo incluido.** Lleva nombre, descripción,
  versión de Minecraft, loader, RAM, imagen de la instancia, mods, resource
  packs, shaders y, si se marca la casilla, la carpeta `config`.
- **Servidor oficial por defecto.** Los códigos se guardan en
  `https://obsidia-share.obsidia-share.workers.dev` y no hace falta configurar
  nada. En Configuración › Compartir se puede poner un servidor propio.
- **Modrinth primero.** Los archivos que están en Modrinth no se suben: el
  código apunta a `cdn.modrinth.com`. Solo se suben los mods privados, la
  imagen y las configs.
- **Código largo `OBS1-…` sin servidor.** Opción alternativa: un texto que
  incluye la configuración y los archivos de Modrinth. La imagen va como
  enlace https y se avisa de los archivos que no están en Modrinth.
- **Plantilla de manifiesto.** La exportación anterior sigue disponible en el
  apartado «Avanzado» del cuadro Compartir.

### Importar

- Un único campo acepta códigos cortos y largos; ya no hay que configurar un
  servicio de manifiestos.
- La vista previa muestra la RAM del pack.
- **Aviso de mods sin verificar:** si un pack trae mods que no vienen de
  Modrinth, se avisa antes de instalar que los subió quien lo compartió.

### Seguridad

- Para subir hace falta una cuenta de Microsoft con Minecraft. El servidor la
  comprueba con Mojang igual que un servidor en online-mode (`join` /
  `hasJoined`): el token de Minecraft nunca se envía al servidor de Obsidia.
- Cada archivo se guarda por su SHA-1, que el servidor vuelve a calcular al
  recibirlo. Al instalar se verifican SHA-1 y tamaño de todo, como siempre.
- Límites por cuenta y día en el servidor; 25 MB como máximo por archivo.

### Servidor

- Nueva carpeta `server/`: Cloudflare Worker con Workers KV (sin tarjeta) o
  R2. Instrucciones en `server/README.md`.

## Compatibilidad

- Los códigos `OBS-` y `NBL-` de servidores de manifiestos propios siguen
  funcionando si se configura su URL en Configuración › Compartir.
- Sin cambios en instancias, ajustes ni sesión.
