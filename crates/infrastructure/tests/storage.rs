use kortex_domain::*;
use kortex_infrastructure::storage::{self, Preferences};

#[test]
fn old_preferences_choose_default_appearance() {
    use kortex_infrastructure::storage::{DiagramTheme, LegendPalette, LegendStyle};
    let prefs: Preferences = serde_json::from_str(r#"{"version":1}"#).unwrap();
    assert_eq!(prefs.appearance.diagram_theme, DiagramTheme::Modern);
    assert_eq!(prefs.appearance.legend_style, LegendStyle::Compact);
    assert_eq!(
        prefs.appearance.legend_palette,
        LegendPalette::FollowDiagram
    );
    let previous: storage::Appearance =
        serde_json::from_str(r#"{"diagram_theme":"Midnight","legend_style":"Detailed"}"#).unwrap();
    assert_eq!(previous.diagram_theme, DiagramTheme::Midnight);
    assert_eq!(previous.legend_style, LegendStyle::Detailed);
    assert_eq!(previous.legend_palette, LegendPalette::FollowDiagram);
    let updated = storage::Appearance {
        diagram_theme: DiagramTheme::Midnight,
        legend_style: LegendStyle::Detailed,
        legend_palette: LegendPalette::Neutral,
    };
    let restored: storage::Appearance =
        serde_json::from_str(&serde_json::to_string(&updated).unwrap()).unwrap();
    assert_eq!(restored, updated);
}

#[test]
fn all_appearance_choices_round_trip_without_changing_existing_names() {
    use storage::{Appearance, DiagramTheme, LegendPalette, LegendStyle};
    assert_eq!(
        serde_json::to_string(&DiagramTheme::Modern).unwrap(),
        "\"Modern\""
    );
    assert_eq!(
        serde_json::to_string(&DiagramTheme::Midnight).unwrap(),
        "\"Midnight\""
    );
    for diagram_theme in [
        DiagramTheme::Modern,
        DiagramTheme::Midnight,
        DiagramTheme::Obsidian,
        DiagramTheme::Emerald,
        DiagramTheme::Amethyst,
        DiagramTheme::HighContrast,
    ] {
        for legend_style in [
            LegendStyle::Compact,
            LegendStyle::Detailed,
            LegendStyle::Cards,
        ] {
            for legend_palette in [
                LegendPalette::FollowDiagram,
                LegendPalette::Neutral,
                LegendPalette::HighContrast,
            ] {
                let appearance = Appearance {
                    diagram_theme,
                    legend_style,
                    legend_palette,
                };
                let restored: Appearance =
                    serde_json::from_str(&serde_json::to_string(&appearance).unwrap()).unwrap();
                assert_eq!(restored, appearance);
            }
        }
    }
}

#[test]
fn atomic_replace_round_trip_and_unknown_version_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.json");
    let mut prefs = Preferences::default();
    prefs.profiles.push(Profile {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::Sqlite {
            path: "test.db".into(),
        },
        remember_password: false,
    });
    let mut diagram = Diagram::default();
    diagram
        .positions
        .insert("orders".into(), Point { x: 42.0, y: 64.0 });
    diagram.routes.insert(
        "fk".into(),
        Route {
            points: vec![Point { x: 100.0, y: 30.0 }],
            source_y: Some(80.0),
            target_right: true,
            ..Route::default()
        },
    );
    diagram.groups.push(Group {
        id: 1,
        name: "Commandes".into(),
        members: vec!["orders".into()],
        origin: Point { x: 1.0, y: 2.0 },
        size: Point { x: 600.0, y: 400.0 },
        color: [110, 80, 25, 45],
    });
    prefs.diagrams.insert("test".into(), diagram.clone());
    storage::save(&path, &prefs).unwrap();
    prefs.data_refresh_secs = 2.0;
    storage::save(&path, &prefs).unwrap();
    assert_eq!(storage::load(&path).unwrap().data_refresh_secs, 2.0);
    assert_eq!(storage::load(&path).unwrap().diagrams["test"], diagram);
    prefs.version = 99;
    storage::save(&path, &prefs).unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(storage::load(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
