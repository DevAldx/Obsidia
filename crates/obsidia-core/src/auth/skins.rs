//! Skins y capas de una cuenta de Microsoft (API de Minecraft Services).
//!
//! - `GET    /minecraft/profile`               perfil con skins y capas
//! - `POST   /minecraft/profile/skins`         sube una skin (multipart)
//! - `DELETE /minecraft/profile/skins/active`  vuelve a la skin por defecto
//! - `PUT    /minecraft/profile/capes/active`  activa una capa que ya posees
//! - `DELETE /minecraft/profile/capes/active`  oculta la capa
//!
//! Las rutas se derivan de `mc_profile_url` y pasan por `AuthEndpoints::check`,
//! así que solo pueden ir a `api.minecraftservices.com` por HTTPS.
//!
//! Además hay una biblioteca local (`data/skins`) con las skins que el usuario
//! ha subido, para volver a ponerlas con un clic.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::endpoints::AuthEndpoints;
use super::mc::normalize_skin_url;
use crate::{error::CoreError, filesystem, Result};

/// Mojang rechaza archivos grandes; una skin real ocupa unos pocos KB.
pub const MAX_SKIN_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinVariant {
    Classic,
    Slim,
}

impl SkinVariant {
    fn api_name(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Slim => "slim",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkinInfo {
    pub id: String,
    pub url: String,
    pub variant: SkinVariant,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapeInfo {
    pub id: String,
    pub url: String,
    pub alias: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkinProfile {
    pub username: String,
    pub skins: Vec<SkinInfo>,
    pub capes: Vec<CapeInfo>,
}

impl SkinProfile {
    pub fn active_skin(&self) -> Option<&SkinInfo> {
        self.skins.iter().find(|s| s.active)
    }
}

#[derive(Deserialize)]
struct RawProfile {
    name: String,
    #[serde(default)]
    skins: Vec<RawSkin>,
    #[serde(default)]
    capes: Vec<RawCape>,
}

#[derive(Deserialize)]
struct RawSkin {
    id: String,
    state: String,
    url: String,
    #[serde(default)]
    variant: String,
}

#[derive(Deserialize)]
struct RawCape {
    id: String,
    state: String,
    url: String,
    #[serde(default)]
    alias: String,
}

pub fn parse_profile(text: &str) -> Result<SkinProfile> {
    let raw: RawProfile = serde_json::from_str(text)
        .map_err(|e| CoreError::Auth(format!("respuesta de perfil inesperada: {e}")))?;
    Ok(SkinProfile {
        username: raw.name,
        skins: raw
            .skins
            .into_iter()
            .map(|s| SkinInfo {
                id: s.id,
                url: normalize_skin_url(&s.url),
                variant: if s.variant.eq_ignore_ascii_case("slim") {
                    SkinVariant::Slim
                } else {
                    SkinVariant::Classic
                },
                active: s.state == "ACTIVE",
            })
            .collect(),
        capes: raw
            .capes
            .into_iter()
            .map(|c| CapeInfo {
                id: c.id,
                url: normalize_skin_url(&c.url),
                alias: c.alias,
                active: c.state == "ACTIVE",
            })
            .collect(),
    })
}

/// Comprueba que los bytes son un PNG de 64×64 o 64×32 (lo que acepta Minecraft).
pub fn validate_skin_png(bytes: &[u8]) -> Result<()> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() > MAX_SKIN_BYTES {
        return Err(CoreError::invalid("la skin supera los 256 KB"));
    }
    // Firma (8) + longitud (4) + "IHDR" (4) + ancho (4) + alto (4).
    if bytes.len() < 24 || &bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err(CoreError::invalid("el archivo no es un PNG válido"));
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width != 64 || (height != 64 && height != 32) {
        return Err(CoreError::invalid(format!(
            "una skin debe medir 64×64 o 64×32 píxeles (esta mide {width}×{height})"
        )));
    }
    Ok(())
}

/// Cuerpo `multipart/form-data` con los campos `variant` y `file`. Se construye
/// a mano para no activar la feature `multipart` de reqwest solo por esto.
fn multipart_body(boundary: &str, variant: SkinVariant, png: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(png.len() + 512);
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"variant\"\r\n\r\n{}\r\n",
            variant.api_name()
        )
        .as_bytes(),
    );
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(png);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

async fn read_profile(resp: reqwest::Response, action: &str) -> Result<SkinProfile> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    if status.as_u16() == 401 {
        return Err(CoreError::Auth(
            "Minecraft rechazó la sesión; vuelve a iniciar sesión con Microsoft.".into(),
        ));
    }
    if status.as_u16() == 429 {
        return Err(CoreError::Network(
            "Mojang limita los cambios seguidos; espera un minuto y vuelve a intentarlo.".into(),
        ));
    }
    if !status.is_success() {
        let detail: String = text.chars().take(200).collect();
        return Err(CoreError::Network(format!("no se pudo {action} ({status}): {detail}")));
    }
    parse_profile(&text)
}

pub async fn fetch_skin_profile(
    client: &reqwest::Client,
    ep: &AuthEndpoints,
    token: &str,
) -> Result<SkinProfile> {
    let url = ep.check(&ep.mc_profile_url)?;
    let resp = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    read_profile(resp, "leer el perfil").await
}

pub async fn upload_skin(
    client: &reqwest::Client,
    ep: &AuthEndpoints,
    token: &str,
    variant: SkinVariant,
    png: &[u8],
) -> Result<SkinProfile> {
    validate_skin_png(png)?;
    let url = ep.check(&format!("{}/skins", ep.mc_profile_url))?;
    let boundary = format!("obsidia-{:016x}", rand::random::<u64>());
    let resp = client
        .post(url)
        .bearer_auth(token)
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(multipart_body(&boundary, variant, png))
        .send()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    read_profile(resp, "subir la skin").await
}

pub async fn reset_skin(
    client: &reqwest::Client,
    ep: &AuthEndpoints,
    token: &str,
) -> Result<SkinProfile> {
    let url = ep.check(&format!("{}/skins/active", ep.mc_profile_url))?;
    let resp = client
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    read_profile(resp, "restablecer la skin").await
}

/// `Some(id)` activa esa capa; `None` la oculta.
pub async fn set_cape(
    client: &reqwest::Client,
    ep: &AuthEndpoints,
    token: &str,
    cape_id: Option<&str>,
) -> Result<SkinProfile> {
    let url = ep.check(&format!("{}/capes/active", ep.mc_profile_url))?;
    let req = match cape_id {
        Some(id) => {
            if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                return Err(CoreError::invalid("identificador de capa no válido"));
            }
            client.put(url).json(&serde_json::json!({ "capeId": id }))
        }
        None => client.delete(url),
    };
    let resp = req
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    read_profile(resp, "cambiar la capa").await
}

