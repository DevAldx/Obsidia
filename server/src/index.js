// Servidor para compartir instancias de Obsidia (Cloudflare Worker + R2).
//
// Rutas públicas (las usa quien importa):
//   GET  /OBS-XXXXXX.json        manifiesto del pack (el formato que ya lee el launcher)
//   GET  /files/<sha1>           archivo subido (mod, imagen, config…)
//
// Rutas con sesión (las usa quien comparte):
//   POST /api/challenge          -> { serverId, ticket }
//   POST /api/login              { username, ticket } -> { token, uuid, name }
//   PUT  /api/files/<sha1>       cuerpo = bytes del archivo
//   POST /api/packs              cuerpo = manifiesto JSON -> { code }
//   DELETE /api/packs/<code>     solo el dueño
//
// Catálogo comunitario (packs y mods que publican los jugadores):
//   GET    /api/catalog          -> { items: [...] } (público)
//   POST   /api/catalog          { kind: "pack", code } | { kind: "mod", name, file, ... }
//   DELETE /api/catalog/<id>     el dueño o un admin (ADMIN_UUIDS)
//
// Inicio de sesión: con el certificado de jugador de Mojang (el par de claves
// que Minecraft usa para firmar el chat). Mojang firma la clave pública de cada
// cuenta; el launcher firma nuestro desafío con la clave privada. Aquí se
// comprueban las dos firmas sin llamar a Mojang (Mojang bloquea las peticiones
// que salen de Cloudflare). El token de Minecraft nunca llega a este servidor.

const MODRINTH_CDN = "cdn.modrinth.com";
// Claves con las que Mojang firma los certificados de jugador
// (`playerCertificateKeys` de https://api.minecraftservices.com/publickeys).
// Si Mojang las cambia, se pueden sobrescribir con la variable MOJANG_PLAYER_KEYS
// (claves base64 separadas por comas) sin tocar el código.
const MOJANG_PLAYER_KEYS = [
  "MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEAylB4B6m5lz7jwrcFz6Fd/fnfUhcvlxsTSn5kIK/2aGG1C3kMy4VjhwlxF6BFUSnfxhNswPjh3ZitkBxEAFY25uzkJFRwHwVA9mdwjashXILtR6OqdLXXFVyUPIURLOSWqGNBtb08EN5fMnG8iFLgEJIBMxs9BvF3s3/FhuHyPKiVTZmXY0WY4ZyYqvoKR+XjaTRPPvBsDa4WI2u1zxXMeHlodT3lnCzVvyOYBLXL6CJgByuOxccJ8hnXfF9yY4F0aeL080Jz/3+EBNG8RO4ByhtBf4Ny8NQ6stWsjfeUIvH7bU/4zCYcYOq4WrInXHqS8qruDmIl7P5XXGcabuzQstPf/h2CRAUpP/PlHXcMlvewjmGU6MfDK+lifScNYwjPxRo4nKTGFZf/0aqHCh/EAsQyLKrOIYRE0lDG3bzBh8ogIMLAugsAfBb6M3mqCqKaTMAf/VAjh5FFJnjS+7bE+bZEV0qwax1CEoPPJL1fIQjOS8zj086gjpGRCtSy9+bTPTfTR/SJ+VUB5G2IeCItkNHpJX2ygojFZ9n5Fnj7R9ZnOM+L8nyIjPu3aePvtcrXlyLhH/hvOfIOjPxOlqW+O5QwSFP4OEcyLAUgDdUgyW36Z5mB285uKW/ighzZsOTevVUG2QwDItObIV6i8RCxFbN2oDHyPaO5j1tTaBNyVt8CAwEAAQ==",
  "MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEAt4t9NPuu7cktclnaH7eZj0omkLcJHeLz5MKsyJEntHZ0INtuBjSSul3Pp3pBeJN8k3ADdcdBLUN90bcAi7WsQqTx3Ft363q3W7TbM8j2iTEdp/0uVspoRt/DP1tkaWFs/w2WwUv9jbVoBUzfUc4pSTIxRwdjmqjZQfvjwKNDbOx3IhP2H0WXodbISejPi1wBZqNW4m1rnZAXp/EpUguxA8mobCa4vUCBkyFDyXdl69/wUSJHyCPmgcMJ364OlAhIqtwVPShBZObvrK/f0BYk6ShJD3N7TFDatSYsIIdcTKRknaIm91s+EsMrdB9U4Yw+ZJ/pyCB4S3vk8zfDCnb0DWIxYH3/EMzaxl77djmTmMzi/JDITup5z3jfWtRZmrAhU2/+W5IO5hEpo3/bCS9PXIY5xb41Lmp2ZO8dXKtyD66Chchy0W129n8vPl2GIruOdrxsjZAHnneyAb9jm0uaGaphwnEnuecX/qgHY6ZMtayvLLsPst8PO6R1vufMy8WqjK+j7LnC1krL7CPDg0NEhyQTmw5l+NCNjSlvB1juM9V4PARg0bYCOkGXm7ydRCjSSH8CJXZpwnd5cBB5WKAX3KPzutRgMi/LFwNSMZzFuUyXaYOZPpD259yqph1LmGqegEdDriACVU+dVEONFMm8eIuBofe7ljmsAFKW9BINwK0CAwEAAQ==",
];
const LOGIN_PREFIX = "obsidia-share-login\n";
const CODE_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const CODE_RE = /^OBS-[A-Z0-9]{6}$/;
const SHA1_RE = /^[0-9a-f]{40}$/;
const UUID_RE = /^[0-9a-f]{32}$/;
const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_MANIFEST_FILES = 512;
const CHALLENGE_TTL_S = 120;
const TOKEN_TTL_S = 60 * 60;
const FILE_LISTS = ["mods", "resourcepacks", "shaderpacks", "config_files"];
const CATALOG_KEY = "catalog/index.json";
const CATALOG_MAX_ITEMS = 2000;
const CATALOG_MAX_PER_USER = 50;
const CATALOG_ID_RE = /^[a-z0-9]{10}$/;
const MC_NAME_RE = /^[A-Za-z0-9_]{1,16}$/;
const MC_VERSION_RE = /^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$/;

