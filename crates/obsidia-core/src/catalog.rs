//! Catálogo comunitario: packs y mods que publican los propios jugadores en
//! el servidor para compartir (`server/`), visibles para todos en Descubrir y
//! en el explorador de mods de cada instancia.
//!
//! - Un **pack** del catálogo es un código `OBS-XXXXXX` ya subido con
//!   «Compartir»; se instala con el flujo de importar por código de siempre.
//! - Un **mod** del catálogo es un `.jar` que no está en Modrinth, alojado en
//!   el servidor por su SHA-1. Se descarga verificando hash y tamaño.
//!
//! Para publicar o borrar hace falta la misma sesión que para compartir
//! (certificado de jugador de Mojang); para listar e instalar, nada.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    downloads::{is_valid, sha1_file, Downloader, Task},
    error::CoreError,
    instances::Loader,
    share_code,
    share_upload::{check, exists_on_server, http_client, login, net, normalize_base, upload_file, Identity},
    Result,
};

/// Igual que el límite del servidor con Workers KV.
const MAX_MOD_BYTES: u64 = 25 * 1024 * 1024;
const MAX_CATALOG_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogFile {
    pub file_name: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatalogEntry {
    pub id: String,
    /// `pack` o `mod`.
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Nombre de Minecraft que declaró quien lo publicó.
    #[serde(default)]
    pub author: String,
    /// UUID (sin guiones) de la cuenta que lo publicó; este sí lo verifica el servidor.
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub created: String,
    pub minecraft_version: String,
    pub loader: Loader,
    #[serde(default)]
    pub loader_version: Option<String>,
    /// Packs: código para importar.
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub mod_count: Option<u32>,
    /// Mods: el archivo alojado.
    #[serde(default)]
    pub file: Option<CatalogFile>,
}

#[derive(Deserialize)]
struct CatalogList {
    items: Vec<CatalogEntry>,
}

/// Todo el catálogo (el servidor lo devuelve del más nuevo al más viejo).
pub async fn list(base_url: &str) -> Result<Vec<CatalogEntry>> {
    let base = normalize_base(base_url)?;
    let client = http_client()?;
    let resp = check(
        client
            .get(format!("{base}/api/catalog"))
            .send()
            .await
            .map_err(net("catálogo de la comunidad"))?,
        "catálogo de la comunidad",
    )
    .await?;
    let bytes = resp.bytes().await.map_err(net("catálogo de la comunidad"))?;
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(CoreError::invalid("el catálogo es demasiado grande"));
    }
    let list: CatalogList =
        serde_json::from_slice(&bytes).map_err(|e| CoreError::invalid(format!("catálogo no válido: {e}")))?;
    Ok(list.items)
}

async fn post_entry(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    body: serde_json::Value,
) -> Result<CatalogEntry> {
    check(
        client
            .post(format!("{base}/api/catalog"))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(net("catálogo de la comunidad"))?,
        "publicar en la comunidad",
    )
    .await?
    .json()
    .await
    .map_err(net("catálogo de la comunidad"))
}

/// Publica en el catálogo un pack ya subido con «Compartir» (código `OBS-XXXXXX`).
pub async fn publish_pack(
    base_url: &str,
    identity: &Identity,
    code: &str,
    description: Option<&str>,
) -> Result<CatalogEntry> {
    let code = crate::share::validate_code(code)?;
    let base = normalize_base(base_url)?;
    let client = http_client()?;
    let token = login(&client, &base, identity).await?;
    post_entry(
        &client,
        &base,
        &token,
        serde_json::json!({
            "kind": "pack",
            "code": code,
            "author": identity.username,
            "description": share_code::clean_description(description),
        }),
    )
    .await
}

pub struct ModUpload<'a> {
    pub mods_dir: &'a Path,
    pub file_name: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub minecraft_version: &'a str,
    pub loader: Loader,
}

