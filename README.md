# Kortex Supervisor

Application desktop locale en Rust + egui pour explorer PostgreSQL et les fichiers SQLite, consulter et modifier les données, exécuter du SQL et organiser un MCD interactif.

## Démarrer

Windows x64, avec un pilote graphique compatible DirectX 12 ou Vulkan. L’application se lance sans serveur web ni compte Kortex.

Pour utiliser la livraison portable, extraire entièrement `dist/Kortex-Supervisor-windows-x64.zip` puis ouvrir **kortex-supervisor.exe**. Garder les DLL fournies à côté de l’exécutable. Rust et Visual Studio ne sont nécessaires que pour compiler le projet.

Pour lancer depuis les sources :

```powershell
cargo run -p kortex-supervisor
# Ouvrir directement une base SQLite existante :
cargo run -p kortex-supervisor -- 'C:\chemin\base.db'
```

Le paquet portable se construit avec `./scripts/Build-Release.ps1`. L’exécutable et les documents sont déposés dans `dist/Kortex-Supervisor/` et dans une archive ZIP. La compilation utilise Rust 1.99.1, fixé par `rust-toolchain.toml`, et des Build Tools Visual Studio stables avec les composants C++/SDK Windows.

## Fonctions

- Plusieurs profils PostgreSQL et SQLite, onglets explicitement rattachés à leur connexion.
- PostgreSQL distant avec TLS, vérification du certificat/nom d’hôte et CA personnalisée. Le mode sans TLS est réservé à `localhost`, `127.0.0.1` et `::1`.
- Explorateur de schémas, tables et vues ; inspecteur de colonnes, index, contraintes et relations.
- Tableur paginé : tri multiple avec Maj+clic, filtres combinables, ajout et suppression de lignes, édition par double-clic sur une cellule.
- **Appliquer** enregistre le lot dans une transaction ; **Annuler les modifications** abandonne le brouillon. Les conflits externes empêchent l’écrasement silencieux des données.
- Éditeur SQL à sessions indépendantes : scripts, sélection, Ctrl+Entrée, insertion des objets connus, résultats multiples et annulation côté moteur.
- MCD : tables déplaçables, zoom, tracés ajustables, cardinalités, groupes nommés et fenêtres natives partageant la disposition.
- Panneaux déplaçables par leur onglet et redimensionnables. Disposition et groupes enregistrés localement.

**Le MCD ne modifie jamais la base.** La création/modification des tables, colonnes et contraintes passe par l’éditeur SQL.

## Actualisation et sauvegarde

PostgreSQL : données de l’onglet actif contrôlées chaque seconde ; catalogue toutes les cinq secondes. Fréquences réglables, activité réduite en arrière-plan. Un brouillon n’est jamais remplacé par un rafraîchissement. Les scripts SQL ne sont jamais relancés automatiquement.

SQLite : changements externes actualisés manuellement ; les écritures de Kortex déclenchent une mise à jour des vues. Le fichier doit déjà exister ; Kortex ne crée pas silencieusement de fichier vide.

Les préférences sont enregistrées dans le dossier de configuration utilisateur Windows (`%APPDATA%\Kortex\Kortex Supervisor\config\workspace.json`, chemin déterminé par `directories`). `KORTEX_CONFIG_DIR` permet de choisir un dossier isolé ou portable. Les mots de passe mémorisés utilisent le Gestionnaire d’informations d’identification Windows. Les données consultées, résultats SQL et brouillons ne sont pas conservés dans cette configuration.

## Développement et tests

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
./scripts/Test-Postgres.ps1                       # cluster jetable PostgreSQL 18 local
./scripts/Test-PgMatrix.ps1                       # archives officielles 16 et 17, clusters jetables
./scripts/Test-Postgres.ps1 -Tls -Port 55438        # certificats éphémères ; PowerShell 7.2+
cargo test --release -p kortex-supervisor -p kortex-infrastructure --test performance -- --ignored --nocapture
cargo audit
```

Les tests PostgreSQL et de performance sont explicitement ignorés dans la suite rapide ; les commandes dédiées et la CI les exécutent séparément. Aucun test ne se connecte à une base métier. Les captures de référence se trouvent dans `crates/desktop/tests/snapshots`. Leur mise à jour doit être revue visuellement : `$env:UPDATE_SNAPSHOTS='1'; cargo test -p kortex-supervisor --test ui`.

Une base de démonstration peut être créée dans un **nouveau** fichier :

```powershell
cargo run -p kortex-infrastructure --example create_demo -- demo.db
```

## Documentation

- [Guide utilisateur](docs/UTILISATION.md)
- [Architecture et règles de sécurité](docs/ARCHITECTURE.md)
- [Validation et limites connues](docs/VALIDATION.md)

Le projet est préparé pour une publication future. Aucune licence du code du projet n’est choisie implicitement. Les polices Noto embarquées disposent de leurs licences OFL dans `assets/fonts/` ; les dépendances conservent leurs licences respectives.
