//! Códigos de instancia autocontenidos (`OBS1-…`).
//!
//! A diferencia de los códigos cortos `OBS-XXXXXX` (que solo son una clave
//! para buscar un manifiesto en un servidor), un código `OBS1-` *es* el
//! contenedor: lleva dentro el nombre, la descripción, la versión de
//! Minecraft, el loader, la memoria, la URL de la imagen y la lista de mods,
//! resource packs y shaders. No hace falta ningún servidor propio.
//!
//! Los archivos no caben en un código, así que cada uno se guarda como
//! `<id de versión de Modrinth>.<prefijo de su SHA-1>`. Al importar se le
//! pregunta a Modrinth por esas versiones y se elige el archivo cuyo SHA-1
//! empieza por ese prefijo; el SHA-1 completo y el tamaño que devuelve
//! Modrinth son los que luego verifica el `Downloader`. Los archivos que no
//! están en Modrinth no se pueden incluir y se avisa al exportar.
//!
//! Formato: `OBS1-` + base64url(deflate(JSON compacto de [`Pack`])).

use std::{collections::HashMap, io::Read, path::Path};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use crate::{
    downloads::sha1_file,
    error::CoreError,
    instances::{validate_loader_version, validate_minecraft_version, Loader, Memory},
    share::{self, InstanceManifest, ManifestFile},
    Result,
};

pub const PREFIX: &str = "OBS1-";
const MODRINTH_API: &str = "https://api.modrinth.com/v2";
pub(crate) const MODRINTH_CDN_HOST: &str = "cdn.modrinth.com";
pub(crate) const USER_AGENT: &str = concat!(
    "ObsidiaLauncher/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/DevAldx/Obsidia)"
);
/// Caracteres del SHA-1 que se guardan en el código para cada archivo.
const HASH_PREFIX_LEN: usize = 12;
/// Un código pegado no debería pasar de unos pocos KB; esto deja margen de sobra.
const MAX_CODE_LEN: usize = 64 * 1024;
/// Tope del JSON descomprimido: evita bombas de compresión.
const MAX_INFLATED_BYTES: u64 = 512 * 1024;
const MAX_ENTRIES: usize = 512;
const MAX_DESCRIPTION_CHARS: usize = 300;
/// Modrinth acepta bastantes IDs por petición, pero la URL tiene un límite.
const IDS_PER_REQUEST: usize = 100;
const HASHES_PER_REQUEST: usize = 200;

pub fn is_pack_code(code: &str) -> bool {
    code.trim().starts_with(PREFIX)
}

/// Contenido del código. Nombres de campo cortos: cada byte cuenta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Pack {
    /// Versión del formato.
    v: u32,
    /// Nombre.
    n: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    d: Option<String>,
    mc: String,
    l: Loader,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lv: Option<String>,
    /// Memoria `[mín, máx]` en MB.
    ram: [u32; 2],
    /// URL https de la imagen de fondo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bg: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    m: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    rp: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    sp: Vec<String>,
}

const PACK_VERSION: u32 = 1;

fn encode(pack: &Pack) -> Result<String> {
    use std::io::Write;
    let json = serde_json::to_vec(pack)
        .map_err(|e| CoreError::invalid(format!("no se pudo generar el código: {e}")))?;
    let mut enc = DeflateEncoder::new(Vec::new(), Compression::best());
    enc.write_all(&json)
        .and_then(|()| enc.finish())
        .map(|bytes| format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(bytes)))
        .map_err(|e| CoreError::invalid(format!("no se pudo comprimir el código: {e}")))
}

fn decode(code: &str) -> Result<Pack> {
    // Al copiar de un chat a veces se cuelan saltos de línea o espacios.
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() > MAX_CODE_LEN {
        return Err(CoreError::invalid("el código es demasiado largo"));
    }
    let body = code
        .strip_prefix(PREFIX)
        .ok_or_else(|| CoreError::invalid("el código debe empezar por OBS1-"))?;
    let invalid = || CoreError::invalid("el código está incompleto o mal copiado");
    let compressed = URL_SAFE_NO_PAD.decode(body).map_err(|_| invalid())?;
    let mut json = Vec::new();
    DeflateDecoder::new(compressed.as_slice())
        .take(MAX_INFLATED_BYTES + 1)
        .read_to_end(&mut json)
        .map_err(|_| invalid())?;
    if json.len() as u64 > MAX_INFLATED_BYTES {
        return Err(CoreError::invalid("el código es demasiado grande"));
    }
    let pack: Pack = serde_json::from_slice(&json).map_err(|_| invalid())?;
    if pack.v != PACK_VERSION {
        return Err(CoreError::invalid(
            "este código es de una versión más nueva de Obsidia; actualiza el launcher",
        ));
    }
    Ok(pack)
}