const enc = new TextEncoder();

export default {
  async fetch(request, env) {
    try {
      // R2 si está configurado (pide tarjeta, aunque sea gratis); si no, Workers KV.
      if (!env.BUCKET && env.FILES) env = { ...env, BUCKET: kvBucket(env.FILES), USING_KV: true };
      if (!env.BUCKET) throw new HttpError(500, "falta configurar el almacenamiento (FILES o BUCKET en wrangler.toml)");
      return await route(request, env);
    } catch (e) {
      if (e instanceof HttpError) return json({ error: e.message }, e.status);
      console.error(e);
      return json({ error: "error interno del servidor" }, 500);
    }
  },
};

class HttpError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

function json(body, status = 200, headers = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", ...headers },
  });
}

/** Workers KV: un valor no puede pasar de 25 MiB. */
const KV_MAX_VALUE = 25 * 1024 * 1024;

/**
 * Adapta Workers KV a la parte de la API de R2 que usa este archivo
 * (`get`, `head`, `put`, `delete`). KV no tiene `head`, así que lee el valor.
 */
function kvBucket(kv) {
  const wrap = (value, metadata) =>
    value === null
      ? null
      : {
          size: metadata?.size ?? value.byteLength,
          customMetadata: metadata?.custom ?? {},
          body: value,
          json: async () => JSON.parse(new TextDecoder().decode(value)),
        };
  const get = async (key) => {
    const { value, metadata } = await kv.getWithMetadata(key, "arrayBuffer");
    return wrap(value, metadata);
  };
  return {
    get,
    head: get,
    async put(key, value, opts = {}) {
      const bytes = typeof value === "string" ? enc.encode(value) : new Uint8Array(value);
      await kv.put(key, bytes, { metadata: { size: bytes.byteLength, custom: opts.customMetadata ?? {} } });
    },
    async delete(key) {
      await kv.delete(key);
    },
  };
}

function limits(env) {
  const mb = (v, d) => (Number(v) > 0 ? Number(v) : d) * 1024 * 1024;
  const maxFile = mb(env.MAX_FILE_MB, env.USING_KV ? 25 : 50);
  return {
    maxFile: env.USING_KV ? Math.min(maxFile, KV_MAX_VALUE) : maxFile,
    dailyUpload: mb(env.DAILY_UPLOAD_MB, 1024),
    dailyPacks: Number(env.DAILY_PACKS) > 0 ? Number(env.DAILY_PACKS) : 30,
  };
}

