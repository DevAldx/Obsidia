// Pruebas del Worker sin Cloudflare: almacenamiento en memoria y claves de Mojang simuladas.
// Ejecutar con `node --test server/test/`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import worker from "../src/index.js";

const ORIGIN = "https://share.example.com";
const UUID = "0123456789abcdef0123456789abcdef";

function memoryBucket() {
  const store = new Map();
  const obj = (v) => ({
    size: v.bytes.byteLength,
    customMetadata: v.customMetadata,
    body: v.bytes,
    json: async () => JSON.parse(new TextDecoder().decode(v.bytes)),
  });
  return {
    store,
    async head(k) {
      return store.has(k) ? obj(store.get(k)) : null;
    },
    async get(k) {
      return store.has(k) ? obj(store.get(k)) : null;
    },
    async put(k, value, opts = {}) {
      const bytes = typeof value === "string" ? new TextEncoder().encode(value) : new Uint8Array(value);
      store.set(k, { bytes, customMetadata: opts.customMetadata });
    },
    async delete(k) {
      store.delete(k);
    },
  };
}

// Workers KV en memoria (solo lo que usa el adaptador del Worker).
function memoryKv() {
  const store = new Map();
  return {
    store,
    async getWithMetadata(k) {
      const v = store.get(k);
      return v ? { value: v.value.buffer.slice(0), metadata: v.metadata } : { value: null, metadata: null };
    },
    async put(k, value, opts = {}) {
      store.set(k, { value: new Uint8Array(value), metadata: structuredClone(opts.metadata ?? null) });
    },
    async delete(k) {
      store.delete(k);
    },
  };
}

// `STORAGE=kv node --test test/` prueba el modo sin tarjeta (Workers KV).
const USE_KV = process.env.STORAGE === "kv";

function makeEnv() {
  const base = { SECRET: "x".repeat(40), MAX_FILE_MB: "1", MOJANG_PLAYER_KEYS: MOJANG_KEY_B64 };
  return USE_KV ? { ...base, FILES: memoryKv() } : { ...base, BUCKET: memoryBucket() };
}

function setOwner(env, key, owner) {
  if (USE_KV) env.FILES.store.get(key).metadata.custom.owner = owner;
  else env.BUCKET.store.get(key).customMetadata.owner = owner;
}

// El Worker no debe llamar a ninguna red: todo se verifica con firmas.
globalThis.fetch = async (url) => {
  throw new Error(`fetch inesperado: ${url}`);
};

// "Mojang" de prueba: una clave RSA propia que firma los certificados.
const RSA = { name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]) };
const b64 = (bytes) => Buffer.from(bytes).toString("base64");
const mojang = await crypto.subtle.generateKey({ ...RSA, hash: "SHA-1" }, true, ["sign", "verify"]);
const MOJANG_KEY_B64 = b64(await crypto.subtle.exportKey("spki", mojang.publicKey));

/** Certificado de jugador como el de /player/certificates, firmado por `signer`. */
async function playerCertificate({ uuid = UUID, expiresInMs = 3600_000, signer = mojang.privateKey } = {}) {
  const player = await crypto.subtle.generateKey({ ...RSA, hash: "SHA-256" }, true, ["sign", "verify"]);
  const spki = new Uint8Array(await crypto.subtle.exportKey("spki", player.publicKey));
  const expires = new Date(Date.now() + expiresInMs);
  // Mojang manda nanosegundos: comprobamos que se truncan como en Java.
  const expiresAt = expires.toISOString().replace("Z", "123456Z");
  const payload = new Uint8Array(24 + spki.length);
  for (let i = 0; i < 16; i++) payload[i] = parseInt(uuid.slice(i * 2, i * 2 + 2), 16);
  new DataView(payload.buffer).setBigInt64(16, BigInt(expires.getTime()));
  payload.set(spki, 24);
  const sigV2 = await crypto.subtle.sign("RSASSA-PKCS1-v1_5", signer, payload);
  return {
    publicKey: `-----BEGIN RSA PUBLIC KEY-----\n${b64(spki)}\n-----END RSA PUBLIC KEY-----\n`,
    publicKeySignatureV2: b64(sigV2),
    expiresAt,
    sign: async (text) =>
      b64(await crypto.subtle.sign("RSASSA-PKCS1-v1_5", player.privateKey, new TextEncoder().encode(text))),
  };
}

const call = (env, method, path, { body, token, headers = {} } = {}) =>
  worker.fetch(
    new Request(ORIGIN + path, {
      method,
      body,
      headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), ...headers },
    }),
    env,
  );

