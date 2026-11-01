//! Compartir con servidor: sube lo que no está en Modrinth (mods privados,
//! imagen, configs) al servidor de Obsidia (`server/` en este repo) y obtiene
//! un código corto `OBS-XXXXXX`.
//!
//! El manifiesto que se publica tiene exactamente el formato que ya lee
//! `share::fetch_manifest`, así que importar no cambia: el servidor sirve
//! `<URL>/OBS-XXXXXX.json` igual que cualquier servicio de manifiestos.
//!
//! Inicio de sesión: como un servidor de Minecraft en online-mode. Se pide un
//! `serverId` al servidor, se llama a `session/minecraft/join` de Mojang con el
//! token de la cuenta y el servidor lo comprueba con `hasJoined`. El token de
//! Minecraft solo viaja a Mojang, nunca al servidor de Obsidia.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    background,
    downloads::sha1_file,
    error::CoreError,
    instances::{Loader, Memory},
    share::{InstanceManifest, ManifestFile},
    share_code::{self, MODRINTH_CDN_HOST, USER_AGENT},
    Result,
};

const MOJANG_JOIN: &str = "https://sessionserver.mojang.com/session/minecraft/join";
/// Topes de la carpeta `config/`: suele pesar poco, pero algunos mods guardan
/// cachés enormes ahí.
const MAX_CONFIG_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CONFIG_FILES: usize = 300;
const MAX_CONFIG_TOTAL_BYTES: u64 = 20 * 1024 * 1024;

/// Cuenta de Minecraft con la que se sube (debe ser de Microsoft, no local).
pub struct Identity {
    pub username: String,
    /// UUID sin guiones.
    pub uuid: String,
    pub access_token: String,
}

pub struct UploadInput<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub minecraft_version: &'a str,
    pub loader: Loader,
    pub loader_version: Option<&'a str>,
    pub memory: Memory,
    pub instance_dir: &'a Path,
    pub include_config: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadedPack {
    pub code: String,
    /// Archivos que se descargarán de Modrinth.
    pub from_modrinth: usize,
    /// Archivos alojados en el servidor (subidos ahora o ya existentes).
    pub hosted: usize,
    /// Bytes subidos en esta operación.
    pub uploaded_bytes: u64,
    pub has_background: bool,
}

/// `fase, hecho, total` (en bytes durante la subida).
pub type Progress<'a> = &'a (dyn Fn(&str, u64, u64) + Send + Sync);

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(15))
        // Subir un mod grande con una conexión lenta tarda.
        .timeout(Duration::from_secs(10 * 60))
        .build()
        .map_err(|e| CoreError::Network(e.to_string()))
}

fn normalize_base(base_url: &str) -> Result<String> {
    let base = base_url.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(base)
        .map_err(|_| CoreError::invalid("la URL del servidor para compartir no es válida"))?;
    if parsed.scheme() != "https" {
        return Err(CoreError::invalid("la URL del servidor para compartir debe ser https"));
    }
    Ok(base.to_owned())
}

#[derive(Deserialize)]
struct ServerError {
    error: String,
}

/// Convierte una respuesta no exitosa en un error legible con el mensaje del servidor.
async fn check(resp: reqwest::Response, what: &str) -> Result<reqwest::Response> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let msg = resp
        .json::<ServerError>()
        .await
        .map(|e| e.error)
        .unwrap_or_else(|_| format!("HTTP {status}"));
    Err(CoreError::Network(format!("{what}: {msg}")))
}

fn net(what: &'static str) -> impl Fn(reqwest::Error) -> CoreError {
    move |e| CoreError::Network(format!("{what}: {e}"))
}

// ───────────────────────────── Sesión ─────────────────────────────