/// Sube un `.jar` de la instancia y lo publica en el catálogo.
pub async fn publish_mod(base_url: &str, identity: &Identity, input: ModUpload<'_>) -> Result<CatalogEntry> {
    let base = normalize_base(base_url)?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 48 {
        return Err(CoreError::invalid("el nombre debe tener entre 1 y 48 caracteres"));
    }
    if input.loader != Loader::Fabric {
        return Err(CoreError::invalid("solo se pueden publicar mods de instancias con Fabric"));
    }
    let lower = input.file_name.to_ascii_lowercase();
    if input.file_name.contains(['/', '\\']) || !(lower.ends_with(".jar") || lower.ends_with(".jar.disabled")) {
        return Err(CoreError::invalid("solo se pueden publicar archivos .jar"));
    }
    let path = input.mods_dir.join(input.file_name);
    let file_name = input.file_name.strip_suffix(".disabled").unwrap_or(input.file_name).to_owned();
    let (sha1, size) = {
        let path = path.clone();
        tokio::task::spawn_blocking(move || -> Result<(String, u64)> {
            let size = std::fs::metadata(&path).map_err(|e| CoreError::io(&path, e))?.len();
            let sha1 = sha1_file(&path).map_err(|e| CoreError::io(&path, e))?;
            Ok((sha1.to_ascii_lowercase(), size))
        })
        .await
        .map_err(|e| CoreError::invalid(e.to_string()))??
    };
    if size > MAX_MOD_BYTES {
        return Err(CoreError::invalid("el mod supera los 25 MB que admite el servidor"));
    }

    let client = http_client()?;
    // Lo que ya está en Modrinth se instala desde allí: no tiene sentido
    // (ni suele estar permitido) volver a alojarlo.
    let modrinth = crate::modrinth::http_client()?;
    if share_code::versions_by_hash(&modrinth, std::slice::from_ref(&sha1))
        .await?
        .contains_key(&sha1)
    {
        return Err(CoreError::invalid(
            "este mod ya está en Modrinth: los demás pueden instalarlo desde la pestaña Modrinth",
        ));
    }

    let token = login(&client, &base, identity).await?;
    if !exists_on_server(&client, &base, &sha1).await? {
        let bytes = tokio::fs::read(&path).await.map_err(|e| CoreError::io(&path, e))?;
        if bytes.len() as u64 != size {
            return Err(CoreError::invalid("el archivo cambió mientras se subía; vuelve a intentarlo"));
        }
        upload_file(&client, &base, &token, &file_name, &sha1, bytes).await?;
    }
    post_entry(
        &client,
        &base,
        &token,
        serde_json::json!({
            "kind": "mod",
            "name": name,
            "author": identity.username,
            "description": share_code::clean_description(input.description),
            "minecraft_version": input.minecraft_version,
            "loader": input.loader,
            "file": { "file_name": file_name, "sha1": sha1, "size": size },
        }),
    )
    .await
}

/// Borra una entrada del catálogo (solo su dueño).
pub async fn delete(base_url: &str, identity: &Identity, id: &str) -> Result<()> {
    if id.is_empty() || id.len() > 32 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(CoreError::invalid("entrada del catálogo no válida"));
    }
    let base = normalize_base(base_url)?;
    let client = http_client()?;
    let token = login(&client, &base, identity).await?;
    check(
        client
            .delete(format!("{base}/api/catalog/{id}"))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(net("catálogo de la comunidad"))?,
        "borrar de la comunidad",
    )
    .await?;
    Ok(())
}

/// Descarga un mod del catálogo a `mods/`, verificando SHA-1 y tamaño.
pub async fn install_mod(base_url: &str, mods_dir: &Path, file: &CatalogFile) -> Result<String> {
    let base = normalize_base(base_url)?;
    let name = &file.file_name;
    let ok_name = !name.is_empty()
        && name.len() <= 255
        && !name.contains(['/', '\\', '\0'])
        && name != ".."
        && name.to_ascii_lowercase().ends_with(".jar");
    if !ok_name {
        return Err(CoreError::invalid("nombre de mod no válido"));
    }
    if file.sha1.len() != 40 || !file.sha1.bytes().all(|b| b.is_ascii_hexdigit()) || file.size > MAX_MOD_BYTES {
        return Err(CoreError::invalid("entrada del catálogo no válida"));
    }
    let dest = mods_dir.join(name);
    if is_valid(dest.clone(), Some(file.sha1.clone()), Some(file.size)).await {
        return Err(CoreError::invalid("este mod ya está instalado en la instancia"));
    }
    if dest.exists() {
        return Err(CoreError::invalid(format!("ya hay otro archivo llamado {name} en la carpeta mods")));
    }
    let host = reqwest::Url::parse(&base)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .ok_or_else(|| CoreError::invalid("URL del servidor no válida"))?;
    let dl = Downloader::new(&[host.as_str()])?;
    dl.download(
        &Task {
            url: format!("{base}/files/{}", file.sha1.to_ascii_lowercase()),
            dest,
            sha1: Some(file.sha1.clone()),
            sha256: None,
            size: Some(file.size),
        },
        &|_| {},
    )
    .await?;
    Ok(name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_entries_parse_with_optional_fields() {
        let json = r#"{"items":[
            {"id":"a1","kind":"pack","name":"Pack","minecraft_version":"1.21.1","loader":"fabric","code":"OBS-K7P2X9"},
            {"id":"b2","kind":"mod","name":"Mod","minecraft_version":"1.21.1","loader":"fabric",
             "file":{"file_name":"x.jar","sha1":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size":3}}
        ]}"#;
        let list: CatalogList = serde_json::from_str(json).unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].code.as_deref(), Some("OBS-K7P2X9"));
        assert_eq!(list.items[1].file.as_ref().unwrap().size, 3);
    }

    #[tokio::test]
    async fn install_mod_rejects_bad_names_and_hashes() {
        let dir = std::env::temp_dir();
        let bad = [
            CatalogFile { file_name: "../x.jar".into(), sha1: "a".repeat(40), size: 1 },
            CatalogFile { file_name: "x.sh".into(), sha1: "a".repeat(40), size: 1 },
            CatalogFile { file_name: "x.jar".into(), sha1: "zz".into(), size: 1 },
        ];
        for f in &bad {
            assert!(install_mod("https://x.example", &dir, f).await.is_err(), "{}", f.file_name);
        }
    }
}