/// `OBS-XXXXXX` estable derivado del código, para usarlo como clave de
/// progreso y de validación en el flujo de importación ya existente.
fn display_key(code: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let digest = Sha1::digest(code.trim().as_bytes());
    let suffix: String = digest
        .iter()
        .take(6)
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect();
    format!("OBS-{suffix}")
}

// ───────────────────────────── Modrinth ─────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct MrHashes {
    pub(crate) sha1: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MrFile {
    pub(crate) hashes: MrHashes,
    pub(crate) url: String,
    pub(crate) filename: String,
    pub(crate) size: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MrVersion {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) project_id: String,
    pub(crate) files: Vec<MrFile>,
}

fn net(e: reqwest::Error) -> CoreError {
    CoreError::Network(format!("Modrinth: {e}"))
}

/// SHA-1 → versión de Modrinth que contiene ese archivo.
pub(crate) async fn versions_by_hash(
    client: &reqwest::Client,
    hashes: &[String],
) -> Result<HashMap<String, MrVersion>> {
    let mut out = HashMap::new();
    for chunk in hashes.chunks(HASHES_PER_REQUEST) {
        let resp = client
            .post(format!("{MODRINTH_API}/version_files"))
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .json(&serde_json::json!({ "hashes": chunk, "algorithm": "sha1" }))
            .send()
            .await
            .map_err(net)?
            .error_for_status()
            .map_err(net)?;
        let found: HashMap<String, MrVersion> = resp.json().await.map_err(net)?;
        out.extend(found.into_iter().map(|(k, v)| (k.to_ascii_lowercase(), v)));
    }
    Ok(out)
}

async fn versions_by_id(
    client: &reqwest::Client,
    ids: &[&str],
) -> Result<HashMap<String, MrVersion>> {
    let mut out = HashMap::new();
    for chunk in ids.chunks(IDS_PER_REQUEST) {
        let ids_json = serde_json::to_string(chunk)
            .map_err(|e| CoreError::invalid(e.to_string()))?;
        let mut url = reqwest::Url::parse(&format!("{MODRINTH_API}/versions"))
            .map_err(|e| CoreError::Network(e.to_string()))?;
        url.query_pairs_mut().append_pair("ids", &ids_json);
        let resp = client
            .get(url)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(net)?
            .error_for_status()
            .map_err(net)?;
        let found: Vec<MrVersion> = resp.json().await.map_err(net)?;
        out.extend(found.into_iter().map(|v| (v.id.clone(), v)));
    }
    Ok(out)
}

// ───────────────────────────── Exportar ─────────────────────────────

/// Datos de la instancia que van dentro del código.
pub struct ExportInput<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub minecraft_version: &'a str,
    pub loader: Loader,
    pub loader_version: Option<&'a str>,
    pub memory: Memory,
    pub background_url: Option<&'a str>,
    pub mods_dir: &'a Path,
    pub resourcepacks_dir: &'a Path,
    pub shaderpacks_dir: &'a Path,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportedCode {
    pub code: String,
    /// Archivos incluidos (encontrados en Modrinth).
    pub included: usize,
    /// Archivos que no están en Modrinth y por tanto no viajan en el código.
    pub missing: Vec<String>,
}

/// Archivos activos de una carpeta con alguna de las extensiones dadas.
fn active_files(dir: &Path, extensions: &[&str]) -> Result<Vec<String>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CoreError::io(dir, e)),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| CoreError::io(dir, e))?;
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        if extensions.iter().any(|ext| lower.ends_with(ext)) {
            out.push(name);
        }
    }
    out.sort_by_key(|n| n.to_ascii_lowercase());
    Ok(out)
}

/// (carpeta, nombres de archivo, SHA-1 de cada uno), calculado fuera del runtime async.
pub(crate) async fn hash_dir(dir: &Path, extensions: &'static [&'static str]) -> Result<Vec<(String, String)>> {
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        active_files(&dir, extensions)?
            .into_iter()
            .map(|name| {
                let path = dir.join(&name);
                sha1_file(&path)
                    .map(|h| (name, h.to_ascii_lowercase()))
                    .map_err(|e| CoreError::io(path, e))
            })
            .collect()
    })
    .await
    .map_err(|e| CoreError::invalid(format!("error calculando hashes: {e}")))?
}

fn validate_background_url(url: &str) -> Result<String> {
    let url = url.trim();
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| CoreError::invalid("la URL de la imagen no es válida"))?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err(CoreError::invalid("la URL de la imagen debe empezar por https://"));
    }
    if url.len() > 512 {
        return Err(CoreError::invalid("la URL de la imagen es demasiado larga"));
    }
    Ok(url.to_owned())
}

pub(crate) fn clean_description(d: Option<&str>) -> Option<String> {
    let d = d?.trim();
    (!d.is_empty()).then(|| d.chars().filter(|c| !c.is_control()).take(MAX_DESCRIPTION_CHARS).collect())
}

