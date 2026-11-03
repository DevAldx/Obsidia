# Obsidia 0.5.0: notas de la versión

Fecha: 2026-11-02

## Cambios

Mods y modpacks dentro del launcher: se buscan, se instalan y se publican sin
descargar archivos a mano.

### Mods

- **Añadir mods** en la pestaña Mods de cada instancia. Busca en Modrinth solo
  lo que funciona con la versión de Minecraft y el loader de la instancia, con
  orden por relevancia, descargas, seguidores o fecha.
- **Dependencias automáticas.** Al instalar un mod se instalan también sus
  dependencias obligatorias (por ejemplo, Sodium Extra trae Sodium). Los mods
  que ya tienes aparecen como «Instalado», aunque estén desactivados.
- Todo se descarga de `cdn.modrinth.com` verificando SHA-1 y tamaño.

### Modpacks

- Nueva sección **Modpacks** en Descubrir con los modpacks de Fabric de
  Modrinth: portada, autor y descargas.
- Al instalar se elige la versión y se ve cuántos mods trae y cuánto pesa. Se
  crea una instancia nueva con sus mods y su configuración, y se descargan
  también Minecraft y Java, así que «Jugar» arranca a la primera.
- **Instalar .mrpack** desde el PC.
- Los modpacks de Forge, NeoForge y Quilt avisan de que todavía no se admiten.

### Comunidad

- **Catálogo comunitario** con packs y mods que publican los propios jugadores.
  Aparece en Descubrir › Modpacks › Comunidad y en Añadir mods › Comunidad.
- **Publicar un pack:** en Compartir, después de generar el código corto, botón
  **Publicar en la comunidad**.
- **Publicar un mod:** botón de subir junto a cada mod de una instancia con
  Fabric. Los mods que ya están en Modrinth no se suben (se instalan desde
  allí) y hay que confirmar que el mod es tuyo o tienes permiso.
- Los mods de la comunidad se marcan «Sin verificar» y piden confirmación antes
  de instalarse. Cada jugador puede borrar sus publicaciones.

### Instancias de Nebula

- Las instancias que venían de Nebula Launcher (`NBL-XXXXXX`) pasan solas a
  `OBS-XXXXXX` al abrir Obsidia, con el mismo sufijo, sus mods, mundos e
  imagen. Si una era la instancia principal, lo sigue siendo. Los códigos
  `NBL-` para importar siguen funcionando.

### Arreglos

- **Compartir fallaba con algunas instancias** si su carpeta `config` tenía
  archivos vacíos (por ejemplo, las copias `.bak` de JEI): el servidor no
  acepta subidas de 0 bytes. Ahora esos archivos no se suben.
- **Desplegables en Linux:** el texto de las listas desplegables salía cortado.
  Ahora todas usan el estilo de Obsidia.
- Las búsquedas en Modrinth se reintentan si la conexión corta la respuesta, y
  si «Cargar más» falla, el error aparece junto al botón con «Reintentar».

### Servidor

- `server/` añade `GET`/`POST /api/catalog` y `DELETE /api/catalog/<id>`.
  Hasta 2.000 publicaciones en total y 50 por cuenta. La variable
  `ADMIN_UUIDS` permite moderar. Hay que volver a desplegar el Worker.

## Compatibilidad

- Los códigos `OBS-` y `OBS1-` funcionan igual que en la 0.4.2.
- Sin cambios en instancias, ajustes ni sesión.
