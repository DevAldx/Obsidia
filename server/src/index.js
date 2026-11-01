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
// Inicio de sesión: igual que un servidor de Minecraft en online-mode. El
// launcher llama a `session/minecraft/join` de Mojang con su token y el
// `serverId` que le damos; aquí solo preguntamos a Mojang `hasJoined`. El token
// de Minecraft nunca llega a este servidor.

const MODRINTH_CDN = "cdn.modrinth.com";
const MOJANG_HAS_JOINED = "https://sessionserver.mojang.com/session/minecraft/hasJoined";
const CODE_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const CODE_RE = /^OBS-[A-Z0-9]{6}$/;
const SHA1_RE = /^[0-9a-f]{40}$/;
const USERNAME_RE = /^[A-Za-z0-9_]{1,16}$/;
const MAX_MANIFEST_BYTES = 1024 * 1024;
const MAX_MANIFEST_FILES = 512;
const CHALLENGE_TTL_S = 120;
const TOKEN_TTL_S = 60 * 60;
const FILE_LISTS = ["mods", "resourcepacks", "shaderpacks", "config_files"];

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

async function login(request, env) {
  const body = await request.json().catch(() => null);
  const username = body?.username;
  if (typeof username !== "string" || !USERNAME_RE.test(username)) {
    throw new HttpError(400, "nombre de usuario no válido");
  }
  const [serverId] = await verify(env, "challenge", body?.ticket, 1);
  const q = new URLSearchParams({ username, serverId });
  const resp = await fetch(`${MOJANG_HAS_JOINED}?${q}`);
  if (resp.status !== 200) {
    throw new HttpError(401, "Mojang no ha confirmado tu cuenta; inicia sesión de nuevo en el launcher");
  }
  const profile = await resp.json();
  if (typeof profile?.id !== "string" || !/^[0-9a-f]{32}$/.test(profile.id)) {
    throw new HttpError(502, "respuesta inesperada de Mojang");
  }
  const token = await sign(env, "token", [profile.id], TOKEN_TTL_S);
  return json({ token, uuid: profile.id, name: profile.name });
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
  return new Response(null, { status: 204 });
}
