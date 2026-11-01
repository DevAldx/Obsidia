use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::{error::CoreError, filesystem, instances::InstanceId, Result};

/// Ubicaciones de Obsidia siguiendo XDG en Linux.
///
/// * `config_dir` (`~/.config/obsidia`): solo `settings.json`. Vive aparte para que
///   el directorio de datos pueda cambiarse sin problema de huevo y gallina.
/// * `data_dir` (`~/.local/share/obsidia`): instancias y almacenamiento compartido.
///
/// Las carpetas de Nebula Launcher (`.../nebula`) se migran solas al arrancar;
/// ver [`migrate_legacy_dirs`].
#[derive(Debug, Clone)]
pub struct ObsidiaPaths {
    config_dir: PathBuf,
    data_dir: PathBuf,
}

impl ObsidiaPaths {
    pub fn from_system() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "obsidia", "obsidia").ok_or(CoreError::NoHomeDir)?;
        let (config_dir, data_dir) = match ProjectDirs::from("dev", "nebula", "nebula") {
            Some(legacy) => migrate_legacy_dirs(
                legacy.config_dir(),
                legacy.data_dir(),
                dirs.config_dir(),
                dirs.data_dir(),
            ),
            None => (dirs.config_dir().to_path_buf(), dirs.data_dir().to_path_buf()),
        };
        Ok(Self { config_dir, data_dir })
    }

    pub fn new(config_dir: impl Into<PathBuf>, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
            data_dir: data_dir.into(),
        }
    }

    /// Devuelve una copia con otro directorio de datos (debe ser absoluto).
    pub fn with_data_dir(&self, data_dir: &Path) -> Result<Self> {
        if !data_dir.is_absolute() {
            return Err(CoreError::invalid(
                "el directorio de datos debe ser una ruta absoluta",
            ));
        }
        Ok(Self {
            config_dir: self.config_dir.clone(),
            data_dir: data_dir.to_path_buf(),
        })
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    pub fn settings_file(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }
    pub fn instances_dir(&self) -> PathBuf {
        self.data_dir.join("instances")
    }
    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir.join("assets")
    }
    pub fn libraries_dir(&self) -> PathBuf {
        self.data_dir.join("libraries")
    }
    pub fn versions_dir(&self) -> PathBuf {
        self.data_dir.join("versions")
    }
    pub fn java_dir(&self) -> PathBuf {
        self.data_dir.join("java")
    }
    pub fn accounts_dir(&self) -> PathBuf {
        self.data_dir.join("accounts")
    }

    /// Ruta de una instancia. Es segura por construcción: `InstanceId` solo admite
    /// `OBS-[A-Z0-9]{6}` (o el antiguo `NBL-`), así que no puede escapar de `instances/`.
    pub fn instance_dir(&self, id: &InstanceId) -> PathBuf {
        self.instances_dir().join(id.as_str())
    }

    /// Crea el árbol base de directorios compartidos.
    pub fn ensure_layout(&self) -> Result<()> {
        for dir in [
            &self.config_dir,
            &self.data_dir,
            &self.instances_dir(),
            &self.assets_dir(),
            &self.libraries_dir(),
            &self.versions_dir(),
            &self.java_dir(),
            &self.accounts_dir(),
        ] {
            filesystem::create_dir_all(dir)?;
        }
        Ok(())
    }
}

// ───────────────────────────── Migración desde Nebula ─────────────────────────────

/// Mueve las carpetas de Nebula Launcher a las de Obsidia la primera vez que
/// arranca Obsidia y devuelve `(config_dir, data_dir)` a usar.
///
/// Es conservadora: solo actúa si la carpeta nueva no existe y la antigua sí;
/// usa `rename` (atómico, sin copiar ni borrar archivos) y, si falla (por
/// ejemplo, otra unidad), sigue usando la carpeta antigua para no perder nada.
pub fn migrate_legacy_dirs(
    old_config: &Path,
    old_data: &Path,
    new_config: &Path,
    new_data: &Path,
) -> (PathBuf, PathBuf) {
    let data = move_dir(old_data, new_data);
    let config = move_dir(old_config, new_config);
    if data == new_data && old_data != new_data {
        rewrite_paths(old_data, new_data, &config);
    }
    for old in [old_data, old_config] {
        remove_empty_legacy_parents(old);
    }
    (config, data)
}

fn move_dir(old: &Path, new: &Path) -> PathBuf {
    if new.exists() || !old.is_dir() {
        return new.to_path_buf();
    }
    if let Some(parent) = new.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("no se pudo preparar {}: {e}; se sigue usando {}", parent.display(), old.display());
            return old.to_path_buf();
        }
    }
    match std::fs::rename(old, new) {
        Ok(()) => {
            eprintln!("migrado {} → {}", old.display(), new.display());
            new.to_path_buf()
        }
        Err(e) => {
            eprintln!("no se pudo mover {}: {e}; se sigue usando esa carpeta", old.display());
            old.to_path_buf()
        }
    }
}

