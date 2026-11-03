# Obsidia

Launcher de Minecraft ligero para Linux y Windows. Rust + Tauri 2 + Svelte 5 +
TypeScript. No incluye Minecraft: descarga los archivos oficiales desde Mojang
con tu propia cuenta.

## Estado

- **Cuenta local**: nombre de jugador, UUID determinista
  (`MD5("OfflinePlayer:<nombre>")`) y aviso de `online-mode=false`.
- **Cuenta Microsoft**: flujo oficial con código de dispositivo y tu propia app
  de Azure, o flujo heredado sin Azure. La contraseña siempre se escribe en una
  página de Microsoft; Obsidia solo recibe tokens.
- **Java automático**: si falta la versión exacta, descarga un JRE de Eclipse
  Temurin desde Adoptium, verifica su SHA-256 y lo extrae en
  `data/java/temurin-<mayor>`.
- **Instancias**: carpetas independientes con `instance.json`, mods, configs,
  resourcepacks, shaderpacks, saves y logs. Minecraft, librerías, assets y Java
  son compartidos entre instancias.
- **Importar por código**: vista previa con nombre, versión, loader, archivos y
  Java mínimo; descarga verificada de mods, resourcepacks, shaders, configs y
  fondo remoto.
- **Exportar**: genera la plantilla del manifiesto con los datos de la instancia
  y sus mods activos. Las URLs de descarga las pones tú.
- **Interfaz**: barra superior propia (Inicio, Instancias, Descubrir, Ajustes),
  escenario a pantalla completa con botón de jugar y selector de instancia,
  biblioteca maestro-detalle con pestañas (resumen, mods, ajustes, registros) y
  ajustes por secciones.
- **Noticias**: noticias y notas de versión oficiales de Minecraft: Java Edition
  (`launchercontent.mojang.com`), con caché de 30 min y copia en disco para
  mostrarlas sin conexión.
- **Skin y capa**: cambia la skin (PNG 64×64/64×32, clásico o slim) y la capa
  de tu cuenta de Microsoft desde el launcher, con biblioteca local de skins.
- **Personalización**: icono propio, tema, acento, transparencia, animaciones y
  fondo PNG/JPG/WEBP/GIF animado (Ajustes → Apariencia).
- **Ventana de bienvenida**: animación con progreso real, comprobación de
  versión, detección de Java y lectura de instancias.
- **Windows**: rutas, `java.exe`, classpath, iconos `.ico` e instalador NSIS.
- **Pendiente**: cancelar una importación a mitad.

## Requisitos

- **Linux**: Rust ≥ 1.77, Node ≥ 20 y dependencias de Tauri 2. En Arch:
  `sudo pacman -S webkit2gtk-4.1 base-devel librsvg`.
- **Windows**: Rust con toolchain MSVC, Node ≥ 20, Visual Studio Build Tools con
  «Desarrollo para el escritorio con C++» y WebView2. Windows 10/11; Windows
  8/8.1 no es compatible con Tauri 2/WebView2 moderno.

## Desarrollo

```bash
npm install
cargo test -p obsidia-core      # tests del núcleo
npm run check                  # svelte-check + TypeScript
npm run tauri dev              # ejecutar la app
./scripts/build-appimage.sh    # AppImage Linux firmado
npm run tauri build -- --bundles nsis  # instalador Windows
```

Estructura principal:

```
crates/obsidia-core/  Lógica pura sin Tauri (config, instancias, descargas,
                     Minecraft, Java, auth, mods, share, updater)
installer/           Instalador del AppImage para Linux
scripts/             bump-version.sh, build-appimage.sh, build-windows.ps1
src-tauri/           Comandos IPC (workspace propio exigido por Tauri)
src/                 Frontend Svelte
```

## Ejecutar el AppImage

Descarga `Obsidia_0.4.0_amd64.AppImage` desde
<https://github.com/DevAldx/Obsidia/releases>:

```bash
chmod +x "Obsidia_0.4.0_amd64.AppImage"
./"Obsidia_0.4.0_amd64.AppImage"
```

No necesita instalación. Para una entrada en el menú, compila el instalador de
`installer/`; ese instalador necesita el AppImage **y** su `.AppImage.sig`, y
verifica la firma minisign antes de copiar nada.

El AppImage usa WebKitGTK 4.1 y GTK 3 del sistema si están disponibles; si no,
usa la copia empaquetada. Para forzarla:
`OBSIDIA_BUNDLED_WEBKIT=1 ./"Obsidia_0.4.0_amd64.AppImage"`.