#[derive(Deserialize)]
struct Challenge {
    #[serde(rename = "serverId")]
    server_id: String,
    ticket: String,
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

async fn login(client: &reqwest::Client, base: &str, identity: &Identity) -> Result<String> {
    let ch: Challenge = check(
        client
            .post(format!("{base}/api/challenge"))
            .send()
            .await
            .map_err(net("servidor para compartir"))?,
        "servidor para compartir",
    )
    .await?
    .json()
    .await
    .map_err(net("servidor para compartir"))?;

    check(
        client
            .post(MOJANG_JOIN)
            .json(&serde_json::json!({
                "accessToken": identity.access_token,
                "selectedProfile": identity.uuid,
                "serverId": ch.server_id,
            }))
            .send()
            .await
            .map_err(net("Mojang"))?,
        "Mojang no aceptó tu sesión (vuelve a iniciar sesión)",
    )
    .await?;

    let login: LoginResponse = check(
        client
            .post(format!("{base}/api/login"))
            .json(&serde_json::json!({ "username": identity.username, "ticket": ch.ticket }))
            .send()
            .await
            .map_err(net("servidor para compartir"))?,
        "inicio de sesión en el servidor",
    )
    .await?
    .json()
    .await
    .map_err(net("servidor para compartir"))?;
    Ok(login.token)
}

// ───────────────────────────── Archivos ─────────────────────────────

/// Un archivo local que hay que alojar en el servidor.
struct LocalFile {
    file_name: String,
    path: PathBuf,
    sha1: String,
    size: u64,
}

/// Archivos de `config/` (recursivo, con `/` como separador). Sin symlinks.
fn collect_config(dir: &Path) -> Result<Vec<LocalFile>> {
    let mut out = Vec::new();
    let mut total = 0u64;
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((current, prefix)) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(CoreError::io(&current, e)),
        };
        for entry in entries {
            let entry = entry.map_err(|e| CoreError::io(&current, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name.contains(['/', '\\']) {
                continue;
            }
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path).map_err(|e| CoreError::io(&path, e))?;
            let rel = format!("{prefix}{name}");
            if meta.is_dir() {
                stack.push((path, format!("{rel}/")));
            } else if meta.is_file() {
                if meta.len() > MAX_CONFIG_FILE_BYTES {
                    continue; // cachés y bases de datos de mods: no son configuración
                }
                total += meta.len();
                if out.len() >= MAX_CONFIG_FILES || total > MAX_CONFIG_TOTAL_BYTES {
                    return Err(CoreError::invalid(
                        "la carpeta config es demasiado grande; desmarca «Incluir configuración»",
                    ));
                }
                let sha1 = sha1_file(&path).map_err(|e| CoreError::io(&path, e))?;
                out.push(LocalFile {
                    file_name: rel,
                    path,
                    sha1,
                    size: meta.len(),
                });
            }
        }
    }
    out.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(out)
}

fn local(dir: &Path, list: Vec<(String, String)>) -> Result<Vec<LocalFile>> {
    list.into_iter()
        .map(|(name, sha1)| {
            let path = dir.join(&name);
            let size = std::fs::metadata(&path).map_err(|e| CoreError::io(&path, e))?.len();
            Ok(LocalFile {
                file_name: name,
                path,
                sha1,
                size,
            })
        })
        .collect()
}

async fn upload_file(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    file_name: &str,
    sha1: &str,
    bytes: Vec<u8>,
) -> Result<()> {
    let what = format!("no se pudo subir {file_name}");
    check(
        client
            .put(format!("{base}/api/files/{sha1}"))
            .bearer_auth(token)
            // Con un `Vec` reqwest ya envía `Content-Length`, que el servidor exige.
            .body(bytes)
            .send()
            .await
            .map_err(|e| CoreError::Network(format!("{what}: {e}")))?,
        &what,
    )
    .await?;
    Ok(())
}

async fn exists_on_server(client: &reqwest::Client, base: &str, sha1: &str) -> Result<bool> {
    let resp = client
        .head(format!("{base}/files/{sha1}"))
        .send()
        .await
        .map_err(net("servidor para compartir"))?;
    Ok(resp.status().is_success())
}

// ───────────────────────────── Subir el pack ─────────────────────────────

