// Pruebas del Worker sin Cloudflare: R2 en memoria y Mojang simulado.
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
  const base = { SECRET: "x".repeat(40), MAX_FILE_MB: "1" };
  return USE_KV ? { ...base, FILES: memoryKv() } : { ...base, BUCKET: memoryBucket() };
}

function setOwner(env, key, owner) {
  if (USE_KV) env.FILES.store.get(key).metadata.custom.owner = owner;
  else env.BUCKET.store.get(key).customMetadata.owner = owner;
}

// Mojang: solo confirma al usuario "Steve" con el último serverId que "se unió".
let joinedServerId = null;
globalThis.fetch = async (url) => {
  const u = new URL(url);
  if (u.hostname === "sessionserver.mojang.com") {
    if (u.searchParams.get("username") === "Steve" && u.searchParams.get("serverId") === joinedServerId) {
      return new Response(JSON.stringify({ id: UUID, name: "Steve" }), { status: 200 });
    }
    return new Response(null, { status: 204 });
  }
  throw new Error(`fetch inesperado: ${url}`);
};

const call = (env, method, path, { body, token, headers = {} } = {}) =>
  worker.fetch(
    new Request(ORIGIN + path, {
      method,
      body,
      headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), ...headers },
    }),
    env,
  );

async function loginAs(env, username = "Steve") {
  const ch = await (await call(env, "POST", "/api/challenge")).json();
  joinedServerId = ch.serverId; // lo que haría el launcher con /session/minecraft/join
  return call(env, "POST", "/api/login", { body: JSON.stringify({ username, ticket: ch.ticket }) });
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

test("login con Mojang emite un token y rechaza a quien no se unió", async () => {
  const env = makeEnv();
  const ok = await loginAs(env);
  assert.equal(ok.status, 200);
  const body = await ok.json();
  assert.equal(body.uuid, UUID);
  assert.ok(body.token);

  const ch = await (await call(env, "POST", "/api/challenge")).json();
  const bad = await call(env, "POST", "/api/login", { body: JSON.stringify({ username: "Steve", ticket: ch.ticket }) });
  assert.equal(bad.status, 401, "sin join previo no hay sesión");

  const forged = await call(env, "POST", "/api/login", {
    body: JSON.stringify({ username: "Steve", ticket: ch.ticket.replace(/.$/, "A") }),
  });
  assert.equal(forged.status, 401);
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
