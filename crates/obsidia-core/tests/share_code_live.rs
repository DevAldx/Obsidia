//! Prueba manual de red: genera un código `OBS1-` con un mod real (Sodium,
//! descargado de Modrinth) y lo vuelve a resolver. No forma parte de la suite
//! normal; se ejecuta a mano con
//! `cargo test -p obsidia-core --test share_code_live -- --ignored --nocapture`.

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use obsidia_core::{
    instances::{Loader, Memory},
    share_code::{self, ExportInput},
};

const SODIUM_URL: &str =
    "https://cdn.modrinth.com/data/AANobbMI/versions/SMxNOGZ6/sodium-fabric-0.8.13%2Bmc1.21.1.jar";
const SODIUM_SHA1: &str = "003c114c85ca88ef3362e018deb6aca0c682d6a1";

#[tokio::test]
#[ignore = "requiere red; ejecutar a mano"]
async fn exports_and_resolves_a_real_pack() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("obsidia-share-live-{stamp}"));
    let mods = root.join("mods");
    fs::create_dir_all(&mods).unwrap();

    let client = reqwest::Client::new();
    let jar = client.get(SODIUM_URL).send().await.unwrap().bytes().await.unwrap();
    fs::write(mods.join("sodium.jar"), &jar).unwrap();
    fs::write(mods.join("privado.jar"), b"no esta en modrinth").unwrap();

    let exported = share_code::export(
        &client,
        ExportInput {
            name: "Prueba",
            description: Some("Pack de prueba"),
            minecraft_version: "1.21.1",
            loader: Loader::Fabric,
            loader_version: Some("0.16.5"),
            memory: Memory { min_mb: 2048, max_mb: 4096 },
            background_url: Some("https://cdn.modrinth.com/data/AANobbMI/icon.png"),
            mods_dir: &mods,
            resourcepacks_dir: &root.join("resourcepacks"),
            shaderpacks_dir: &root.join("shaderpacks"),
        },
    )
    .await
    .unwrap();
    println!("código ({} caracteres): {}", exported.code.len(), exported.code);
    assert_eq!(exported.included, 1);
    assert_eq!(exported.missing, vec!["privado.jar".to_owned()]);

    let manifest = share_code::resolve(&client, &exported.code).await.unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(manifest.name, "Prueba");
    assert_eq!(manifest.memory.map(|m| m.max_mb), Some(4096));
    assert_eq!(manifest.mods.len(), 1);
    assert_eq!(manifest.mods[0].sha1.as_deref(), Some(SODIUM_SHA1));
    assert_eq!(manifest.mods[0].size, Some(jar.len() as u64));
}
