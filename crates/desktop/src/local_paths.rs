use kortex_domain::{Engine, Profile};
use std::path::{Path, PathBuf};

pub(crate) struct PathEntry {
    label: String,
    path: Option<PathBuf>,
    exists: bool,
}

fn entry(label: impl Into<String>, path: Option<PathBuf>) -> PathEntry {
    let path = path.map(|p| std::path::absolute(&p).unwrap_or(p));
    let exists = path.as_ref().is_some_and(|p| p.exists());
    PathEntry {
        label: label.into(),
        path,
        exists,
    }
}

// Collect once when entering the tab, rather than probing files on every frame.
pub(crate) fn collect(config: Option<&Path>, profiles: &[Profile]) -> Vec<PathEntry> {
    let exe = std::env::current_exe().ok();
    let mut entries = vec![
        entry("Exécutable", exe.clone()),
        entry(
            "Dossier d’installation",
            exe.and_then(|p| p.parent().map(Path::to_path_buf)),
        ),
        entry(
            "Dossier de configuration",
            config.and_then(Path::parent).map(Path::to_path_buf),
        ),
        entry("Fichier de configuration", config.map(Path::to_path_buf)),
    ];
    for profile in profiles {
        match &profile.engine {
            Engine::Sqlite { path } => {
                entries.push(entry(
                    format!("SQLite · {}", profile.name),
                    Some(path.into()),
                ));
                for suffix in ["-wal", "-shm"] {
                    let sidecar = PathBuf::from(format!("{path}{suffix}"));
                    if sidecar.exists() {
                        entries.push(entry(
                            format!("SQLite {suffix} · {}", profile.name),
                            Some(sidecar),
                        ));
                    }
                }
            }
            Engine::Postgres { ca_file, .. } if !ca_file.is_empty() => {
                entries.push(entry(
                    format!("Certificat CA · {}", profile.name),
                    Some(ca_file.into()),
                ));
            }
            _ => {}
        }
    }
    entries
}

pub(crate) fn show(ui: &mut egui::Ui, entries: &[PathEntry]) {
    egui::ScrollArea::vertical().id_salt("local_paths").show(ui, |ui| {
        ui.label("Emplacements utilisés par Kortex Supervisor — lecture seule.");
        for item in entries {
            ui.add_space(8.0);
            ui.strong(&item.label);
            if let Some(path) = &item.path {
                ui.add(egui::Label::new(path.to_string_lossy()).wrap().selectable(true));
                if !item.exists {
                    ui.weak("Absent actuellement — emplacement conservé.");
                }
            } else {
                ui.weak("Emplacement indisponible.");
            }
        }
        ui.separator();
        ui.label("Mots de passe : Gestionnaire d’informations d’identification Windows (service Kortex Supervisor), sans fichier géré par l’application.");
        ui.label("Résultats SQL, brouillons et activité : uniquement en mémoire. Aucun dossier persistant de cache ou de journaux.");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    #[test]
    fn paths_include_missing_files_sidecars_and_ca_without_changing_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("données SQLite.db");
        let wal = PathBuf::from(format!("{}-wal", db.display()));
        std::fs::write(&wal, b"fixture").unwrap();
        let profiles = vec![
            Profile {
                id: "sqlite".into(),
                name: "Locale".into(),
                engine: Engine::Sqlite {
                    path: db.to_string_lossy().into(),
                },
                remember_password: false,
            },
            Profile {
                id: "pg".into(),
                name: "Distante".into(),
                engine: Engine::Postgres {
                    host: "localhost".into(),
                    port: 5432,
                    database: "test".into(),
                    user: "test".into(),
                    tls: true,
                    ca_file: dir.path().join("ca.pem").to_string_lossy().into(),
                },
                remember_password: false,
            },
        ];
        let original = profiles.clone();
        let config = dir
            .path()
            .join("configuration personnalisée")
            .join("workspace.json");
        let entries = collect(Some(&config), &profiles);
        assert_eq!(entries[3].path.as_deref(), Some(config.as_path()));
        assert!(!entries[3].exists);
        assert!(
            entries
                .iter()
                .any(|e| e.path.as_deref() == Some(wal.as_path()) && e.exists)
        );
        assert!(
            entries
                .iter()
                .any(|e| e.label == "Certificat CA · Distante")
        );
        assert!(!db.exists());
        assert_eq!(profiles, original);
    }

    #[test]
    fn long_paths_are_selectable_labels_and_never_editable_inputs() {
        let path = PathBuf::from(format!(
            "C:/{}workspace.json",
            "chemin très long/".repeat(25)
        ));
        let mut initialized = false;
        let mut h = Harness::builder()
            .with_size([420.0, 500.0])
            .build_ui(move |ui| {
                if !initialized {
                    crate::theme::apply(ui.ctx());
                    initialized = true;
                }
                show(ui, &[entry("Fichier de configuration", Some(path.clone()))]);
            });
        h.run();
        assert!(h.query_by_role(egui::Role::TextInput).is_none());
        let label = h.get_by_label("Fichier de configuration").rect();
        assert!(label.max.x <= 420.0);
        h.snapshot("local_paths");
    }
}
