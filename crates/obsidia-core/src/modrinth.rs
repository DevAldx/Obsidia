//! Catálogo de Modrinth dentro del launcher: buscar mods y modpacks,
//! instalar un mod (con sus dependencias obligatorias) en una instancia e
//! instalar modpacks `.mrpack` (de Modrinth o elegidos desde el PC).
//!
//! Todo lo que se descarga se verifica con SHA-1 y tamaño con el mismo
//! `Downloader` que usan Mojang y Fabric, y solo desde los hosts que permite
//! el formato `.mrpack` (Modrinth y GitHub/GitLab).

use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    downloads::{is_valid, Downloader, Task},
    error::CoreError,
    filesystem::{self, join_safe},
    instances::{
        validate_loader_version, validate_minecraft_version, InstanceId, InstanceStore, Loader,
        Memory, NewInstance,
    },
    minecraft::Reporter,
    share_code::{self, USER_AGENT},
    ObsidiaPaths, Result,
};

const API: &str = "https://api.modrinth.com/v2";
const CDN_HOST: &str = "cdn.modrinth.com";
/// Hosts que admite el formato `.mrpack` para `files[].downloads`, más los
/// que usa GitHub para servir los adjuntos de una release tras redirigir.
const MRPACK_HOSTS: &[&str] = &[
    CDN_HOST,
    "github.com",
    "raw.githubusercontent.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
    "gitlab.com",
];
const MAX_SEARCH_LIMIT: u32 = 40;
/// Tope de proyectos que puede arrastrar un mod con sus dependencias.
const MAX_DEPENDENCIES: usize = 64;
const MAX_MRPACK_BYTES: usize = 256 * 1024 * 1024;
const MAX_MRPACK_FILES: usize = 2000;
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_OVERRIDE_ENTRIES: usize = 20_000;
const MAX_OVERRIDES_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 1024 * 1024 * 1024;

pub fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| CoreError::Network(e.to_string()))
}

fn net(e: reqwest::Error) -> CoreError {
    CoreError::Network(format!("Modrinth: {e}"))
}

/// IDs y slugs de Modrinth: se usan dentro de la ruta de la URL.
fn validate_id(id: &str) -> Result<&str> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && id != "."
        && id != "..";
    if ok {
        Ok(id)
    } else {
        Err(CoreError::invalid("identificador de Modrinth no válido"))
    }
}

/// GET con hasta 3 intentos: con una conexión inestable, Modrinth a veces corta
/// la respuesta a medias o tarda en contestar.
async fn get_json<T: serde::de::DeserializeOwned>(client: &reqwest::Client, url: reqwest::Url) -> Result<T> {
    let mut last = CoreError::Network("Modrinth: sin respuesta".into());
    for attempt in 0..3u64 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(500 * attempt)).await;
        }
        let resp = match client.get(url.clone()).timeout(Duration::from_secs(20)).send().await {
            Ok(r) => r,
            Err(e) => {
                last = net(e);
                continue;
            }
        };
        let status = resp.status();
        if status.as_u16() == 404 {
            return Err(CoreError::invalid("Modrinth no tiene ese proyecto o versión"));
        }
        if status.as_u16() == 429 || status.is_server_error() {
            last = CoreError::Network(format!("Modrinth respondió {status}"));
            continue;
        }
        let resp = resp.error_for_status().map_err(net)?;
        match resp.json().await {
            Ok(v) => return Ok(v),
            Err(e) => last = net(e), // respuesta cortada: se reintenta
        }
    }
    Err(last)
}

fn api_url(path: &str) -> Result<reqwest::Url> {
    reqwest::Url::parse(&format!("{API}{path}")).map_err(|e| CoreError::Network(e.to_string()))
}

// ───────────────────────────── Búsqueda ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    Modpack,
    Resourcepack,
    Shader,
}

impl ProjectType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Modpack => "modpack",
            Self::Resourcepack => "resourcepack",
            Self::Shader => "shader",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    pub query: String,
    pub project_type: ProjectType,
    /// Solo resultados que funcionan en esta versión de Minecraft.
    #[serde(default)]
    pub game_version: Option<String>,
    /// Solo resultados para este loader (los modpacks se filtran por Fabric,
    /// el único que sabe instalar Obsidia).
    #[serde(default)]
    pub loader: Option<Loader>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: Option<u32>,
    /// `relevance`, `downloads`, `follows`, `newest` o `updated`.
    #[serde(default)]
    pub sort: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    #[serde(default)]
    pub display_categories: Vec<String>,
    #[serde(default)]
    pub date_modified: Option<String>,
    #[serde(default)]
    pub featured_gallery: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u64,
}