async function route(request, env) {
  if (!env.SECRET || env.SECRET.length < 32) {
    throw new HttpError(500, "falta configurar SECRET (wrangler secret put SECRET)");
  }
  const url = new URL(request.url);
  const path = url.pathname;
  const method = request.method;

  if (path === "/" && method === "GET") {
    return json({ service: "obsidia-share", version: 1 });
  }

  let m = path.match(/^\/(OBS-[A-Z0-9]{6})\.json$/);
  if (m && (method === "GET" || method === "HEAD")) {
    const obj = await env.BUCKET.get(`packs/${m[1]}.json`);
    if (!obj) return json({ error: "no existe ningún pack con ese código" }, 404);
    return new Response(method === "HEAD" ? null : obj.body, {
      headers: { "content-type": "application/json; charset=utf-8", "cache-control": "public, max-age=60" },
    });
  }

  m = path.match(/^\/files\/([0-9a-f]{40})$/);
  if (m && (method === "GET" || method === "HEAD")) {
    const key = `files/${m[1]}`;
    const obj = method === "HEAD" ? await env.BUCKET.head(key) : await env.BUCKET.get(key);
    if (!obj) return new Response(null, { status: 404 });
    return new Response(method === "HEAD" ? null : obj.body, {
      headers: {
        "content-type": "application/octet-stream",
        "content-length": String(obj.size),
        // El contenido de una ruta es su propio hash: nunca cambia.
        "cache-control": "public, max-age=31536000, immutable",
      },
    });
  }

  if (path === "/api/challenge" && method === "POST") return challenge(env);
  if (path === "/api/login" && method === "POST") return login(request, env);

  m = path.match(/^\/api\/files\/([0-9a-f]{40})$/);
  if (m && method === "PUT") return uploadFile(request, env, m[1]);

  if (path === "/api/packs" && method === "POST") return createPack(request, env, url.origin);

  m = path.match(/^\/api\/packs\/(OBS-[A-Z0-9]{6})$/);
  if (m && method === "DELETE") return deletePack(request, env, m[1]);

  if (path === "/api/catalog" && method === "GET") return listCatalog(env);
  if (path === "/api/catalog" && method === "POST") return publishToCatalog(request, env, url.origin);
  m = path.match(/^\/api\/catalog\/([a-z0-9]{10})$/);
  if (m && method === "DELETE") return deleteFromCatalog(request, env, m[1]);

  return json({ error: "ruta no encontrada" }, 404);
}

// ───────────────────────────── Firma (HMAC) ─────────────────────────────

const b64url = (bytes) =>
  btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");

const hex = (bytes) => [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, "0")).join("");

async function hmac(env, data) {
  const key = await crypto.subtle.importKey("raw", enc.encode(env.SECRET), { name: "HMAC", hash: "SHA-256" }, false, [
    "sign",
  ]);
  return b64url(await crypto.subtle.sign("HMAC", key, enc.encode(data)));
}

function safeEqual(a, b) {
  if (a.length !== b.length) return false;
  let diff = 0;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return diff === 0;
}

/** Firma `parts` (sin puntos) con caducidad. Devuelve `parts...exp.firma`. */
async function sign(env, kind, parts, ttl) {
  const exp = Math.floor(Date.now() / 1000) + ttl;
  const body = [kind, ...parts, exp].join(".");
  return `${[...parts, exp].join(".")}.${await hmac(env, body)}`;
}

/** Comprueba firma y caducidad; devuelve las partes firmadas o lanza 401. */
async function verify(env, kind, value, nParts) {
  const pieces = String(value ?? "").split(".");
  if (pieces.length !== nParts + 2) throw new HttpError(401, "sesión no válida");
  const sig = pieces.pop();
  const exp = Number(pieces[nParts]);
  const expected = await hmac(env, [kind, ...pieces].join("."));
  if (!safeEqual(sig, expected)) throw new HttpError(401, "sesión no válida");
  if (!(exp > Date.now() / 1000)) throw new HttpError(401, "la sesión ha caducado; vuelve a intentarlo");
  return pieces.slice(0, nParts);
}

