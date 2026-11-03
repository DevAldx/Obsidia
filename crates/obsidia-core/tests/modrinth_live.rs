//! Prueba manual de red contra la API real de Modrinth: buscar, instalar un
//! mod con sus dependencias y previsualizar e instalar un modpack. No forma
//! parte de la suite normal; se ejecuta a mano con
//! `cargo test -p obsidia-core --test modrinth_live -- --ignored --nocapture`.

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use obsidia_core::{
    instances::{InstanceStore, Loader},
    minecraft::Reporter,
    modrinth::{self, ProjectType, SearchQuery},
    ObsidiaPaths,
};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("obsidia-modrinth-live-{tag}-{stamp}"));
    fs::create_dir_all(&root).unwrap();
    root
}

#[tokio::test]
#[ignore = "requiere red; ejecutar a mano"]
async fn searches_and_installs_a_mod_with_dependencies() {
    let client = modrinth::http_client().unwrap();
    let results = modrinth::search(
        &client,
        &SearchQuery {
            query: "sodium extra".into(),
            project_type: ProjectType::Mod,
            game_version: Some("1.21.1".into()),
            loader: Some(Loader::Fabric),
            offset: 0,
            limit: Some(5),
            sort: None,
        },
    )
    .await
    .unwrap();
    println!("{} resultados; primero: {}", results.total_hits, results.hits[0].title);
    let hit = results.hits.iter().find(|h| h.slug == "sodium-extra").expect("sodium-extra en resultados");

    let root = temp_root("mod");
    let mods = root.join("mods");
    let out = modrinth::install_mod(&client, &mods, &hit.project_id, "1.21.1", Loader::Fabric)
        .await
        .unwrap();
    println!("{out:#?}");
    assert!(out.installed.iter().any(|f| f.requested));
    // Sodium Extra necesita Sodium.
    assert!(out.installed.iter().any(|f| !f.requested), "debe arrastrar dependencias");
    for f in &out.installed {
        assert!(mods.join(&f.file_name).is_file());
    }

    let installed = modrinth::installed_projects(&client, &mods).await.unwrap();
    assert!(installed.contains(&hit.project_id));
    let again = modrinth::install_mod(&client, &mods, &hit.project_id, "1.21.1", Loader::Fabric).await;
    assert!(again.is_err(), "no se instala dos veces");
    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
#[ignore = "requiere red; ejecutar a mano"]
async fn previews_and_installs_a_real_modpack() {
    let client = modrinth::http_client().unwrap();
    let results = modrinth::search(
        &client,
        &SearchQuery {
            query: "Fabulously Optimized".into(),
            project_type: ProjectType::Modpack,
            game_version: None,
            loader: Some(Loader::Fabric),
            offset: 0,
            limit: Some(5),
            sort: None,
        },
    )
    .await
    .unwrap();
    let hit = results.hits.iter().find(|h| h.slug == "fabulously-optimized").expect("FO en resultados");
    let versions = modrinth::project_versions(&client, &hit.project_id, None, None).await.unwrap();
    let v = versions.iter().find(|v| v.loaders.iter().any(|l| l == "fabric")).expect("versión fabric");

    let root = temp_root("pack");
    let paths = ObsidiaPaths::new(root.join("config"), root.join("data"));
    paths.ensure_layout().unwrap();
    let preview = modrinth::preview_version(&client, &paths, &v.id, hit.icon_url.clone()).await.unwrap();
    println!("{preview:#?}");
    assert_eq!(preview.loader, Loader::Fabric);
    assert!(preview.mod_count > 5);

    let store = InstanceStore::new(paths.clone());
    let id = modrinth::install_modpack(&paths, &store, &preview.token, &Reporter::silent())
        .await
        .unwrap();
    let dir = paths.instance_dir(&id);
    let jars = fs::read_dir(dir.join("mods")).unwrap().count();
    println!("instancia {id}: {jars} archivos en mods/");
    assert!(jars >= preview.mod_count);
    assert_eq!(store.get(&id).unwrap().instance.minecraft_version, preview.minecraft_version);
    let _ = fs::remove_dir_all(&root);
}

/// La misma búsqueda que hace el explorador de mods al abrirse.
#[tokio::test]
#[ignore = "requiere red; ejecutar a mano"]
async fn first_page_like_the_mod_browser() {
    let client = modrinth::http_client().unwrap();
    for v in ["1.20.1", "1.21.1"] {
        let q: SearchQuery = serde_json::from_value(serde_json::json!({
            "query": "", "project_type": "mod", "game_version": v,
            "loader": "fabric", "offset": 0, "limit": 20, "sort": "relevance"
        }))
        .unwrap();
        let r = modrinth::search(&client, &q).await;
        println!("{v}: {:?}", r.as_ref().map(|r| r.hits.len()));
        r.unwrap();
    }
}