fn facets(q: &SearchQuery) -> Result<String> {
    let mut groups: Vec<Vec<String>> = vec![vec![format!("project_type:{}", q.project_type.as_str())]];
    if let Some(v) = q.game_version.as_deref().filter(|v| !v.trim().is_empty()) {
        groups.push(vec![format!("versions:{}", validate_minecraft_version(v)?)]);
    }
    match (q.project_type, q.loader) {
        // Los modpacks que no son de Fabric no se pueden instalar.
        (ProjectType::Modpack, _) | (ProjectType::Mod, Some(Loader::Fabric)) => {
            groups.push(vec!["categories:fabric".into()]);
        }
        // Iris es el cargador de shaders de Fabric.
        (ProjectType::Shader, Some(Loader::Fabric)) => groups.push(vec!["categories:iris".into()]),
        _ => {}
    }
    if q.project_type == ProjectType::Mod {
        // Mods solo de servidor no hacen nada en el cliente.
        groups.push(vec!["client_side:required".into(), "client_side:optional".into()]);
    }
    serde_json::to_string(&groups).map_err(|e| CoreError::invalid(e.to_string()))
}

pub async fn search(client: &reqwest::Client, q: &SearchQuery) -> Result<SearchResults> {
    let sort = match q.sort.as_deref().unwrap_or("relevance") {
        s @ ("relevance" | "downloads" | "follows" | "newest" | "updated") => s,
        _ => return Err(CoreError::invalid("orden de búsqueda no válido")),
    };
    let limit = q.limit.unwrap_or(20).clamp(1, MAX_SEARCH_LIMIT);
    let mut url = api_url("/search")?;
    url.query_pairs_mut()
        .append_pair("query", q.query.trim())
        .append_pair("facets", &facets(q)?)
        .append_pair("index", sort)
        .append_pair("offset", &q.offset.min(10_000).to_string())
        .append_pair("limit", &limit.to_string());
    get_json(client, url).await
}

