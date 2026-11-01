# Servidor para compartir instancias

Un Cloudflare Worker. Guarda los archivos que no están en
Modrinth (mods privados, la imagen de la instancia y la carpeta `config`) y
genera códigos cortos `OBS-XXXXXX`. Lo que sí está en Modrinth no se sube: el
manifiesto apunta directamente a `cdn.modrinth.com`.

Funciona con dos tipos de almacenamiento:

| | Workers KV (por defecto) | R2 |
|---|---|---|
| ¿Pide tarjeta? | **No** | Sí, aunque sea gratis |
| Espacio gratis | 1 GB | 10 GB |
| Máximo por archivo | 25 MB | 50 MB |
| Escrituras gratis al día | 1.000 (unas 500 subidas) | de sobra |

## Desplegarlo (Workers KV, sin tarjeta)

Necesitas una cuenta de Cloudflare (gratis) y Node.js.

```bash
cd server
npx wrangler login
npx wrangler kv namespace create FILES      # copia el "id" que imprime
```

Abre `wrangler.toml` y pon ese id en `id = "PEGA_AQUI_EL_ID"`. Luego:

```bash
npx wrangler secret put SECRET              # pega lo que salga de: openssl rand -base64 48
npx wrangler deploy
```

`wrangler deploy` imprime la URL, del tipo
`https://obsidia-share.<tu-usuario>.workers.dev`. Pégala en Obsidia, en
**Configuración › Compartir**. Quien vaya a importar tus códigos necesita la
misma URL.

**Para usar R2 en su lugar:** activa R2 en el panel, crea el bucket con
`npx wrangler r2 bucket create obsidia-share`, descomenta el bloque
`[[r2_buckets]]` de `wrangler.toml`, sube `MAX_FILE_MB` a 50 y vuelve a
desplegar. Si los dos están configurados, se usa R2.

## Cómo funciona

| Ruta | Quién | Para qué |
|---|---|---|
| `GET /OBS-XXXXXX.json` | cualquiera | el manifiesto del pack (el formato de siempre) |
| `GET /files/<sha1>` | cualquiera | un archivo subido |
| `POST /api/challenge`, `POST /api/login` | el launcher | iniciar sesión |
| `PUT /api/files/<sha1>` | con sesión | subir un archivo |
| `POST /api/packs` | con sesión | publicar un pack y obtener el código |
| `DELETE /api/packs/<código>` | el dueño | borrar un pack |

- **Inicio de sesión:** igual que un servidor de Minecraft en online-mode. El
  launcher se "une" en Mojang con un `serverId` que da el Worker, y el Worker lo
  confirma con `hasJoined`. El token de Minecraft nunca llega al servidor, y
  solo pueden subir cuentas que tengan Minecraft.
- **Integridad:** cada archivo se guarda por su SHA-1, que el Worker vuelve a
  calcular al recibirlo. Un pack solo se publica si todos sus archivos están en
  Modrinth o ya subidos con ese hash y tamaño.
- **Límites** (en `wrangler.toml`): 25 MB por archivo con KV (50 MB con R2),
  y 500 MB y 30 packs por cuenta y día.

## Lo que debes saber

- **Eres responsable de lo que se sube.** Cualquiera con una cuenta de Minecraft
  y tu URL puede subir archivos. Un `.jar` es código ejecutable; el launcher
  avisa al importar cuando un mod no viene de Modrinth.
- Los archivos subidos no se borran solos. Se borran desde el panel de
  Cloudflare (Workers KV o R2, clave `files/<sha1>`).
- Con KV, un archivo recién subido puede tardar unos segundos en verse desde
  otras regiones. Si un amigo importa justo al instante y falla, que lo
  intente otra vez en un minuto.

## Pruebas

```bash
npm test        # prueba con R2 y con KV
```

Simulan el almacenamiento y Mojang en memoria, así que no necesitan Cloudflare.