// ───────────────────────────── Sesión ─────────────────────────────

async function challenge(env) {
  const serverId = hex(crypto.getRandomValues(new Uint8Array(16)));
  return json({ serverId, ticket: await sign(env, "challenge", [serverId], CHALLENGE_TTL_S) });
}

const b64decode = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0));
const pemToDer = (pem) => b64decode(String(pem).replace(/-----[^-]+-----/g, "").replace(/\s+/g, ""));

/** Milisegundos de un instante ISO de Mojang (trae nanosegundos; Java los trunca a ms). */
function isoMillis(iso) {
  const m = String(iso).match(/^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d+))?Z$/);
  if (!m) return NaN;
  return Date.parse(`${m[1]}Z`) + Number((m[2] ?? "0").padEnd(3, "0").slice(0, 3));
}

async function rsaVerify(spki, hash, signature, data) {
  try {
    const key = await crypto.subtle.importKey("spki", spki, { name: "RSASSA-PKCS1-v1_5", hash }, false, ["verify"]);
    return await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, signature, data);
  } catch {
    return false;
  }
}

/** ¿Mojang firmó esta clave pública para este UUID y esta caducidad? (publicKeySignatureV2) */
async function mojangSignedKey(env, uuid, expiresAtMs, spki, signature) {
  const payload = new Uint8Array(24 + spki.length);
  for (let i = 0; i < 16; i++) payload[i] = parseInt(uuid.slice(i * 2, i * 2 + 2), 16);
  new DataView(payload.buffer).setBigInt64(16, BigInt(expiresAtMs));
  payload.set(spki, 24);
  const keys = env.MOJANG_PLAYER_KEYS ? String(env.MOJANG_PLAYER_KEYS).split(",") : MOJANG_PLAYER_KEYS;
  for (const k of keys) {
    if (await rsaVerify(b64decode(k.trim()), "SHA-1", signature, payload)) return true;
  }
  return false;
}

async function login(request, env) {
  const body = await request.json().catch(() => null);
  const uuid = String(body?.uuid ?? "").replace(/-/g, "").toLowerCase();
  if (!UUID_RE.test(uuid)) throw new HttpError(400, "UUID no válido");
  const ticket = body?.ticket;
  await verify(env, "challenge", ticket, 1);

  const expiresAt = isoMillis(body?.expiresAt);
  if (!(expiresAt > Date.now())) throw new HttpError(401, "el certificado de tu cuenta ha caducado; vuelve a iniciar sesión");
  let spki, mojangSig, playerSig;
  try {
    spki = pemToDer(body.publicKey);
    mojangSig = b64decode(body.publicKeySignatureV2);
    playerSig = b64decode(body.signature);
  } catch {
    throw new HttpError(400, "certificado ilegible");
  }
  if (!(await mojangSignedKey(env, uuid, expiresAt, spki, mojangSig))) {
    throw new HttpError(401, "Mojang no ha firmado el certificado de esta cuenta");
  }
  if (!(await rsaVerify(spki, "SHA-256", playerSig, enc.encode(LOGIN_PREFIX + ticket)))) {
    throw new HttpError(401, "la firma de la cuenta no es válida");
  }
  const token = await sign(env, "token", [uuid], TOKEN_TTL_S);
  return json({ token, uuid });
}

async function requireUser(request, env) {
  const auth = request.headers.get("authorization") ?? "";
  if (!auth.startsWith("Bearer ")) throw new HttpError(401, "falta iniciar sesión");
  const [uuid] = await verify(env, "token", auth.slice(7), 1);
  return uuid;
}

// ───────────────────────────── Cuotas ─────────────────────────────

async function readQuota(env, uuid) {
  const day = new Date().toISOString().slice(0, 10);
  const key = `quota/${uuid}/${day}`;
  const obj = await env.BUCKET.get(key);
  const data = obj ? await obj.json() : { bytes: 0, packs: 0 };
  return { key, data };
}

const writeQuota = (env, q) => env.BUCKET.put(q.key, JSON.stringify(q.data));

// ───────────────────────────── Archivos ─────────────────────────────