// ───────────────────────────── Versiones ─────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MrHashes {
    pub sha1: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MrFile {
    pub hashes: MrHashes,
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MrDependency {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MrVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub dependencies: Vec<MrDependency>,
    pub files: Vec<MrFile>,
}

impl MrVersion {
    fn primary_file(&self) -> Option<&MrFile> {
        self.files.iter().find(|f| f.primary).or_else(|| self.files.first())
    }

    fn fits(&self, game_version: &str, loader: &str) -> bool {
        self.game_versions.iter().any(|v| v == game_version) && self.loaders.iter().any(|l| l == loader)
    }
}

/// Versiones de un proyecto, de la más nueva a la más vieja, opcionalmente
/// filtradas por versión de Minecraft y loader.
pub async fn project_versions(
    client: &reqwest::Client,
    project: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<MrVersion>> {
    let mut url = api_url(&format!("/project/{}/version", validate_id(project)?))?;
    {
        let mut qp = url.query_pairs_mut();
        if let Some(v) = game_version {
            qp.append_pair("game_versions", &serde_json::json!([v]).to_string());
        }
        if let Some(l) = loader {
            qp.append_pair("loaders", &serde_json::json!([l]).to_string());
        }
    }
    get_json(client, url).await
}

async fn version(client: &reqwest::Client, id: &str) -> Result<MrVersion> {
    get_json(client, api_url(&format!("/version/{}", validate_id(id)?))?).await
}

/// La mejor versión: la release más nueva; si no hay, la más nueva de cualquier tipo.
fn pick_best(versions: Vec<MrVersion>) -> Option<MrVersion> {
    let release = versions.iter().position(|v| v.version_type == "release");
    let idx = release.unwrap_or(0);
    versions.into_iter().nth(idx)
}

/// Loader de Modrinth para los mods de una instancia.
fn mod_loader(loader: Loader) -> Result<&'static str> {
    match loader {
        Loader::Fabric => Ok("fabric"),
        Loader::Vanilla => Err(CoreError::invalid(
            "esta instancia es Vanilla: crea una instancia con Fabric para usar mods",
        )),
    }
}

// ───────────────────────────── Instalar un mod ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct InstalledFile {
    pub project_id: String,
    pub file_name: String,
    /// `false` si es una dependencia arrastrada por el mod pedido.
    pub requested: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct InstallOutcome {
    pub installed: Vec<InstalledFile>,
    /// Dependencias obligatorias que no tienen versión para esta instancia.
    pub missing_dependencies: Vec<String>,
}

/// Proyectos de Modrinth que ya están en `mods/` (activos o desactivados),
/// identificados por el SHA-1 de cada `.jar`.
pub async fn installed_projects(client: &reqwest::Client, mods_dir: &Path) -> Result<HashSet<String>> {
    let hashes: Vec<String> = share_code::hash_dir(mods_dir, &[".jar", ".jar.disabled"])
        .await?
        .into_iter()
        .map(|(_, h)| h)
        .collect();
    if hashes.is_empty() {
        return Ok(HashSet::new());
    }
    Ok(share_code::versions_by_hash(client, &hashes)
        .await?
        .into_values()
        .map(|v| v.project_id)
        .filter(|p| !p.is_empty())
        .collect())
}

fn safe_jar_name(name: &str) -> Result<String> {
    let base = Path::new(name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if base.is_empty() || base != name || !base.to_ascii_lowercase().ends_with(".jar") || base.len() > 255 {
        return Err(CoreError::invalid(format!("nombre de archivo no válido: {name}")));
    }
    Ok(base)
}

/// Instala `project` (y sus dependencias obligatorias que falten) en la
/// carpeta `mods/` de una instancia con `game_version` y `loader`.
pub async fn install_mod(
    client: &reqwest::Client,
    mods_dir: &Path,
    project: &str,
    game_version: &str,
    loader: Loader,
) -> Result<InstallOutcome> {
    let loader = mod_loader(loader)?;
    validate_minecraft_version(game_version)?;
    let mut have = installed_projects(client, mods_dir).await?;
    let dl = Downloader::new(&[CDN_HOST])?;
    let mut outcome = InstallOutcome::default();
    // (proyecto, versión concreta pedida por una dependencia)
    let mut queue: VecDeque<(String, Option<String>, bool)> = VecDeque::new();
    queue.push_back((project.to_owned(), None, true));
    let mut seen = HashSet::new();

    while let Some((project_id, version_id, requested)) = queue.pop_front() {
        if !seen.insert(project_id.clone()) || (!requested && have.contains(&project_id)) {
            continue;
        }
        if seen.len() > MAX_DEPENDENCIES {
            return Err(CoreError::invalid("este mod arrastra demasiadas dependencias"));
        }
        let mut chosen = None;
        if let Some(vid) = &version_id {
            let v = version(client, vid).await?;
            if v.fits(game_version, loader) {
                chosen = Some(v);
            }
        }
        if chosen.is_none() {
            chosen = pick_best(project_versions(client, &project_id, Some(game_version), Some(loader)).await?);
        }
        let Some(v) = chosen else {
            if requested {
                return Err(CoreError::invalid(format!(
                    "este mod no tiene ninguna versión para Minecraft {game_version} con {loader}"
                )));
            }
            outcome.missing_dependencies.push(project_id);
            continue;
        };
        // Modrinth admite slugs en la petición; a partir de aquí, el ID real.
        if requested && have.contains(&v.project_id) {
            return Err(CoreError::invalid("este mod ya está instalado en la instancia"));
        }
        let file = v
            .primary_file()
            .ok_or_else(|| CoreError::invalid("la versión no tiene archivos"))?;
        let file_name = safe_jar_name(&file.filename)?;
        let dest = mods_dir.join(&file_name);
        if !is_valid(dest.clone(), Some(file.hashes.sha1.clone()), Some(file.size)).await {
            if dest.exists() {
                return Err(CoreError::invalid(format!(
                    "ya hay otro archivo llamado {file_name} en la carpeta mods"
                )));
            }
            dl.download(
                &Task {
                    url: file.url.clone(),
                    dest,
                    sha1: Some(file.hashes.sha1.clone()),
                    sha256: None,
                    size: Some(file.size),
                },
                &|_| {},
            )
            .await?;
        }
        have.insert(v.project_id.clone());
        outcome.installed.push(InstalledFile {
            project_id: v.project_id.clone(),
            file_name,
            requested,
        });
        for dep in &v.dependencies {
            if dep.dependency_type != "required" {
                continue;
            }
            let dep_version = dep.version_id.clone();
            let dep_project = match (&dep.project_id, &dep_version) {
                (Some(p), _) => p.clone(),
                (None, Some(vid)) => version(client, vid).await?.project_id,
                (None, None) => continue,
            };
            queue.push_back((dep_project, dep_version, false));
        }
    }
    Ok(outcome)
}

// ───────────────────────────── Modpacks (.mrpack) ─────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrpackIndex {
    format_version: u32,
    game: String,
    #[serde(default)]
    version_id: String,
    name: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    files: Vec<MrpackFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrpackFile {
    path: String,
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<HashMap<String, String>>,
    downloads: Vec<String>,
    file_size: u64,
}

/// Archivo de un modpack ya validado y listo para descargar.
#[derive(Debug, Clone)]
struct PackFile {
    path: String,
    url: String,
    sha1: String,
    size: u64,
}

/// Lo que se enseña antes de instalar un modpack.
#[derive(Debug, Clone, Serialize)]
pub struct ModpackPreview {
    /// Clave del `.mrpack` guardado en caché: lo que se instala es
    /// exactamente lo que se previsualizó.
    pub token: String,
    pub name: String,
    pub summary: Option<String>,
    pub version: String,
    pub minecraft_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    pub mod_count: usize,
    pub file_count: usize,
    pub download_bytes: u64,
    pub override_count: usize,
    /// Archivos que no vienen de Modrinth (GitHub/GitLab).
    pub external_files: usize,
    pub icon_url: Option<String>,
}

/// Rutas que un modpack no puede tocar dentro de la instancia.
fn reserved_path(path: &str) -> bool {
    let first = path.split('/').next().unwrap_or_default().to_ascii_lowercase();
    first == "instance.json" || first.starts_with("background.") || first.starts_with("instance.json")
}

fn validate_pack_path(path: &str) -> Result<String> {
    let path = path.replace('\\', "/");
    let bad = path.is_empty()
        || path.len() > 512
        || path.starts_with('/')
        || path.contains('\0')
        || path.contains(':')
        || path.split('/').any(|p| p.is_empty() || p == "." || p == "..")
        || reserved_path(&path);
    if bad {
        return Err(CoreError::invalid(format!("el modpack trae una ruta no permitida: {path}")));
    }
    Ok(path)
}

/// Lee y valida `modrinth.index.json` y cuenta los overrides.
fn parse_mrpack(bytes: &[u8]) -> Result<(MrpackIndex, Vec<PackFile>, usize)> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| CoreError::invalid("el archivo no es un modpack .mrpack válido"))?;
    let index: MrpackIndex = {
        let entry = zip
            .by_name("modrinth.index.json")
            .map_err(|_| CoreError::invalid("al modpack le falta modrinth.index.json (¿es un .mrpack?)"))?;
        let mut text = Vec::new();
        entry
            .take(MAX_INDEX_BYTES)
            .read_to_end(&mut text)
            .map_err(|e| CoreError::invalid(format!("no se pudo leer el modpack: {e}")))?;
        serde_json::from_slice(&text).map_err(|e| CoreError::invalid(format!("modrinth.index.json no válido: {e}")))?
    };
    if index.format_version != 1 || index.game != "minecraft" {
        return Err(CoreError::invalid("formato de modpack no compatible"));
    }
    if index.files.len() > MAX_MRPACK_FILES {
        return Err(CoreError::invalid("el modpack declara demasiados archivos"));
    }
    let mut files = Vec::with_capacity(index.files.len());
    let mut total = 0u64;
    for f in &index.files {
        if f.env.as_ref().and_then(|e| e.get("client")).is_some_and(|c| c == "unsupported") {
            continue; // solo de servidor
        }
        let path = validate_pack_path(&f.path)?;
        let sha1 = f
            .hashes
            .get("sha1")
            .filter(|h| h.len() == 40 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| CoreError::invalid(format!("{path}: falta un SHA-1 válido")))?
            .to_ascii_lowercase();
        if f.file_size > MAX_FILE_BYTES {
            return Err(CoreError::invalid(format!("{path}: archivo demasiado grande")));
        }
        total += f.file_size;
        let url = f
            .downloads
            .iter()
            .find(|u| {
                reqwest::Url::parse(u).is_ok_and(|p| {
                    p.scheme() == "https" && p.host_str().is_some_and(|h| MRPACK_HOSTS.contains(&h))
                })
            })
            .ok_or_else(|| CoreError::invalid(format!("{path}: se descarga de un sitio no permitido")))?;
        files.push(PackFile {
            path,
            url: url.clone(),
            sha1,
            size: f.file_size,
        });
    }
    if total > 10 * MAX_FILE_BYTES {
        return Err(CoreError::invalid("el modpack es demasiado grande"));
    }
    let mut overrides = 0;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| CoreError::invalid(format!("modpack dañado: {e}")))?;
        if entry.is_file() && override_target(entry.name()).is_some() {
            overrides += 1;
        }
    }
    if overrides > MAX_OVERRIDE_ENTRIES {
        return Err(CoreError::invalid("el modpack trae demasiados archivos"));
    }
    Ok((index, files, overrides))
}