// ───────────────────────────── Biblioteca local ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibrarySkin {
    /// SHA-1 del PNG: evita duplicados y sirve de nombre de archivo.
    pub id: String,
    pub name: String,
    pub variant: SkinVariant,
    pub added_at: u64,
}

fn library_index(dir: &Path) -> PathBuf {
    dir.join("library.json")
}

pub fn list_library(dir: &Path) -> Vec<LibrarySkin> {
    let Ok(bytes) = std::fs::read(library_index(dir)) else {
        return Vec::new();
    };
    let mut list: Vec<LibrarySkin> = serde_json::from_slice(&bytes).unwrap_or_default();
    list.retain(|s| is_skin_id(&s.id) && dir.join(format!("{}.png", s.id)).is_file());
    list
}

fn write_library(dir: &Path, list: &[LibrarySkin]) -> Result<()> {
    let path = library_index(dir);
    let json = serde_json::to_vec_pretty(list).map_err(|source| CoreError::Json {
        path: path.clone(),
        source,
    })?;
    filesystem::write_atomic(&path, &json)
}

fn is_skin_id(id: &str) -> bool {
    id.len() == 40 && id.chars().all(|c| c.is_ascii_hexdigit())
}

/// Guarda una skin en la biblioteca (o actualiza su nombre/variante si ya estaba).
pub fn save_to_library(
    dir: &Path,
    name: &str,
    variant: SkinVariant,
    png: &[u8],
    now: u64,
) -> Result<LibrarySkin> {
    validate_skin_png(png)?;
    filesystem::create_dir_all(dir)?;
    let id = filesystem::sha1_hex(png);
    filesystem::write_atomic(&dir.join(format!("{id}.png")), png)?;
    let clean: String = name
        .trim()
        .trim_end_matches(".png")
        .chars()
        .filter(|c| !c.is_control())
        .take(40)
        .collect();
    let entry = LibrarySkin {
        id: id.clone(),
        name: if clean.is_empty() { "Skin".into() } else { clean },
        variant,
        added_at: now,
    };
    let mut list = list_library(dir);
    list.retain(|s| s.id != id);
    list.insert(0, entry.clone());
    write_library(dir, &list)?;
    Ok(entry)
}