async function uploadFile(request, env, sha1) {
  const uuid = await requireUser(request, env);
  const lim = limits(env);
  const declared = Number(request.headers.get("content-length"));
  if (!(declared > 0)) throw new HttpError(411, "falta el tamaño del archivo");
  if (declared > lim.maxFile) throw new HttpError(413, "el archivo supera el tamaño máximo permitido");

  const key = `files/${sha1}`;
  if (await env.BUCKET.head(key)) return json({ url: `/files/${sha1}`, existed: true });

  const quota = await readQuota(env, uuid);
  if (quota.data.bytes + declared > lim.dailyUpload) {
    throw new HttpError(429, "has alcanzado el límite de subida de hoy");
  }

  const bytes = await request.arrayBuffer();
  if (bytes.byteLength !== declared || bytes.byteLength > lim.maxFile) {
    throw new HttpError(400, "el tamaño recibido no coincide");
  }
  const got = hex(await crypto.subtle.digest("SHA-1", bytes));
  if (got !== sha1) throw new HttpError(400, "el SHA-1 del archivo no coincide");

  await env.BUCKET.put(key, bytes, { customMetadata: { uploader: uuid } });
  quota.data.bytes += bytes.byteLength;
  await writeQuota(env, quota);
  return json({ url: `/files/${sha1}`, existed: false }, 201);
}

// ───────────────────────────── Packs ─────────────────────────────

function validFileName(name, list) {
  if (typeof name !== "string" || !name || name.length > 255) return false;
  const parts = name.split("/");
  if (parts.some((p) => !p || p === "." || p === ".." || p.includes("\\") || p.includes("\0"))) return false;
  if (list !== "config_files" && parts.length !== 1) return false;
  const lower = name.toLowerCase();
  if (list === "mods") return lower.endsWith(".jar");
  if (list === "resourcepacks" || list === "shaderpacks") return lower.endsWith(".zip");
  return true;
}

/** Una URL es válida si apunta a Modrinth o a un archivo nuestro que existe con ese hash y tamaño. */
async function checkFileUrl(env, origin, rawUrl, sha1, size) {
  let u;
  try {
    u = new URL(rawUrl);
  } catch {
    return false;
  }
  if (u.protocol !== "https:") return false;
  if (u.hostname === MODRINTH_CDN) return true;
  if (u.origin !== origin || u.pathname !== `/files/${sha1}` || u.search) return false;
  const obj = await env.BUCKET.head(`files/${sha1}`);
  return obj !== null && (size === undefined || obj.size === size);
}

async function createPack(request, env, origin) {
  const uuid = await requireUser(request, env);
  const lim = limits(env);
  const text = await request.text();
  if (text.length > MAX_MANIFEST_BYTES) throw new HttpError(413, "el manifiesto es demasiado grande");
  let manifest;
  try {
    manifest = JSON.parse(text);
  } catch {
    throw new HttpError(400, "manifiesto inválido");
  }
  const bad = (msg) => {
    throw new HttpError(400, msg);
  };
  if (typeof manifest !== "object" || manifest === null) bad("manifiesto inválido");
  if (typeof manifest.name !== "string" || !manifest.name.trim() || manifest.name.length > 48) bad("nombre inválido");
  if (typeof manifest.minecraft_version !== "string" || !/^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$/.test(manifest.minecraft_version)) {
    bad("versión de Minecraft inválida");
  }
  if (!["vanilla", "fabric"].includes(manifest.loader)) bad("loader inválido");
  if (manifest.description != null && (typeof manifest.description !== "string" || manifest.description.length > 300)) {
    bad("descripción inválida");
  }

  let count = 0;
  for (const list of FILE_LISTS) {
    const files = manifest[list] ?? [];
    if (!Array.isArray(files)) bad(`${list} inválido`);
    for (const f of files) {
      if (++count > MAX_MANIFEST_FILES) bad("demasiados archivos");
      if (!validFileName(f?.file_name, list)) bad(`${list}: nombre de archivo inválido`);
      if (typeof f.sha1 !== "string" || !SHA1_RE.test(f.sha1)) bad(`${list}/${f.file_name}: SHA-1 inválido`);
      if (!Number.isInteger(f.size) || f.size < 0) bad(`${list}/${f.file_name}: tamaño inválido`);
      if (!(await checkFileUrl(env, origin, f.url, f.sha1, f.size))) {
        bad(`${list}/${f.file_name}: el archivo no se ha subido o su URL no está permitida`);
      }
    }
  }
  if (manifest.background_url != null) {
    const m = String(manifest.background_url).match(/\/files\/([0-9a-f]{40})$/);
    if (!m || !(await checkFileUrl(env, origin, manifest.background_url, m[1]))) {
      bad("la imagen no se ha subido a este servidor");
    }
  }

  const quota = await readQuota(env, uuid);
  if (quota.data.packs >= lim.dailyPacks) throw new HttpError(429, "has alcanzado el límite de packs de hoy");

  let code = null;
  for (let i = 0; i < 16 && !code; i++) {
    const rnd = crypto.getRandomValues(new Uint8Array(6));
    const candidate = `OBS-${[...rnd].map((b) => CODE_ALPHABET[b % CODE_ALPHABET.length]).join("")}`;
    if (!(await env.BUCKET.head(`packs/${candidate}.json`))) code = candidate;
  }
  if (!code) throw new HttpError(503, "no se pudo generar un código; inténtalo de nuevo");

  manifest.code = code;
  await env.BUCKET.put(`packs/${code}.json`, JSON.stringify(manifest), {
    httpMetadata: { contentType: "application/json" },
    customMetadata: { owner: uuid, created: new Date().toISOString() },
  });
  quota.data.packs += 1;
  await writeQuota(env, quota);
  return json({ code }, 201);
}