/// `overrides/x` y `client-overrides/x` → `x` (los de cliente pisan a los comunes).
fn override_target(name: &str) -> Option<(&str, bool)> {
    name.strip_prefix("overrides/")
        .map(|r| (r, false))
        .or_else(|| name.strip_prefix("client-overrides/").map(|r| (r, true)))
        .filter(|(r, _)| !r.is_empty())
}

fn pack_loader(deps: &HashMap<String, String>) -> Result<(String, Loader, Option<String>)> {
    let mc = deps
        .get("minecraft")
        .ok_or_else(|| CoreError::invalid("el modpack no indica la versión de Minecraft"))?;
    let mc = validate_minecraft_version(mc)?;
    for other in ["forge", "neoforge", "quilt-loader"] {
        if deps.contains_key(other) {
            let name = match other {
                "forge" => "Forge",
                "neoforge" => "NeoForge",
                _ => "Quilt",
            };
            return Err(CoreError::invalid(format!(
                "este modpack usa {name}; Obsidia por ahora solo instala modpacks de Fabric"
            )));
        }
    }
    match deps.get("fabric-loader") {
        Some(v) => Ok((mc, Loader::Fabric, Some(validate_loader_version(v)?))),
        None => Ok((mc, Loader::Vanilla, None)),
    }
}