async function loginAs(env, opts = {}) {
  const ch = await (await call(env, "POST", "/api/challenge")).json();
  const cert = await playerCertificate(opts);
  const message = opts.signTicket ?? ch.ticket;
  return call(env, "POST", "/api/login", {
    body: JSON.stringify({
      uuid: opts.claimUuid ?? UUID.replace(/^(.{8})(.{4})(.{4})(.{4})(.{12})$/, "$1-$2-$3-$4-$5"),
      ticket: ch.ticket,
      publicKey: cert.publicKey,
      publicKeySignatureV2: cert.publicKeySignatureV2,
      expiresAt: cert.expiresAt,
      signature: await cert.sign(`obsidia-share-login\n${message}`),
    }),
  });
}

const sha1 = (bytes) => createHash("sha1").update(bytes).digest("hex");

async function upload(env, token, bytes) {
  return call(env, "PUT", `/api/files/${sha1(bytes)}`, {
    body: bytes,
    token,
    headers: { "content-length": String(bytes.byteLength) },
  });
}

function manifest(fileUrl, hash, size) {
  return {
    code: "",
    name: "Pack",
    minecraft_version: "1.21.1",
    loader: "fabric",
    loader_version: "0.16.5",
    memory: { min_mb: 2048, max_mb: 4096 },
    mods: [{ file_name: "privado.jar", url: fileUrl, sha1: hash, size }],
    resourcepacks: [],
    shaderpacks: [],
    config_files: [],
  };
}

test("login con certificado de jugador: acepta el bueno y rechaza los falsos", async () => {
  const env = makeEnv();
  const ok = await loginAs(env);
  assert.equal(ok.status, 200, await ok.clone().text());
  const body = await ok.json();
  assert.equal(body.uuid, UUID);
  assert.ok(body.token);

  const otherSigner = await crypto.subtle.generateKey({ ...RSA, hash: "SHA-1" }, true, ["sign", "verify"]);
  assert.equal((await loginAs(env, { signer: otherSigner.privateKey })).status, 401, "no firmado por Mojang");
  assert.equal((await loginAs(env, { claimUuid: "f".repeat(32) })).status, 401, "UUID ajeno al certificado");
  assert.equal((await loginAs(env, { expiresInMs: -1000 })).status, 401, "certificado caducado");
  assert.equal((await loginAs(env, { signTicket: "otro" })).status, 401, "firma de otro desafío");

  const ch = await (await call(env, "POST", "/api/challenge")).json();
  const forged = await call(env, "POST", "/api/login", {
    body: JSON.stringify({ uuid: UUID, ticket: ch.ticket.replace(/.$/, "A") }),
  });
  assert.equal(forged.status, 401, "ticket manipulado");
});

test("flujo completo: subir, crear pack, leerlo, descargar y borrar", async () => {
  const env = makeEnv();
  const { token } = await (await loginAs(env)).json();
  const jar = new TextEncoder().encode("contenido de un mod privado");
  const hash = sha1(jar);

  assert.equal((await call(env, "PUT", `/api/files/${hash}`, { body: jar })).status, 401, "subir requiere sesión");
  assert.equal((await upload(env, token, jar)).status, 201);
  assert.equal((await (await upload(env, token, jar)).json()).existed, true, "dedupe por hash");

  const created = await call(env, "POST", "/api/packs", {
    token,
    body: JSON.stringify(manifest(`${ORIGIN}/files/${hash}`, hash, jar.byteLength)),
  });
  assert.equal(created.status, 201);
  const { code } = await created.json();
  assert.match(code, /^OBS-[A-Z0-9]{6}$/);

  const fetched = await (await call(env, "GET", `/${code}.json`)).json();
  assert.equal(fetched.code, code);
  assert.equal(fetched.mods[0].sha1, hash);

  const file = await call(env, "GET", `/files/${hash}`);
  assert.equal(file.status, 200);
  assert.equal(sha1(new Uint8Array(await file.arrayBuffer())), hash);

  assert.equal((await call(env, "DELETE", `/api/packs/${code}`, { token })).status, 204);
  assert.equal((await call(env, "GET", `/${code}.json`)).status, 404);
});

test("rechaza hashes falsos, archivos grandes y URLs ajenas", async () => {
  const env = makeEnv();
  const { token } = await (await loginAs(env)).json();
  const jar = new TextEncoder().encode("abc");

  const wrongHash = await call(env, "PUT", `/api/files/${"0".repeat(40)}`, {
    body: jar,
    token,
    headers: { "content-length": "3" },
  });
  assert.equal(wrongHash.status, 400);

  const big = new Uint8Array(2 * 1024 * 1024);
  assert.equal((await upload(env, token, big)).status, 413);

  const hash = sha1(jar);
  for (const url of [`https://evil.example/files/${hash}`, `${ORIGIN}/files/${hash}`]) {
    // la segunda es nuestra pero el archivo nunca se subió
    const r = await call(env, "POST", "/api/packs", { token, body: JSON.stringify(manifest(url, hash, 3)) });
    assert.equal(r.status, 400, url);
  }

  const traversal = manifest(`https://${"cdn.modrinth.com"}/x.jar`, hash, 3);
  traversal.mods[0].file_name = "../../evil.jar";
  assert.equal((await call(env, "POST", "/api/packs", { token, body: JSON.stringify(traversal) })).status, 400);

  const modrinth = manifest("https://cdn.modrinth.com/data/x/sodium.jar", hash, 3);
  assert.equal((await call(env, "POST", "/api/packs", { token, body: JSON.stringify(modrinth) })).status, 201);
});