/// Genera el código `OBS1-…` de una instancia: busca cada mod, resource pack
/// y shader en Modrinth por su SHA-1 y empaqueta todo.
pub async fn export(client: &reqwest::Client, input: ExportInput<'_>) -> Result<ExportedCode> {
    let background_url = input
        .background_url
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(validate_background_url)
        .transpose()?;

    let mods = hash_dir(input.mods_dir, &[".jar"]).await?;
    let rps = hash_dir(input.resourcepacks_dir, &[".zip"]).await?;
    let sps = hash_dir(input.shaderpacks_dir, &[".zip"]).await?;
    if mods.len() + rps.len() + sps.len() > MAX_ENTRIES {
        return Err(CoreError::invalid("la instancia tiene demasiados archivos para un código"));
    }

    let all_hashes: Vec<String> = mods
        .iter()
        .chain(&rps)
        .chain(&sps)
        .map(|(_, h)| h.clone())
        .collect();
    let found = if all_hashes.is_empty() {
        HashMap::new()
    } else {
        versions_by_hash(client, &all_hashes).await?
    };

    let mut missing = Vec::new();
    let mut included = 0;
    let mut entries = |files: &[(String, String)]| -> Vec<String> {
        let mut out = Vec::new();
        for (name, hash) in files {
            match found.get(hash) {
                Some(version) => {
                    included += 1;
                    out.push(format!("{}.{}", version.id, &hash[..HASH_PREFIX_LEN]));
                }
                None => missing.push(name.clone()),
            }
        }
        out
    };
    let m = entries(&mods);
    let rp = entries(&rps);
    let sp = entries(&sps);

    let pack = Pack {
        v: PACK_VERSION,
        n: input.name.to_owned(),
        d: clean_description(input.description),
        mc: input.minecraft_version.to_owned(),
        l: input.loader,
        lv: input.loader_version.map(str::to_owned),
        ram: [input.memory.min_mb, input.memory.max_mb],
        bg: background_url,
        m,
        rp,
        sp,
    };
    Ok(ExportedCode {
        code: encode(&pack)?,
        included,
        missing,
    })
}

// ───────────────────────────── Importar ─────────────────────────────

/// `"<id>.<prefijo sha1>"` → (id, prefijo), validando ambos.
fn parse_entry(entry: &str) -> Result<(&str, &str)> {
    let bad = || CoreError::invalid("el código contiene un archivo mal formado");
    let (id, prefix) = entry.split_once('.').ok_or_else(bad)?;
    let id_ok = !id.is_empty() && id.len() <= 16 && id.bytes().all(|b| b.is_ascii_alphanumeric());
    let prefix_ok = prefix.len() == HASH_PREFIX_LEN && prefix.bytes().all(|b| b.is_ascii_hexdigit());
    if id_ok && prefix_ok {
        Ok((id, prefix))
    } else {
        Err(bad())
    }
}

fn safe_file_name(name: &str, extension: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && !name.contains(['/', '\\', '\0'])
        && name.to_ascii_lowercase().ends_with(extension)
}

fn resolve_entries(
    entries: &[String],
    versions: &HashMap<String, MrVersion>,
    extension: &str,
    label: &str,
) -> Result<Vec<ManifestFile>> {
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        let (id, prefix) = parse_entry(entry)?;
        let file = versions
            .get(id)
            .and_then(|v| {
                v.files
                    .iter()
                    .find(|f| f.hashes.sha1.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase()))
            })
            .ok_or_else(|| {
                CoreError::invalid(format!(
                    "{label}: el archivo {id} ya no está disponible en Modrinth"
                ))
            })?;
        let host_ok = reqwest::Url::parse(&file.url)
            .ok()
            .is_some_and(|u| u.scheme() == "https" && u.host_str() == Some(MODRINTH_CDN_HOST));
        if !host_ok || !safe_file_name(&file.filename, extension) {
            return Err(CoreError::invalid(format!(
                "{label}: Modrinth devolvió un archivo no válido ({})",
                file.filename
            )));
        }
        out.push(ManifestFile {
            file_name: file.filename.clone(),
            url: file.url.clone(),
            sha1: Some(file.hashes.sha1.to_ascii_lowercase()),
            size: Some(file.size),
        });
    }
    Ok(out)
}