fn cache_dir(paths: &ObsidiaPaths) -> PathBuf {
    paths.data_dir().join("cache").join("modpacks")
}

fn token_path(paths: &ObsidiaPaths, token: &str) -> Result<PathBuf> {
    if token.len() != 40 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(CoreError::invalid("modpack no válido; vuelve a elegirlo"));
    }
    Ok(cache_dir(paths).join(format!("{token}.mrpack")))
}

/// Guarda un `.mrpack` en la caché y devuelve su vista previa.
pub fn preview_bytes(paths: &ObsidiaPaths, bytes: &[u8], icon_url: Option<String>) -> Result<ModpackPreview> {
    if bytes.len() > MAX_MRPACK_BYTES {
        return Err(CoreError::invalid("el modpack es demasiado grande"));
    }
    let (index, files, overrides) = parse_mrpack(bytes)?;
    let (minecraft_version, loader, loader_version) = pack_loader(&index.dependencies)?;
    let token = filesystem::sha1_hex(bytes);
    let dir = cache_dir(paths);
    filesystem::create_dir_all(&dir)?;
    // Solo se guarda el último par de modpacks: limpia los anteriores.
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy() != format!("{token}.mrpack") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    filesystem::write_atomic(&token_path(paths, &token)?, bytes)?;
    Ok(ModpackPreview {
        token,
        name: index.name.chars().filter(|c| !c.is_control()).take(80).collect(),
        summary: share_code::clean_description(index.summary.as_deref()),
        version: index.version_id.chars().take(64).collect(),
        minecraft_version,
        loader,
        loader_version,
        mod_count: files.iter().filter(|f| f.path.starts_with("mods/")).count(),
        file_count: files.len(),
        download_bytes: files.iter().map(|f| f.size).sum(),
        override_count: overrides,
        external_files: files
            .iter()
            .filter(|f| reqwest::Url::parse(&f.url).ok().and_then(|u| u.host_str().map(str::to_owned)).as_deref() != Some(CDN_HOST))
            .count(),
        icon_url: icon_url.filter(|u| u.starts_with("https://")),
    })
}