/// En Windows/macOS quedan carpetas `nebula` vacías por encima: se quitan
/// solo si están vacías y se llaman `nebula`.
fn remove_empty_legacy_parents(old: &Path) {
    let mut dir = old.parent();
    while let Some(d) = dir {
        if d.file_name().and_then(|n| n.to_str()) != Some("nebula") || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

fn replace_prefix(value: &str, old: &str, new: &str) -> Option<String> {
    value.strip_prefix(old).map(|rest| format!("{new}{rest}"))
}

fn rewrite_json(path: &Path, edit: impl FnOnce(&mut serde_json::Value) -> bool) {
    let Ok(bytes) = std::fs::read(path) else { return };
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return };
    if edit(&mut value) {
        if let Ok(json) = serde_json::to_vec_pretty(&value) {
            if let Err(e) = filesystem::write_atomic(path, &json) {
                eprintln!("no se pudo actualizar {}: {e}", path.display());
            }
        }
    }
}

/// Corrige rutas absolutas que apuntaban dentro de la carpeta antigua: el Java
/// manual de cada instancia (p. ej. un Temurin descargado) y `data_dir`.
fn rewrite_paths(old_data: &Path, new_data: &Path, config: &Path) {
    let (Some(old), Some(new)) = (old_data.to_str(), new_data.to_str()) else { return };
    if let Ok(entries) = std::fs::read_dir(new_data.join("instances")) {
        for entry in entries.flatten() {
            rewrite_json(&entry.path().join("instance.json"), |v| {
                let Some(path) = v.pointer_mut("/java/path") else { return false };
                match path.as_str().and_then(|p| replace_prefix(p, old, new)) {
                    Some(fixed) => {
                        *path = fixed.into();
                        true
                    }
                    None => false,
                }
            });
        }
    }
    rewrite_json(&config.join("settings.json"), |v| {
        let Some(dir) = v.get_mut("data_dir") else { return false };
        match dir.as_str().and_then(|p| replace_prefix(p, old, new)) {
            Some(fixed) => {
                *dir = fixed.into();
                true
            }
            None => false,
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("obsidia-migrate-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn migrates_nebula_dirs_and_rewrites_paths() {
        let root = tmp();
        let (old_c, old_d) = (root.join("config/nebula"), root.join("share/nebula"));
        let (new_c, new_d) = (root.join("config/obsidia"), root.join("share/obsidia"));
        let inst = old_d.join("instances/NBL-AAAAAA");
        std::fs::create_dir_all(&inst).ok();
        std::fs::create_dir_all(&old_c).ok();
        let java = format!("{}/java/temurin-21/bin/java", old_d.display());
        std::fs::write(inst.join("instance.json"), format!(r#"{{"java":{{"kind":"path","path":"{java}"}}}}"#)).ok();
        std::fs::write(old_c.join("settings.json"), format!(r#"{{"data_dir":"{}"}}"#, old_d.display())).ok();

        let (c, d) = migrate_legacy_dirs(&old_c, &old_d, &new_c, &new_d);
        assert_eq!((c.as_path(), d.as_path()), (new_c.as_path(), new_d.as_path()));
        assert!(!old_d.exists() && !old_c.exists());
        let inst_json = std::fs::read_to_string(new_d.join("instances/NBL-AAAAAA/instance.json")).unwrap_or_default();
        assert!(inst_json.contains(&format!("{}/java/temurin-21", new_d.display())), "{inst_json}");
        let settings = std::fs::read_to_string(new_c.join("settings.json")).unwrap_or_default();
        assert!(settings.contains(&new_d.display().to_string()), "{settings}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn never_overwrites_existing_obsidia_dirs() {
        let root = tmp();
        let (old_c, old_d) = (root.join("c_old"), root.join("d_old"));
        let (new_c, new_d) = (root.join("c_new"), root.join("d_new"));
        for d in [&old_c, &old_d, &new_c, &new_d] {
            std::fs::create_dir_all(d).ok();
        }
        std::fs::write(old_d.join("marca"), b"vieja").ok();
        let (c, d) = migrate_legacy_dirs(&old_c, &old_d, &new_c, &new_d);
        assert_eq!((c, d), (new_c, new_d));
        assert!(old_d.join("marca").exists(), "la carpeta antigua no se toca");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn fresh_install_uses_new_dirs() {
        let root = tmp();
        let (c, d) = migrate_legacy_dirs(&root.join("a"), &root.join("b"), &root.join("c"), &root.join("d"));
        assert_eq!((c, d), (root.join("c"), root.join("d")));
        let _ = std::fs::remove_dir_all(&root);
    }
}