/// Decodifica un código `OBS1-…` y lo convierte en un manifiesto completo
/// (con URL, SHA-1 y tamaño de cada archivo), listo para la vista previa y
/// para `share::install`.
pub async fn resolve(client: &reqwest::Client, code: &str) -> Result<InstanceManifest> {
    let pack = decode(code)?;
    if pack.m.len() + pack.rp.len() + pack.sp.len() > MAX_ENTRIES {
        return Err(CoreError::invalid("el código declara demasiados archivos"));
    }
    validate_minecraft_version(&pack.mc)?;
    if let Some(lv) = &pack.lv {
        validate_loader_version(lv)?;
    }
    if pack.l == Loader::Fabric && pack.lv.is_none() {
        return Err(CoreError::invalid(
            "el código pide Fabric pero no indica la versión del loader",
        ));
    }
    let background_url = pack.bg.as_deref().map(validate_background_url).transpose()?;

    let mut ids: Vec<&str> = Vec::new();
    for entry in pack.m.iter().chain(&pack.rp).chain(&pack.sp) {
        let (id, _) = parse_entry(entry)?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    let versions = if ids.is_empty() {
        HashMap::new()
    } else {
        versions_by_id(client, &ids).await?
    };

    let manifest = InstanceManifest {
        code: display_key(code),
        name: pack.n,
        description: clean_description(pack.d.as_deref()),
        icon_url: None,
        background_url,
        minecraft_version: pack.mc,
        loader: pack.l,
        loader_version: pack.lv,
        java: None,
        memory: Some(Memory {
            min_mb: pack.ram[0],
            max_mb: pack.ram[1],
        }),
        mods: resolve_entries(&pack.m, &versions, ".jar", "mods")?,
        resourcepacks: resolve_entries(&pack.rp, &versions, ".zip", "resourcepacks")?,
        shaderpacks: resolve_entries(&pack.sp, &versions, ".zip", "shaderpacks")?,
        config_files: Vec::new(),
    };
    share::validate_manifest(&manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Pack {
        Pack {
            v: PACK_VERSION,
            n: "Survival con amigos".into(),
            d: Some("Fabric + Sodium".into()),
            mc: "1.21.1".into(),
            l: Loader::Fabric,
            lv: Some("0.16.5".into()),
            ram: [2048, 6144],
            bg: Some("https://i.imgur.com/abc.png".into()),
            m: vec!["AANobbMI.0123456789ab".into(), "gvQqBUqZ.ba9876543210".into()],
            rp: vec![],
            sp: vec!["xyz12345.aaaaaaaaaaaa".into()],
        }
    }

    #[test]
    fn roundtrip() {
        let code = encode(&sample()).unwrap();
        assert!(code.starts_with(PREFIX));
        assert!(is_pack_code(&code));
        assert_eq!(decode(&code).unwrap(), sample());
    }

    #[test]
    fn decode_tolerates_whitespace_from_chat_copies() {
        let code = encode(&sample()).unwrap();
        let (a, b) = code.split_at(20);
        assert_eq!(decode(&format!("  {a}\n{b} ")).unwrap(), sample());
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode("OBS1-noesuncodigo!!").is_err());
        assert!(decode("OBS-K7P2X9").is_err());
        let code = encode(&sample()).unwrap();
        assert!(decode(&code[..code.len() - 10]).is_err());
    }

    #[test]
    fn display_key_is_a_valid_instance_code() {
        let code = encode(&sample()).unwrap();
        let key = display_key(&code);
        assert_eq!(share::validate_code(&key).unwrap(), key);
        assert_eq!(display_key(&code), key);
    }

    #[test]
    fn entries_are_validated() {
        assert!(parse_entry("AANobbMI.0123456789ab").is_ok());
        assert!(parse_entry("AANobbMI").is_err());
        assert!(parse_entry("../x.0123456789ab").is_err());
        assert!(parse_entry("AANobbMI.0123").is_err());
    }

    #[test]
    fn resolve_picks_file_by_hash_prefix_and_rejects_foreign_hosts() {
        let mut versions = HashMap::new();
        versions.insert(
            "abc".to_owned(),
            MrVersion {
                id: "abc".into(),
                project_id: String::new(),
                files: vec![
                    MrFile {
                        hashes: MrHashes { sha1: "f".repeat(40) },
                        url: "https://cdn.modrinth.com/data/x/other.jar".into(),
                        filename: "other.jar".into(),
                        size: 1,
                    },
                    MrFile {
                        hashes: MrHashes { sha1: format!("0123456789ab{}", "0".repeat(28)) },
                        url: "https://cdn.modrinth.com/data/x/sodium.jar".into(),
                        filename: "sodium.jar".into(),
                        size: 42,
                    },
                ],
            },
        );
        let files = resolve_entries(&["abc.0123456789ab".into()], &versions, ".jar", "mods").unwrap();
        assert_eq!(files[0].file_name, "sodium.jar");
        assert_eq!(files[0].size, Some(42));

        versions.get_mut("abc").unwrap().files[1].url = "https://evil.example/sodium.jar".into();
        assert!(resolve_entries(&["abc.0123456789ab".into()], &versions, ".jar", "mods").is_err());
        assert!(resolve_entries(&["zzz.0123456789ab".into()], &versions, ".jar", "mods").is_err());
    }
}