/// Descarga el `.mrpack` de una versión de Modrinth y lo previsualiza.
pub async fn preview_version(
    client: &reqwest::Client,
    paths: &ObsidiaPaths,
    version_id: &str,
    icon_url: Option<String>,
) -> Result<ModpackPreview> {
    let v = version(client, version_id).await?;
    let file = v
        .files
        .iter()
        .find(|f| f.primary && f.filename.ends_with(".mrpack"))
        .or_else(|| v.files.iter().find(|f| f.filename.ends_with(".mrpack")))
        .ok_or_else(|| CoreError::invalid("esta versión no trae un archivo .mrpack"))?;
    if file.size > MAX_MRPACK_BYTES as u64 {
        return Err(CoreError::invalid("el modpack es demasiado grande"));
    }
    let dl = Downloader::new(&[CDN_HOST])?;
    let bytes = dl.fetch_bytes(&file.url, MAX_MRPACK_BYTES).await?;
    if bytes.len() as u64 != file.size || !filesystem::sha1_hex(&bytes).eq_ignore_ascii_case(&file.hashes.sha1) {
        return Err(CoreError::Integrity("el modpack descargado no coincide con Modrinth".into()));
    }
    preview_bytes(paths, &bytes, icon_url)
}

/// Nombre de instancia a partir del nombre del modpack (máx. 48 caracteres).
fn instance_name(name: &str) -> String {
    let clean: String = name.chars().filter(|c| !c.is_control()).collect();
    let clean = clean.trim();
    let short: String = clean.chars().take(48).collect();
    if short.trim().is_empty() {
        "Modpack".into()
    } else {
        short.trim().to_owned()
    }
}

/// Versión de Minecraft y loader del modpack en caché (para preparar el juego antes de instalar).
pub fn cached_requirements(paths: &ObsidiaPaths, token: &str) -> Result<(String, Loader, Option<String>)> {
    let bytes = std::fs::read(token_path(paths, token)?)
        .map_err(|_| CoreError::invalid("el modpack ya no está en caché; vuelve a elegirlo"))?;
    let (index, _, _) = parse_mrpack(&bytes)?;
    pack_loader(&index.dependencies)
}

/// Crea la instancia del modpack en caché `token`: descarga sus archivos
/// y extrae los overrides. Si algo falla, borra la instancia a medias.
pub async fn install_modpack(
    paths: &ObsidiaPaths,
    instances: &InstanceStore,
    token: &str,
    reporter: &Reporter,
) -> Result<InstanceId> {
    let path = token_path(paths, token)?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| CoreError::invalid("el modpack ya no está en caché; vuelve a elegirlo"))?;
    if !filesystem::sha1_hex(&bytes).eq_ignore_ascii_case(token) {
        return Err(CoreError::Integrity("el modpack en caché está dañado".into()));
    }
    let (index, files, _) = parse_mrpack(&bytes)?;
    let (minecraft_version, loader, loader_version) = pack_loader(&index.dependencies)?;

    let created = instances.create(NewInstance {
        name: instance_name(&index.name),
        minecraft_version,
        loader,
        loader_version,
        memory: Memory {
            min_mb: 1024,
            max_mb: 4096,
        },
        jvm_args: Vec::new(),
    })?;
    let id = created.instance.id;
    let dir = paths.instance_dir(&id);

    let result = async {
        let dl = Downloader::new(MRPACK_HOSTS)?;
        let mut pending = Vec::new();
        for f in &files {
            let dest = join_safe(&dir, &f.path)?;
            pending.push(Task {
                url: f.url.clone(),
                dest,
                sha1: Some(f.sha1.clone()),
                sha256: None,
                size: Some(f.size),
            });
        }
        reporter.phase("Descargando mods del modpack", pending.iter().filter_map(|t| t.size).sum());
        for task in &pending {
            dl.download(task, &|d| reporter.add(d)).await?;
        }

        reporter.phase("Copiando la configuración del modpack", 0);
        let dir2 = dir.clone();
        tokio::task::spawn_blocking(move || extract_overrides(&bytes, &dir2))
            .await
            .map_err(|e| CoreError::invalid(e.to_string()))??;
        Ok::<(), CoreError>(())
    }
    .await;

    if let Err(e) = result {
        let _ = instances.delete(&id);
        return Err(e);
    }
    let _ = std::fs::remove_file(&path);
    Ok(id)
}