## Compilar y publicar

```bash
npm install
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/obsidia.key)"
./scripts/build-appimage.sh

cd installer && npm install
npm run tauri build -- --bundles appimage
# deja el instalador junto al AppImage y su .sig
```

El AppImage queda en `src-tauri/target/release/bundle/appimage/` y el instalador
NSIS en `src-tauri/target/release/bundle/nsis/`.

Para publicar: sube la versión con `scripts/bump-version.sh` (`patch`, `minor`,
`major` o `set X.Y.Z`), escribe `PATCH_NOTES_<versión>.md`, haz commit y empuja
la etiqueta `vX.Y.Z`. El workflow `.github/workflows/release.yml` compila y
firma el AppImage y el instalador de Windows, genera `latest.json` con
`create_latest_json.sh` y crea la release con esas notas.

## Actualizaciones y versiones

Obsidia usa `tauri-plugin-updater` con firmas minisign. Al arrancar consulta
`https://github.com/DevAldx/Obsidia/releases/latest/download/latest.json`;
si hay una versión nueva, muestra las novedades, descarga el artefacto, verifica
la firma e instala. También se puede comprobar a mano en **Ajustes →
Actualizaciones**.

El formato es **MAYOR.MENOR.PARCHE**: parche para arreglos y cambios pequeños,
menor para funciones nuevas y mayor para rediseños o cambios incompatibles. Los
cambios pequeños se agrupan en un solo parche. Guía completa: `VERSIONADO.md`.

## Cuentas

### Local

Escribe un nombre de 3-16 caracteres (letras, números y `_`). Se guarda en
`account.json` sin contraseña ni token. Funciona en servidores con
`online-mode=false`; un servidor oficial rechazará la cuenta local.

### Microsoft

Hay dos vías; en ambas la contraseña se escribe en una página real de Microsoft:

- **Oficial (Azure)**: registra una app en <https://portal.azure.com> con
  «Personal Microsoft accounts only», activa «Allow public client flows», pega
  el Client ID en **Ajustes → Cuenta** y envía la app a revisión en
  <https://aka.ms/mce-reviewappid>. Sin esa aprobación aparece *Invalid app
  registration*.
- **Heredada (sin Azure, no oficial)**: usa el Client ID público
  `00000000402b5328`. No requiere Azure ni tarjeta, pero Microsoft podría
  cambiarlo o bloquearlo. Sirve para uso personal o grupos pequeños, no como
  base de un servicio público.

Sin cuenta configurada, Obsidia funciona en **modo demo**.

## Java

Cada versión de Minecraft declara su Java. En modo automático Obsidia busca un
runtime instalado con la versión mayor exacta; si no existe, descarga Temurin
desde Adoptium, verifica el SHA-256 y lo extrae en `data/java/temurin-<mayor>`.
Para versiones anteriores a 1.13 que piden Java 8 se exige exactamente Java 8.
También puedes elegir un Java concreto en los ajustes de la instancia.

## Instancias compartibles

El botón **Compartir** de cada instancia genera uno de dos tipos de código:

- **Código corto `OBS-XXXXXX`** (con servidor): sube a tu servidor los mods
  que no están en Modrinth, la imagen y la carpeta `config`, y publica el
  manifiesto. El servidor está en [`server/`](server/README.md) (Cloudflare
  Worker + R2).
- **Código largo `OBS1-…`** (sin servidor): lleva dentro nombre, versión,
  loader, RAM, URL de la imagen y los archivos de Modrinth.

También puedes alojar los manifiestos tú mismo:

Un código `OBS-XXXXXX` se busca en `<URL configurada>/OBS-XXXXXX.json`. Sirve
cualquier sitio que publique archivos estáticos por HTTPS (GitHub raw, S3, tu
servidor). Solo se permiten los hosts que el manifiesto declara y cada archivo
se descarga con el mismo `Downloader` verificado que usa Mojang.

```json
{
  "code": "OBS-K7P2X9",
  "name": "SkyFactory liviano",
  "description": "Pack de supervivencia con Fabric",
  "background_url": "https://.../fondo.jpg",
  "minecraft_version": "1.21.1",
  "loader": "fabric",
  "loader_version": "0.15.11",
  "java": { "min_major": 21 },
  "memory": { "min_mb": 2048, "max_mb": 4096 },
  "mods": [
    { "file_name": "sodium.jar", "url": "https://.../sodium.jar", "sha1": "...", "size": 123456 }
  ],
  "resourcepacks": [], "shaderpacks": [], "config_files": []
}
```

