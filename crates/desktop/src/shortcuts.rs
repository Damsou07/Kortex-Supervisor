//! One registry for keyboard dispatch and the searchable help catalogue.
use egui::{Context, Key, Modifiers, Ui};
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Help,
    Close,
    Next,
    Previous,
    Execute,
    SelectAll,
    Undo,
    Redo,
    Group,
    Fit,
    Select,
    Pan,
    Cancel,
}
struct Binding {
    command: Command,
    key: Key,
    modifiers: Modifiers,
    scope: &'static str,
    label: &'static str,
    description: &'static str,
}
const BINDINGS: &[Binding] = &[
    Binding {
        command: Command::Help,
        key: Key::F1,
        modifiers: Modifiers::NONE,
        scope: "Général",
        label: "F1",
        description: "Afficher les raccourcis clavier",
    },
    Binding {
        command: Command::Close,
        key: Key::W,
        modifiers: Modifiers::CTRL,
        scope: "Général",
        label: "Ctrl+W",
        description: "Fermer l’onglet actif",
    },
    Binding {
        command: Command::Next,
        key: Key::Tab,
        modifiers: Modifiers::CTRL,
        scope: "Général",
        label: "Ctrl+Tab",
        description: "Onglet suivant",
    },
    Binding {
        command: Command::Previous,
        key: Key::Tab,
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::CTRL
        },
        scope: "Général",
        label: "Ctrl+Maj+Tab",
        description: "Onglet précédent",
    },
    Binding {
        command: Command::Execute,
        key: Key::Enter,
        modifiers: Modifiers::CTRL,
        scope: "SQL",
        label: "Ctrl+Entrée",
        description: "Exécuter la sélection ou le script",
    },
    Binding {
        command: Command::SelectAll,
        key: Key::A,
        modifiers: Modifiers::CTRL,
        scope: "MCD",
        label: "Ctrl+A",
        description: "Sélectionner toutes les tables visibles",
    },
    Binding {
        command: Command::Undo,
        key: Key::Z,
        modifiers: Modifiers::CTRL,
        scope: "MCD",
        label: "Ctrl+Z",
        description: "Annuler le placement, tracé ou groupe",
    },
    Binding {
        command: Command::Redo,
        key: Key::Y,
        modifiers: Modifiers::CTRL,
        scope: "MCD",
        label: "Ctrl+Y",
        description: "Rétablir",
    },
    Binding {
        command: Command::Group,
        key: Key::G,
        modifiers: Modifiers::CTRL,
        scope: "MCD",
        label: "Ctrl+G",
        description: "Créer un groupe de tables",
    },
    Binding {
        command: Command::Fit,
        key: Key::F,
        modifiers: Modifiers::NONE,
        scope: "MCD",
        label: "F",
        description: "Cadrer la sélection ou la vue",
    },
    Binding {
        command: Command::Select,
        key: Key::V,
        modifiers: Modifiers::NONE,
        scope: "MCD",
        label: "V",
        description: "Mode sélection rectangulaire",
    },
    Binding {
        command: Command::Pan,
        key: Key::H,
        modifiers: Modifiers::NONE,
        scope: "MCD",
        label: "H",
        description: "Mode déplacement",
    },
    Binding {
        command: Command::Cancel,
        key: Key::Escape,
        modifiers: Modifiers::NONE,
        scope: "MCD",
        label: "Échap",
        description: "Annuler la sélection ou le brouillon de groupe",
    },
];
pub fn consume(ctx: &Context, command: Command) -> bool {
    let b = BINDINGS
        .iter()
        .find(|b| b.command == command)
        .expect("registered command");
    ctx.input_mut(|i| i.consume_key(b.modifiers, b.key))
}
pub fn catalogue(ui: &mut Ui, search: &mut String) {
    ui.heading("Raccourcis clavier");
    ui.add(
        egui::TextEdit::singleline(search)
            .hint_text("Rechercher une action, une touche…")
            .desired_width(f32::INFINITY),
    );
    let query = search.to_lowercase();
    egui::ScrollArea::vertical()
        .max_height(500.0)
        .show(ui, |ui| {
            for scope in ["Général", "SQL", "MCD", "Tableur", "Champs texte"] {
                let entries: Vec<_> = BINDINGS
                    .iter()
                    .filter(|b| b.scope == scope)
                    .map(|b| (b.label, b.description))
                    .chain(
                        INTERACTIONS
                            .iter()
                            .filter(|(s, _, _)| *s == scope)
                            .map(|(_, k, d)| (*k, *d)),
                    )
                    .filter(|(k, d)| format!("{scope} {k} {d}").to_lowercase().contains(&query))
                    .collect();
                if entries.is_empty() {
                    continue;
                }
                ui.separator();
                ui.strong(scope);
                for (keys, description) in entries {
                    ui.horizontal_wrapped(|ui| {
                        ui.monospace(keys);
                        ui.label(description);
                    });
                }
            }
        });
}
const INTERACTIONS: &[(&str, &str, &str)] = &[
    (
        "Général",
        "Tab / Maj+Tab",
        "Parcourir les contrôles accessibles au clavier",
    ),
    (
        "Champs texte",
        "Début / Fin · Ctrl+Début / Ctrl+Fin",
        "Aller au début ou à la fin de la ligne / du texte",
    ),
    (
        "Champs texte",
        "Ctrl+Retour arrière / Ctrl+Suppr",
        "Effacer le mot précédent / suivant",
    ),
    (
        "MCD",
        "Maj+clic",
        "Ajouter ou retirer une table de la sélection",
    ),
    (
        "MCD",
        "Glisser sur le fond · mode Sélection",
        "Sélectionner les tables touchées par le rectangle ; Maj ajoute",
    ),
    ("MCD", "Molette", "Zoomer autour du pointeur"),
    (
        "MCD",
        "Glisser sur le fond · mode Déplacement / bouton du milieu",
        "Déplacer la vue",
    ),
    (
        "MCD",
        "Glisser un en-tête",
        "Déplacer une table ou un groupe",
    ),
    (
        "MCD",
        "Double-clic table / groupe",
        "Ouvrir les données / une fenêtre détachée",
    ),
    (
        "MCD",
        "Double-clic lien / point de passage",
        "Ajouter / retirer un point de passage",
    ),
    (
        "MCD",
        "Glisser une poignée rouge",
        "Déplacer un ancrage visuel",
    ),
    (
        "Tableur",
        "Clic / Maj+clic sur un en-tête",
        "Trier / ajouter un critère de tri",
    ),
    ("Tableur", "Double-clic cellule", "Éditer une cellule"),
    ("Tableur", "Clic droit cellule", "Copier la valeur"),
    (
        "Champs texte",
        "Ctrl+C / Ctrl+X / Ctrl+V",
        "Copier / couper / coller",
    ),
    (
        "Champs texte",
        "Ctrl+A / Ctrl+Z / Ctrl+Y",
        "Tout sélectionner / annuler / rétablir la saisie",
    ),
    (
        "Champs texte",
        "Ctrl+← / Ctrl+→ · Maj+déplacement",
        "Déplacer par mot / étendre la sélection",
    ),
];