async function deletePack(request, env, code) {
  const uuid = await requireUser(request, env);
  if (!CODE_RE.test(code)) throw new HttpError(400, "código inválido");
  const key = `packs/${code}.json`;
  const obj = await env.BUCKET.head(key);
  if (!obj) throw new HttpError(404, "no existe ningún pack con ese código");
  if (obj.customMetadata?.owner !== uuid) throw new HttpError(403, "este pack no es tuyo");
  await env.BUCKET.delete(key);
  // Un pack borrado tampoco debe seguir en el catálogo.
  const catalog = await readCatalog(env);
  const before = catalog.items.length;
  catalog.items = catalog.items.filter((e) => e.code !== code);
  if (catalog.items.length !== before) await writeCatalog(env, catalog);
  return new Response(null, { status: 204 });
}

// ───────────────────────────── Catálogo comunitario ─────────────────────────────
//
// Un único índice JSON (`catalog/index.json`), del más nuevo al más viejo. Vale
// igual para KV y R2 sin depender de `list`. Con mucho tráfico simultáneo dos
// publicaciones podrían pisarse; para un servidor de comunidad pequeño basta.

async function readCatalog(env) {
  const obj = await env.BUCKET.get(CATALOG_KEY);
  if (!obj) return { items: [] };
  const data = await obj.json().catch(() => null);
  return Array.isArray(data?.items) ? data : { items: [] };
}

const writeCatalog = (env, catalog) => env.BUCKET.put(CATALOG_KEY, JSON.stringify(catalog));

async function listCatalog(env) {
  const catalog = await readCatalog(env);
  return json(catalog, 200, { "cache-control": "public, max-age=30" });
}

function isAdmin(env, uuid) {
  return String(env.ADMIN_UUIDS ?? "")
    .split(",")
    .map((u) => u.trim().replace(/-/g, "").toLowerCase())
    .includes(uuid);
}

function cleanText(value, max) {
  if (value == null) return null;
  if (typeof value !== "string") throw new HttpError(400, "texto inválido");
  // eslint-disable-next-line no-control-regex
  const t = value.replace(/[\u0000-\u001f\u007f]/g, " ").trim();
  if (t.length > max) throw new HttpError(400, `el texto no puede pasar de ${max} caracteres`);
  return t || null;
}

function newCatalogId() {
  const alphabet = "abcdefghijklmnopqrstuvwxyz0123456789";
  return [...crypto.getRandomValues(new Uint8Array(10))].map((b) => alphabet[b % alphabet.length]).join("");
}