pub fn read_library_skin(dir: &Path, id: &str) -> Result<Vec<u8>> {
    if !is_skin_id(id) {
        return Err(CoreError::invalid("identificador de skin no válido"));
    }
    let path = dir.join(format!("{id}.png"));
    std::fs::read(&path).map_err(|e| CoreError::io(&path, e))
}

pub fn delete_from_library(dir: &Path, id: &str) -> Result<()> {
    if !is_skin_id(id) {
        return Err(CoreError::invalid("identificador de skin no válido"));
    }
    let mut list = list_library(dir);
    list.retain(|s| s.id != id);
    write_library(dir, &list)?;
    let _ = std::fs::remove_file(dir.join(format!("{id}.png")));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        v.extend_from_slice(&w.to_be_bytes());
        v.extend_from_slice(&h.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0]);
        v
    }

    #[test]
    fn validates_skin_dimensions() {
        assert!(validate_skin_png(&png(64, 64)).is_ok());
        assert!(validate_skin_png(&png(64, 32)).is_ok());
        assert!(validate_skin_png(&png(128, 128)).is_err());
        assert!(validate_skin_png(b"GIF89a....................").is_err());
        assert!(validate_skin_png(&vec![0u8; MAX_SKIN_BYTES + 1]).is_err());
    }

    #[test]
    fn parses_profile_with_skins_and_capes() {
        let text = r#"{
            "id": "abc", "name": "Steve",
            "skins": [{ "id": "s1", "state": "ACTIVE", "url": "http://textures.minecraft.net/texture/aa", "variant": "SLIM" }],
            "capes": [
                { "id": "c1", "state": "ACTIVE", "url": "http://textures.minecraft.net/texture/bb", "alias": "Migrator" },
                { "id": "c2", "state": "INACTIVE", "url": "http://textures.minecraft.net/texture/cc", "alias": "Pan" }
            ]
        }"#;
        let p = parse_profile(text).expect("perfil");
        assert_eq!(p.username, "Steve");
        let skin = p.active_skin().expect("skin activa");
        assert_eq!(skin.variant, SkinVariant::Slim);
        assert!(skin.url.starts_with("https://"));
        assert_eq!(p.capes.iter().filter(|c| c.active).count(), 1);
    }

    #[test]
    fn multipart_contains_both_fields() {
        let body = multipart_body("B", SkinVariant::Slim, b"PNGDATA");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("name=\"variant\"\r\n\r\nslim\r\n"));
        assert!(text.contains("filename=\"skin.png\""));
        assert!(text.contains("PNGDATA\r\n--B--"));
    }

    #[test]
    fn library_round_trip() {
        let dir = std::env::temp_dir().join(format!("obsidia-skins-{}", rand::random::<u64>()));
        let skin = png(64, 64);
        let saved = save_to_library(&dir, "mi_skin.png", SkinVariant::Classic, &skin, 1).expect("guardar");
        assert_eq!(saved.name, "mi_skin");
        // Repetir el mismo archivo no duplica la entrada.
        save_to_library(&dir, "otra", SkinVariant::Slim, &skin, 2).expect("guardar");
        let list = list_library(&dir);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].variant, SkinVariant::Slim);
        assert_eq!(read_library_skin(&dir, &saved.id).expect("leer"), skin);
        assert!(read_library_skin(&dir, "../../etc/passwd").is_err());
        delete_from_library(&dir, &saved.id).expect("borrar");
        assert!(list_library(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