Cada instancia vive en `instances/OBS-XXXXXX/`. Al copiarla sola a otro PC,
Obsidia conserva mods, configs y saves y descarga al jugar lo que falte de
Minecraft, librerías, assets y Java. Si copias también `versions/`,
`libraries/`, `assets/` y `java/`, evitas esa descarga.

## Seguridad

- Solo HTTPS y hosts conocidos; los redirects no pueden salir de la lista.
- Descargas verificadas por tamaño y SHA-1 (los JRE además por SHA-256) antes de
  renombrarse; nada queda a medias.
- Natives y JRE extraídos con protección contra zip-slip y entradas gigantes.
- Escritura atómica de JSON, `join_safe` contra path traversal y rechazo de
  symlinks en rutas críticas.
- El refresh token vive en el almacén seguro del sistema (Secret Service,
  KWallet o Credential Manager) y los tokens se censuran en los logs.
- El frontend solo puede elegir un Java de la lista detectada y abrir el enlace
  de inicio de sesión de Microsoft; no tiene fs, shell ni http.

## Windows

El instalador NSIS se puede compilar en Windows con
`scripts/build-windows.ps1`, desde Linux con `cargo-xwin` + NSIS 3.11, o probar
un `.exe` ya publicado con `.github/workflows/test-windows-installer.yml`. Se
probó en Windows Server 2025 (GitHub Actions): instalación, versión, ventana
principal y app viva. Falta un Windows 10/11 de escritorio y el flujo completo
de juego.

**Wine/Bottles no es compatible**: el instalador intenta instalar WebView2 y
Wine no lo soporta. En Linux hay que usar el AppImage nativo.

## Aviso legal

Obsidia es un proyecto independiente. **No está afiliado, asociado,
autorizado, respaldado ni patrocinado por Mojang Studios, Microsoft Corporation
ni sus filiales.** Minecraft es una marca registrada de Mojang Studios. Obsidia
no incluye ni distribuye Minecraft.

El copyright, las marcas y los créditos completos están en **Ajustes → Acerca
de**.

## Actualizar desde versiones anteriores (0.3.x)

Hasta la 0.3.x el launcher usaba otro nombre interno. Obsidia lo migra todo solo
la primera vez que arranca:

| Qué | Antes | Ahora | Cómo se migra |
|---|---|---|---|
| Datos e instancias | `~/.local/share/nebula` | `~/.local/share/obsidia` | Se **mueven** (`rename`, sin copiar ni borrar) si la carpeta nueva no existe. Las rutas absolutas guardadas (Java manual de cada instancia, `data_dir`) se corrigen. Si el movimiento falla, se sigue usando la carpeta antigua. |
| Ajustes | `~/.config/nebula` | `~/.config/obsidia` | Igual que los datos. |
| Sesión de Microsoft | llavero `dev.nebula.launcher` | llavero `dev.obsidia.launcher` | El refresh token se copia al servicio nuevo y se borra el antiguo: no hay que volver a iniciar sesión. |
| IDs y códigos | `NBL-XXXXXX` | `OBS-XXXXXX` | Las instancias nuevas usan `OBS-`; las existentes y los códigos ya compartidos con `NBL-` siguen siendo válidos. |
| Identificador de la app | `dev.nebula.launcher` | `dev.obsidia.launcher` | El WebView empieza con un perfil limpio (no guardaba nada importante). |
| Repositorio | `DevAldx/nebula-launcher` | `DevAldx/Obsidia` | GitHub redirige la URL antigua, así que las versiones instaladas siguen recibiendo actualizaciones. **No crees otro repositorio con el nombre antiguo** o esa redirección se rompería. |

En Windows las carpetas son `%APPDATA%\obsidia\obsidia\{config,data}`. El
instalador NSIS usa el nombre de producto para la carpeta del programa: al
actualizar desde 0.3.x, Obsidia se instala como app nueva y la antigua sigue en
«Aplicaciones instaladas»; se puede desinstalar sin perder datos. El instalador
de Linux retira solo el AppImage y la entrada de menú antiguos.

Si después de migrar vuelves a abrir una versión 0.3.x, no verá tus instancias
(ya están en la carpeta nueva).

## Documentación

- `VERSIONADO.md`: formato y proceso de versiones.
- `PATCH_NOTES_<versión>.md`: cambios de la versión actual. Las notas de las
  versiones anteriores están en las releases de GitHub.
- `server/README.md`: servidor para compartir y catálogo comunitario.