async function publishToCatalog(request, env, origin) {
  const uuid = await requireUser(request, env);
  const body = await request.json().catch(() => null);
  if (typeof body !== "object" || body === null) throw new HttpError(400, "petición inválida");
  const author = String(body.author ?? "");
  if (!MC_NAME_RE.test(author)) throw new HttpError(400, "nombre de autor inválido");
  const description = cleanText(body.description, 300);

  const catalog = await readCatalog(env);
  if (catalog.items.length >= CATALOG_MAX_ITEMS) throw new HttpError(507, "el catálogo está lleno");
  if (catalog.items.filter((e) => e.owner === uuid).length >= CATALOG_MAX_PER_USER) {
    throw new HttpError(429, `ya tienes ${CATALOG_MAX_PER_USER} publicaciones; borra alguna antes`);
  }
  const quota = await readQuota(env, uuid);
  if (quota.data.packs >= limits(env).dailyPacks) throw new HttpError(429, "has alcanzado el límite de publicaciones de hoy");

  let entry;
  if (body.kind === "pack") {
    const code = String(body.code ?? "");
    if (!CODE_RE.test(code)) throw new HttpError(400, "código inválido");
    const obj = await env.BUCKET.get(`packs/${code}.json`);
    if (!obj) throw new HttpError(404, "no existe ningún pack con ese código");
    if (obj.customMetadata?.owner !== uuid) throw new HttpError(403, "solo puedes publicar tus propios packs");
    if (catalog.items.some((e) => e.code === code)) throw new HttpError(409, "este pack ya está publicado");
    const manifest = await obj.json();
    entry = {
      kind: "pack",
      name: manifest.name,
      description: description ?? manifest.description ?? null,
      minecraft_version: manifest.minecraft_version,
      loader: manifest.loader,
      loader_version: manifest.loader_version ?? null,
      code,
      image_url: manifest.background_url ?? null,
      mod_count: Array.isArray(manifest.mods) ? manifest.mods.length : 0,
    };
  } else if (body.kind === "mod") {
    const name = cleanText(body.name, 48);
    if (!name) throw new HttpError(400, "falta el nombre del mod");
    if (typeof body.minecraft_version !== "string" || !MC_VERSION_RE.test(body.minecraft_version)) {
      throw new HttpError(400, "versión de Minecraft inválida");
    }
    if (body.loader !== "fabric") throw new HttpError(400, "loader inválido");
    const f = body.file ?? {};
    if (!validFileName(f.file_name, "mods")) throw new HttpError(400, "nombre de archivo inválido");
    if (typeof f.sha1 !== "string" || !SHA1_RE.test(f.sha1)) throw new HttpError(400, "SHA-1 inválido");
    if (!Number.isInteger(f.size) || f.size <= 0) throw new HttpError(400, "tamaño inválido");
    if (!(await checkFileUrl(env, origin, `${origin}/files/${f.sha1}`, f.sha1, f.size))) {
      throw new HttpError(400, "el archivo no se ha subido a este servidor");
    }
    if (catalog.items.some((e) => e.file?.sha1 === f.sha1)) throw new HttpError(409, "este mod ya está publicado");
    entry = {
      kind: "mod",
      name,
      description,
      minecraft_version: body.minecraft_version,
      loader: "fabric",
      file: { file_name: f.file_name, sha1: f.sha1, size: f.size },
    };
  } else {
    throw new HttpError(400, "tipo de publicación inválido");
  }

  entry = { id: newCatalogId(), ...entry, author, owner: uuid, created: new Date().toISOString() };
  catalog.items.unshift(entry);
  await writeCatalog(env, catalog);
  quota.data.packs += 1;
  await writeQuota(env, quota);
  return json(entry, 201);
}

async function deleteFromCatalog(request, env, id) {
  const uuid = await requireUser(request, env);
  if (!CATALOG_ID_RE.test(id)) throw new HttpError(400, "identificador inválido");
  const catalog = await readCatalog(env);
  const entry = catalog.items.find((e) => e.id === id);
  if (!entry) throw new HttpError(404, "esa publicación no existe");
  if (entry.owner !== uuid && !isAdmin(env, uuid)) throw new HttpError(403, "esta publicación no es tuya");
  catalog.items = catalog.items.filter((e) => e.id !== id);
  await writeCatalog(env, catalog);
  return new Response(null, { status: 204 });
}
