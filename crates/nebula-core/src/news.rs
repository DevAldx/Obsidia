//! Noticias y notas de versión de Minecraft: Java Edition para la pantalla de
//! inicio.
//!
//! Se leen los mismos feeds públicos que usa el launcher oficial
//! (`launchercontent.mojang.com`). Cada entrada se valida por separado: si
//! Mojang añade un campo raro a una noticia, se descarta esa noticia y no el
//! feed entero. El resultado se guarda en disco para mostrar algo sin conexión.

use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{filesystem, CoreError, Result};

const CONTENT_HOST: &str = "https://launchercontent.mojang.com";
const NEWS_URL: &str = "https://launchercontent.mojang.com/v2/news.json";
const PATCH_NOTES_URL: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";
const JAVA_CATEGORY: &str = "Minecraft: Java Edition";
const MAX_NEWS: usize = 12;
const MAX_PATCH_NOTES: usize = 8;
/// Tiempo que una copia descargada se considera fresca.
pub const FRESH_FOR: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NewsKind {
    News,
    Release,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewsItem {
    pub id: String,
    pub kind: NewsKind,
    pub title: String,
    pub summary: String,
    /// `AAAA-MM-DD`.
    pub date: String,
    /// Siempre `https://launchercontent.mojang.com/...` (lo exige la CSP).
    pub image_url: Option<String>,
    /// Solo enlaces https.
    pub link: Option<String>,
    /// Versión de Minecraft para las notas de versión.
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewsFeed {
    /// Segundos UNIX de la descarga.
    pub fetched_at: u64,
    pub items: Vec<NewsItem>,
    /// `true` si viene de la caché porque la red falló.
    #[serde(default)]
    pub stale: bool,
}

impl NewsFeed {
    pub fn is_fresh(&self) -> bool {
        now_secs().saturating_sub(self.fetched_at) < FRESH_FOR.as_secs()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

/// Las imágenes del feed son rutas relativas a `launchercontent.mojang.com`.
fn image_url(v: Option<&Value>) -> Option<String> {
    let url = v?.get("url")?.as_str()?.trim();
    if url.starts_with('/') && !url.starts_with("//") {
        Some(format!("{CONTENT_HOST}{url}"))
    } else if url.starts_with(CONTENT_HOST) && url[CONTENT_HOST.len()..].starts_with('/') {
        Some(url.to_owned())
    } else {
        None
    }
}

fn https_link(v: Option<&str>) -> Option<String> {
    let url = reqwest::Url::parse(v?).ok()?;
    (url.scheme() == "https").then(|| url.to_string())
}

fn day(date: &str) -> Option<String> {
    let d = date.get(..10)?;
    let ok = d.len() == 10
        && d.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        });
    ok.then(|| d.to_owned())
}

/// Noticias de Java Edition de `news.json`.
pub fn parse_news(feed: &Value) -> Vec<NewsItem> {
    let Some(entries) = feed.get("entries").and_then(Value::as_array) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|e| {
            str_field(e, "category") == Some(JAVA_CATEGORY)
                || e.get("newsType")
                    .and_then(Value::as_array)
                    .is_some_and(|t| t.iter().any(|x| x.as_str() == Some("Java")))
        })
        .filter_map(|e| {
            Some(NewsItem {
                id: str_field(e, "id")?.to_owned(),
                kind: NewsKind::News,
                title: str_field(e, "title")?.to_owned(),
                summary: str_field(e, "text").unwrap_or_default().to_owned(),
                date: day(str_field(e, "date")?)?,
                image_url: image_url(e.get("newsPageImage"))
                    .or_else(|| image_url(e.get("playPageImage"))),
                link: https_link(str_field(e, "readMoreLink")),
                version: None,
            })
        })
        .take(MAX_NEWS)
        .collect()
}

/// Releases y snapshots de `javaPatchNotes.json`.
pub fn parse_patch_notes(feed: &Value) -> Vec<NewsItem> {
    let Some(entries) = feed.get("entries").and_then(Value::as_array) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|e| {
            let kind = match str_field(e, "type")? {
                "release" => NewsKind::Release,
                "snapshot" => NewsKind::Snapshot,
                _ => return None,
            };
            Some(NewsItem {
                id: format!("patch-{}", str_field(e, "id")?),
                kind,
                title: str_field(e, "title")?.to_owned(),
                summary: str_field(e, "shortText").unwrap_or_default().to_owned(),
                date: day(str_field(e, "date")?)?,
                image_url: image_url(e.get("image")),
                link: None,
                version: str_field(e, "version").map(str::to_owned),
            })
        })
        .take(MAX_PATCH_NOTES)
        .collect()
}