/// Sube la instancia y devuelve su código corto.
pub async fn upload(
    base_url: &str,
    identity: &Identity,
    input: UploadInput<'_>,
    progress: Progress<'_>,
) -> Result<UploadedPack> {
    let base = normalize_base(base_url)?;
    let client = http_client()?;
    let dir = input.instance_dir;

    progress("Analizando la instancia", 0, 0);
    let mods = share_code::hash_dir(&dir.join("mods"), &[".jar"]).await?;
    let rps = share_code::hash_dir(&dir.join("resourcepacks"), &[".zip"]).await?;
    let sps = share_code::hash_dir(&dir.join("shaderpacks"), &[".zip"]).await?;
    let (mods, rps, sps, configs) = {
        let dir = dir.to_path_buf();
        let include_config = input.include_config;
        tokio::task::spawn_blocking(move || -> Result<_> {
            let configs = if include_config {
                collect_config(&dir.join("config"))?
            } else {
                Vec::new()
            };
            Ok((
                local(&dir.join("mods"), mods)?,
                local(&dir.join("resourcepacks"), rps)?,
                local(&dir.join("shaderpacks"), sps)?,
                configs,
            ))
        })
        .await
        .map_err(|e| CoreError::invalid(e.to_string()))??
    };

    progress("Buscando archivos en Modrinth", 0, 0);
    let hashes: Vec<String> = mods.iter().chain(&rps).chain(&sps).map(|f| f.sha1.clone()).collect();
    let modrinth = if hashes.is_empty() {
        HashMap::new()
    } else {
        share_code::versions_by_hash(&client, &hashes).await?
    };
    let modrinth_file = |f: &LocalFile| -> Option<ManifestFile> {
        let file = modrinth
            .get(&f.sha1)?
            .files
            .iter()
            .find(|mf| mf.hashes.sha1.eq_ignore_ascii_case(&f.sha1))?;
        let host_ok = reqwest::Url::parse(&file.url)
            .ok()
            .is_some_and(|u| u.host_str() == Some(MODRINTH_CDN_HOST));
        host_ok.then(|| ManifestFile {
            file_name: f.file_name.clone(),
            url: file.url.clone(),
            sha1: Some(f.sha1.clone()),
            size: Some(file.size),
        })
    };

    let background = background::load(dir)?;

    progress("Iniciando sesión", 0, 0);
    let token = login(&client, &base, identity).await?;

    // Qué hay que subir: lo que no está en Modrinth, configs e imagen.
    let mut to_host: Vec<(&LocalFile, Option<&[u8]>)> = Vec::new();
    let mut from_modrinth = 0;
    let mut lists: [Vec<ManifestFile>; 4] = Default::default();
    for (i, files) in [&mods, &rps, &sps, &configs].into_iter().enumerate() {
        for f in files {
            let entry = if i < 3 { modrinth_file(f) } else { None };
            match entry {
                Some(entry) => {
                    from_modrinth += 1;
                    lists[i].push(entry);
                }
                None => {
                    to_host.push((f, None));
                    lists[i].push(ManifestFile {
                        file_name: f.file_name.clone(),
                        url: format!("{base}/files/{}", f.sha1),
                        sha1: Some(f.sha1.clone()),
                        size: Some(f.size),
                    });
                }
            }
        }
    }
    let bg_file = background.as_ref().map(|(_, bytes)| {
        use sha1::{Digest, Sha1};
        LocalFile {
            file_name: "background".into(),
            path: PathBuf::new(),
            sha1: crate::downloads::hex(&Sha1::digest(bytes)),
            size: bytes.len() as u64,
        }
    });
    if let (Some(f), Some((_, bytes))) = (&bg_file, &background) {
        to_host.push((f, Some(bytes.as_slice())));
    }

    let total: u64 = to_host.iter().map(|(f, _)| f.size).sum();
    let mut done = 0u64;
    let mut uploaded_bytes = 0u64;
    for (f, in_memory) in &to_host {
        progress(&format!("Subiendo {}", f.file_name), done, total);
        if !exists_on_server(&client, &base, &f.sha1).await? {
            let bytes = match in_memory {
                Some(b) => b.to_vec(),
                None => tokio::fs::read(&f.path).await.map_err(|e| CoreError::io(&f.path, e))?,
            };
            // El archivo pudo cambiar desde que se calculó el hash.
            if bytes.len() as u64 != f.size {
                return Err(CoreError::invalid(format!(
                    "{} cambió mientras se compartía; vuelve a intentarlo",
                    f.file_name
                )));
            }
            upload_file(&client, &base, &token, &f.file_name, &f.sha1, bytes).await?;
            uploaded_bytes += f.size;
        }
        done += f.size;
    }

    progress("Publicando el pack", total, total);
    let [mods, resourcepacks, shaderpacks, config_files] = lists;
    let manifest = InstanceManifest {
        code: String::new(),
        name: input.name.to_owned(),
        description: share_code::clean_description(input.description),
        icon_url: None,
        background_url: bg_file.as_ref().map(|f| format!("{base}/files/{}", f.sha1)),
        minecraft_version: input.minecraft_version.to_owned(),
        loader: input.loader,
        loader_version: input.loader_version.map(str::to_owned),
        java: None,
        memory: Some(input.memory),
        mods,
        resourcepacks,
        shaderpacks,
        config_files,
    };
    #[derive(Deserialize)]
    struct Created {
        code: String,
    }
    let created: Created = check(
        client
            .post(format!("{base}/api/packs"))
            .bearer_auth(&token)
            .json(&manifest)
            .send()
            .await
            .map_err(net("servidor para compartir"))?,
        "publicar el pack",
    )
    .await?
    .json()
    .await
    .map_err(net("servidor para compartir"))?;
    crate::share::validate_code(&created.code)?;

    Ok(UploadedPack {
        code: created.code,
        from_modrinth,
        hosted: to_host.len(),
        uploaded_bytes,
        has_background: bg_file.is_some(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instances::test_util::TempDir;

    #[test]
    fn base_url_must_be_https() {
        assert_eq!(normalize_base("https://x.workers.dev/").unwrap(), "https://x.workers.dev");
        assert!(normalize_base("http://x.workers.dev").is_err());
        assert!(normalize_base("nada").is_err());
    }

    #[test]
    fn config_is_collected_recursively_and_skips_big_or_hidden_files() {
        let tmp = TempDir::new();
        let config = tmp.path().join("config");
        std::fs::create_dir_all(config.join("sodium")).unwrap();
        std::fs::write(config.join("a.json"), b"{}").unwrap();
        std::fs::write(config.join("sodium/options.json"), b"{\"x\":1}").unwrap();
        std::fs::write(config.join(".oculto"), b"x").unwrap();
        std::fs::write(config.join("cache.db"), vec![0u8; (MAX_CONFIG_FILE_BYTES + 1) as usize]).unwrap();

        let files = collect_config(&config).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.file_name.as_str()).collect();
        assert_eq!(names, ["a.json", "sodium/options.json"]);
        assert_eq!(files[1].size, 7);
    }

    #[test]
    fn missing_config_dir_is_empty() {
        let tmp = TempDir::new();
        assert!(collect_config(&tmp.path().join("config")).unwrap().is_empty());
    }
}