fn extract_overrides(bytes: &[u8], dir: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| CoreError::invalid("el modpack está dañado"))?;
    // Primero los comunes y después los de cliente, que tienen prioridad.
    let mut order: Vec<(usize, bool)> = Vec::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| CoreError::invalid(format!("modpack dañado: {e}")))?;
        if let Some((_, client)) = override_target(entry.name()).filter(|_| entry.is_file()) {
            order.push((i, client));
        }
    }
    order.sort_by_key(|(_, client)| *client);
    let mut written = 0u64;
    for (i, _) in order {
        let mut entry = zip.by_index(i).map_err(|e| CoreError::invalid(format!("modpack dañado: {e}")))?;
        let name = entry.name().to_owned();
        let Some((rel, _)) = override_target(&name) else { continue };
        let rel = validate_pack_path(rel)?;
        let dest = join_safe(dir, &rel)?;
        let mut data = Vec::new();
        let remaining = MAX_OVERRIDES_BYTES.saturating_sub(written);
        (&mut entry)
            .take(remaining + 1)
            .read_to_end(&mut data)
            .map_err(|e| CoreError::invalid(format!("no se pudo leer {rel}: {e}")))?;
        written += data.len() as u64;
        if written > MAX_OVERRIDES_BYTES {
            return Err(CoreError::invalid("el modpack ocupa demasiado al descomprimirlo"));
        }
        if let Some(parent) = dest.parent() {
            filesystem::create_dir_all(parent)?;
        }
        filesystem::write_atomic(&dest, &data)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instances::test_util::TempDir;
    use std::io::Write;

    fn mrpack(index: &serde_json::Value, extra: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("modrinth.index.json", opts).unwrap();
            zip.write_all(index.to_string().as_bytes()).unwrap();
            for (name, data) in extra {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    fn index(files: serde_json::Value, deps: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "formatVersion": 1, "game": "minecraft", "versionId": "1.0.0",
            "name": "Pack de prueba", "summary": "Un pack", "files": files, "dependencies": deps
        })
    }

    fn file(path: &str, url: &str) -> serde_json::Value {
        serde_json::json!({
            "path": path, "hashes": {"sha1": "a".repeat(40), "sha512": "b"},
            "env": {"client": "required", "server": "required"},
            "downloads": [url], "fileSize": 10
        })
    }

    #[test]
    fn preview_reads_index_and_counts_overrides() {
        let tmp = TempDir::new();
        let paths = ObsidiaPaths::new(tmp.path().join("c"), tmp.path().join("d"));
        let files = serde_json::json!([
            file("mods/sodium.jar", "https://cdn.modrinth.com/data/x/sodium.jar"),
            file("mods/gh.jar", "https://github.com/a/b/releases/download/1/gh.jar"),
            {
                "path": "mods/server-only.jar", "hashes": {"sha1": "c".repeat(40)},
                "env": {"client": "unsupported", "server": "required"},
                "downloads": ["https://cdn.modrinth.com/s.jar"], "fileSize": 1
            }
        ]);
        let bytes = mrpack(
            &index(files, serde_json::json!({"minecraft": "1.21.1", "fabric-loader": "0.16.5"})),
            &[("overrides/config/a.json", b"{}"), ("client-overrides/options.txt", b"x")],
        );
        let p = preview_bytes(&paths, &bytes, None).unwrap();
        assert_eq!(p.minecraft_version, "1.21.1");
        assert_eq!(p.loader, Loader::Fabric);
        assert_eq!(p.loader_version.as_deref(), Some("0.16.5"));
        assert_eq!(p.mod_count, 2, "los de solo servidor no cuentan");
        assert_eq!(p.external_files, 1);
        assert_eq!(p.override_count, 2);
        assert!(token_path(&paths, &p.token).unwrap().is_file());
        assert_eq!(cached_requirements(&paths, &p.token).unwrap().0, "1.21.1");
    }

    #[test]
    fn rejects_other_loaders_bad_paths_and_foreign_hosts() {
        let tmp = TempDir::new();
        let paths = ObsidiaPaths::new(tmp.path().join("c"), tmp.path().join("d"));
        let forge = mrpack(&index(serde_json::json!([]), serde_json::json!({"minecraft": "1.20.1", "forge": "47"})), &[]);
        assert!(preview_bytes(&paths, &forge, None).unwrap_err().to_string().contains("Forge"));

        let deps = serde_json::json!({"minecraft": "1.21.1", "fabric-loader": "0.16.5"});
        for (path, url) in [
            ("../evil.jar", "https://cdn.modrinth.com/a.jar"),
            ("/etc/x", "https://cdn.modrinth.com/a.jar"),
            ("instance.json", "https://cdn.modrinth.com/a.jar"),
            ("mods/a.jar", "https://evil.example/a.jar"),
            ("mods/a.jar", "http://cdn.modrinth.com/a.jar"),
        ] {
            let bytes = mrpack(&index(serde_json::json!([file(path, url)]), deps.clone()), &[]);
            assert!(preview_bytes(&paths, &bytes, None).is_err(), "{path} {url}");
        }
        assert!(preview_bytes(&paths, b"no es un zip", None).is_err());
        assert!(token_path(&paths, "../../x").is_err());
    }

    #[test]
    fn overrides_are_extracted_with_client_priority_and_safe_paths() {
        let tmp = TempDir::new();
        let dir = tmp.path().join("inst");
        std::fs::create_dir_all(&dir).unwrap();
        let deps = serde_json::json!({"minecraft": "1.21.1"});
        let bytes = mrpack(
            &index(serde_json::json!([]), deps),
            &[
                ("overrides/options.txt", b"comun"),
                ("client-overrides/options.txt", b"cliente"),
                ("overrides/config/sodium.json", b"{}"),
            ],
        );
        extract_overrides(&bytes, &dir).unwrap();
        assert_eq!(std::fs::read(dir.join("options.txt")).unwrap(), b"cliente");
        assert_eq!(std::fs::read(dir.join("config/sodium.json")).unwrap(), b"{}");

        let evil = mrpack(&index(serde_json::json!([]), serde_json::json!({"minecraft": "1.21.1"})), &[("overrides/instance.json", b"{}")]);
        assert!(extract_overrides(&evil, &dir).is_err());
    }

    #[test]
    fn facets_filter_by_type_version_and_loader() {
        let q = SearchQuery {
            query: "sodium".into(),
            project_type: ProjectType::Mod,
            game_version: Some("1.21.1".into()),
            loader: Some(Loader::Fabric),
            offset: 0,
            limit: None,
            sort: None,
        };
        let f = facets(&q).unwrap();
        assert!(f.contains("project_type:mod") && f.contains("versions:1.21.1") && f.contains("categories:fabric"));
        let bad = SearchQuery { game_version: Some("1.21\"]".into()), ..q };
        assert!(facets(&bad).is_err());
    }

    #[test]
    fn ids_and_jar_names_are_validated() {
        assert!(validate_id("AANobbMI").is_ok());
        assert!(validate_id("fabric-api").is_ok());
        assert!(validate_id("../x").is_err());
        assert!(validate_id("a/b").is_err());
        assert!(safe_jar_name("sodium-0.6.jar").is_ok());
        assert!(safe_jar_name("../sodium.jar").is_err());
        assert!(safe_jar_name("sodium.zip").is_err());
    }

    #[test]
    fn pick_best_prefers_newest_release() {
        let v = |id: &str, t: &str| MrVersion {
            id: id.into(),
            project_id: "p".into(),
            name: id.into(),
            version_number: id.into(),
            version_type: t.into(),
            game_versions: vec![],
            loaders: vec![],
            date_published: String::new(),
            dependencies: vec![],
            files: vec![],
        };
        assert_eq!(pick_best(vec![v("b", "beta"), v("r", "release")]).unwrap().id, "r");
        assert_eq!(pick_best(vec![v("b", "beta"), v("a", "alpha")]).unwrap().id, "b");
        assert!(pick_best(vec![]).is_none());
    }
}