async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    let res = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| CoreError::Network(e.to_string()))?;
    res.json::<Value>()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))
}

/// Descarga ambos feeds. Basta con que uno responda.
pub async fn fetch(client: &reqwest::Client) -> Result<NewsFeed> {
    let (news, notes) = tokio::join!(get_json(client, NEWS_URL), get_json(client, PATCH_NOTES_URL));
    if let (Err(e), Err(_)) = (&news, &notes) {
        return Err(CoreError::Network(e.to_string()));
    }
    let mut items = Vec::new();
    if let Ok(v) = &news {
        items.extend(parse_news(v));
    }
    if let Ok(v) = &notes {
        items.extend(parse_patch_notes(v));
    }
    items.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(NewsFeed {
        fetched_at: now_secs(),
        items,
        stale: false,
    })
}

pub fn load_cache(path: &Path) -> Option<NewsFeed> {
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn save_cache(path: &Path, feed: &NewsFeed) -> Result<()> {
    let bytes = serde_json::to_vec(feed).map_err(|source| CoreError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    if let Some(parent) = path.parent() {
        filesystem::create_dir_all(parent)?;
    }
    filesystem::write_atomic(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_java_news_and_skips_other_editions() {
        let feed = json!({ "entries": [
            {
                "id": "a", "title": "Halloween", "text": "Mapas", "category": "Minecraft: Java Edition",
                "date": "2026-10-30",
                "newsPageImage": { "url": "/v2/images/a.jpg" },
                "readMoreLink": "https://www.minecraft.net/article/a"
            },
            { "id": "b", "title": "Dungeons", "category": "Minecraft Dungeons", "date": "2026-10-29" },
            { "id": "c", "title": "Sin fecha", "category": "Minecraft: Java Edition" },
            {
                "id": "d", "title": "Imagen ajena", "category": "Minecraft: Java Edition",
                "date": "2026-10-28", "newsPageImage": { "url": "https://evil.example/x.jpg" },
                "readMoreLink": "http://inseguro.example"
            }
        ]});
        let items = parse_news(&feed);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].image_url.as_deref(), Some("https://launchercontent.mojang.com/v2/images/a.jpg"));
        assert_eq!(items[0].link.as_deref(), Some("https://www.minecraft.net/article/a"));
        assert_eq!(items[1].image_url, None);
        assert_eq!(items[1].link, None);
    }

    #[test]
    fn parses_patch_notes() {
        let feed = json!({ "entries": [
            { "id": "1-21", "title": "Minecraft 1.21", "version": "1.21", "type": "release",
              "date": "2024-06-13T10:00:00.000Z", "image": { "url": "/v2/images/r.jpg" } },
            { "id": "x", "title": "Raro", "type": "beta", "date": "2024-06-13" }
        ]});
        let items = parse_patch_notes(&feed);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, NewsKind::Release);
        assert_eq!(items[0].date, "2024-06-13");
        assert_eq!(items[0].version.as_deref(), Some("1.21"));
    }

    #[test]
    fn rejects_protocol_relative_images() {
        assert_eq!(image_url(Some(&json!({ "url": "//evil.example/x.jpg" }))), None);
        assert_eq!(image_url(Some(&json!({ "url": "https://launchercontent.mojang.com.evil/x" }))), None);
    }

    #[test]
    fn cache_round_trip() {
        let dir = std::env::temp_dir().join(format!("obsidia-news-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("news.json");
        let feed = NewsFeed { fetched_at: 1, items: Vec::new(), stale: false };
        assert!(save_cache(&path, &feed).is_ok());
        assert_eq!(load_cache(&path).map(|f| f.fetched_at), Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