test("sin almacenamiento configurado responde un error claro", async () => {
  const r = await worker.fetch(new Request(`${ORIGIN}/`), { SECRET: "x".repeat(40) });
  assert.equal(r.status, 500);
  assert.match((await r.json()).error, /almacenamiento/);
});

test("solo el dueño puede borrar un pack", async () => {
  const env = makeEnv();
  const { token } = await (await loginAs(env)).json();
  const r = await call(env, "POST", "/api/packs", {
    token,
    body: JSON.stringify(manifest("https://cdn.modrinth.com/a.jar", "a".repeat(40), 1)),
  });
  const { code } = await r.json();
  const key = `packs/${code}.json`;
  setOwner(env, key, "f".repeat(32));
  assert.equal((await call(env, "DELETE", `/api/packs/${code}`, { token })).status, 403);
});

test("catálogo: publicar pack y mod, listar, y solo el dueño o un admin borra", async () => {
  const env = makeEnv();
  const { token } = await (await loginAs(env)).json();
  const jar = new TextEncoder().encode("mod de la comunidad");
  const hash = sha1(jar);

  assert.deepEqual(await (await call(env, "GET", "/api/catalog")).json(), { items: [] });

  // El mod tiene que estar subido antes.
  const modBody = (size = jar.byteLength) =>
    JSON.stringify({
      kind: "mod",
      name: "Mi mod",
      author: "Steve",
      description: "Hace cosas",
      minecraft_version: "1.21.1",
      loader: "fabric",
      file: { file_name: "mi-mod.jar", sha1: hash, size },
    });
  assert.equal((await call(env, "POST", "/api/catalog", { token, body: modBody() })).status, 400, "sin subir");
  await upload(env, token, jar);
  assert.equal((await call(env, "POST", "/api/catalog", { body: modBody() })).status, 401, "sin sesión");
  assert.equal((await call(env, "POST", "/api/catalog", { token, body: modBody(999) })).status, 400, "tamaño falso");
  const mod = await call(env, "POST", "/api/catalog", { token, body: modBody() });
  assert.equal(mod.status, 201, await mod.clone().text());
  const modEntry = await mod.json();
  assert.equal(modEntry.owner, UUID);
  assert.equal((await call(env, "POST", "/api/catalog", { token, body: modBody() })).status, 409, "duplicado");

  const created = await call(env, "POST", "/api/packs", {
    token,
    body: JSON.stringify(manifest(`${ORIGIN}/files/${hash}`, hash, jar.byteLength)),
  });
  const { code } = await created.json();
  const pack = await call(env, "POST", "/api/catalog", {
    token,
    body: JSON.stringify({ kind: "pack", code, author: "Steve" }),
  });
  assert.equal(pack.status, 201);
  const packEntry = await pack.json();
  assert.equal(packEntry.mod_count, 1);

  const bad = await call(env, "POST", "/api/catalog", {
    token,
    body: JSON.stringify({ kind: "pack", code, author: "no válido!" }),
  });
  assert.equal(bad.status, 400, "autor inválido");

  const list = await (await call(env, "GET", "/api/catalog")).json();
  assert.deepEqual(
    list.items.map((e) => e.kind),
    ["pack", "mod"],
  );

  // Otro dueño no puede borrar; un admin sí.
  const catalogKey = "catalog/index.json";
  const raw = USE_KV ? env.FILES.store.get(catalogKey).value : env.BUCKET.store.get(catalogKey).bytes;
  const data = JSON.parse(new TextDecoder().decode(raw));
  data.items.find((e) => e.id === modEntry.id).owner = "f".repeat(32);
  await (USE_KV ? env.FILES : env.BUCKET).put(catalogKey, new TextEncoder().encode(JSON.stringify(data)));
  assert.equal((await call(env, "DELETE", `/api/catalog/${modEntry.id}`, { token })).status, 403);
  env.ADMIN_UUIDS = UUID;
  assert.equal((await call(env, "DELETE", `/api/catalog/${modEntry.id}`, { token })).status, 204);

  // Borrar el pack lo quita también del catálogo.
  assert.equal((await call(env, "DELETE", `/api/packs/${code}`, { token })).status, 204);
  assert.deepEqual((await (await call(env, "GET", "/api/catalog")).json()).items, []);
});
