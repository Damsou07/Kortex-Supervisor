# Validation et limites

## Procédure reproductible

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked -j 1` : règles pures, états applicatifs, transactions/conflits SQLite, sauvegarde atomique et captures UI. La compilation séquentielle évite les pics de mémoire du linker Windows.
4. `scripts/Test-Postgres.ps1` : création puis arrêt/nettoyage d’un cluster jetable. La variable `KORTEX_TEST_PG_PORT` est exigée par le test ignoré de la suite rapide.
5. `scripts/Test-PgMatrix.ps1` : même test sur les binaires officiels PostgreSQL 16 et 17, conservés uniquement dans `.test-data/tools`.
   `scripts/Test-Postgres.ps1 -Tls -Port 55438` vérifie TLS avec une CA générée pour ce test uniquement, sans installation dans le magasin Windows (PowerShell 7.2+).
6. Tests de performance en release : un million de lignes SQLite et 200 tables/500 relations MCD. Le temps MCD mesuré correspond au traitement CPU d’une frame egui, sans présenter cette mesure comme un temps GPU de bout en bout.
7. `cargo audit` puis `scripts/Build-Release.ps1`.

Les tests couvrent la conservation des brouillons pendant un refresh, les résultats périmés, les filtres malveillants, les clés composites/nullable, les scripts à plusieurs instructions, les conflits externes, l’annulation PostgreSQL et la préservation du catalogue par le MCD. Les captures UI doivent être relues avant de mettre à jour les références.

## État initial du 29 octobre 2026, avant la refonte UX

- Rust 1.99.1, Windows 11 x64 (10.0.26200), Build Tools Visual Studio stables 18.11.2, MSVC 14.51.36231.
- `cargo fmt` et Clippy avec `-D warnings` : réussis.
- Suite rapide : **26 tests réussis**, dont les captures UI de l’espace de travail et du MCD, relues visuellement. Quatre tests nécessitant une invocation dédiée sont ignorés dans cette suite.
- Intégration réelle sur des clusters jetables **PostgreSQL 16.15, 17.11 et 18.0** : réussie sur les trois versions. Inclut clés composites, MATCH FULL, JSONB/UUID, rollback atomique, conflit `xmin`, annulation, droits insuffisants et transaction explicite conservée entre deux exécutions SQL.
- SQLite embarqué : clés binaires et sans affinité, pagination, scripts multiples, annulation, transactions persistantes, conflits sous collation NOCASE et clé nullable `INTEGER PRIMARY KEY DESC` vérifiés.
- Mesures release locales : **4,99 ms au p95** sur 120 frames CPU du MCD avec 200 tables/500 relations, canvas 1400×900 ; **0,33 ms** pour une page SQLite de 200 lignes après la clé 900000 dans une table d’un million de lignes. Le processus conserve cette page, sans charger toute la table. Ces mesures ne constituent ni un benchmark GPU de bout en bout, ni une mesure de latence réseau PostgreSQL. La mémoire globale du processus n’a pas fait l’objet d’un profilage exhaustif.
- `cargo audit` 0.22.2 : **aucune vulnérabilité connue** dans le verrouillage à cette date. Trois avis de maintenance concernent `zbus_names`, `zvariant` et `zvariant_derive`, dépendances transitives pour Linux absentes de la cible Windows (`cargo tree --target x86_64-pc-windows-msvc -i zvariant`). Ils restent à suivre avant un portage Linux.
- Le paquet portable a été lancé avec une base de démonstration et un dossier de préférences isolés. La validation manuelle complète des fenêtres MCD détachées, des changements de moniteur et des facteurs DPI reste à réaliser ; les captures automatisées ne la remplacent pas. Le contrôle automatisé de la fenêtre Windows a rencontré l’erreur `foreground window did not report a process id` et n’a pas permis d’achever ces interactions.
- TLS réellement vérifié sur PostgreSQL 18 avec une PKI éphémère : connexion réussie avec CA personnalisée et nom `localhost`, refus du nom `127.0.0.1` absent du certificat, refus du certificat sans sa CA. Les tests rapides contrôlent aussi le refus du texte clair distant et d’un fichier CA vide. Aucun certificat de test n’a été installé dans Windows.

La CI est fournie mais n’a pas été exécutée sur GitHub pendant cette livraison.

## Validation de la refonte UX du 29 octobre 2026

- Suite rapide : **46 tests réussis**, quatre tests dédiés ignorés. La suite couvre le placement déterministe, l’absence de chevauchements, les cycles et autoréférences, les relations parallèles, les ports distincts, les groupes partageant des tables, les ponts aux intersections et la conservation des retouches extérieures à une réorganisation partielle.
- Tests d’interaction : rectangle traversant une relation, sélection additive avec Maj, création/annulation de groupe, texte présent aux zooms 12/25/27/100/250 %, loupe avec type complet, respect du focus des champs, tri protégé pendant un brouillon, raccourcis de navigation/fermeture et confirmation de perte de brouillon.
- Le cache rejette une réponse de réorganisation devenue périmée après un déplacement. Une entrée inchangée conserve sa géométrie. La sauvegarde atomique et la lecture du format existant restent couvertes.
- Six captures de référence relues : espace de travail, MCD, groupes, tableur, raccourcis et onglets avec titre long et indicateur de brouillon distinct.
- Performances finales en release : **20,96 ms au p95** sur 120 frames CPU du MCD avec 200 tables/500 relations (budget : 33 ms), géométrie calculée avant la mesure et réutilisée pendant le panoramique. Page SQLite de 200 lignes après la clé 900000 sur un million de lignes : **0,30 ms**. Ces mesures n’incluent pas la présentation GPU ni une latence réseau PostgreSQL.
- PostgreSQL **18**, cluster jetable local : test de contrat réussi, serveur arrêté et dossier supprimé. Le premier essai dans le sandbox échouait dès la connexion ; la relance hors du sandbox réussit sans changement du code des adaptateurs SQL.
- Paquet portable contrôlé nativement avec la base de démonstration et des préférences isolées : affichage du MCD, sélection Ctrl+A, ouverture/fermeture d’une fenêtre détachée, zoom indépendant à 53 %, loupe lisible et sélection rectangulaire d’une table dans le brouillon de groupe. Le lancement dans le sandbox produisait `foreground window did not report a process id` ; le contrôle a pu être effectué hors du sandbox. Le changement de moniteur et les autres facteurs DPI restent à vérifier ; les tests egui ne remplacent pas ces contrôles.
- Un premier lancement de la suite parallèle a dépassé la mémoire du linker (`LNK1102`) ; la suite et la CI Windows utilisent désormais `-j 1`. Les tests de stockage utilisent ici `TEMP`/`TMP` dirigés vers `target` pour éviter un refus d’accès au dossier temporaire du sandbox. Les avertissements de copie du cache incrémental ne sont pas des avertissements Clippy du code.

## Validation des améliorations MCD du 29 octobre 2026

- Suite rapide : 59 tests réussis et quatre tests spécialisés ignorés ; un test supplémentaire de contour a ensuite été exécuté avec succès. Les nouveaux tests couvrent les profils de disposition, les groupes partagés et leur historique, la sélection par le corps, la priorité de la barre d’outils, les relations entrantes et sortantes, la saturation des cardinalités et la compatibilité des préférences.
- `cargo fmt --all -- --check` et Clippy avec `-D warnings` réussis. Les captures des deux thèmes MCD, de la légende détaillée et des autres vues modifiées ont été examinées.
- Les fenêtres natives, le plein écran et les changements de moniteur ou de DPI restent à vérifier manuellement sur la nouvelle version ; les tests egui couvrent le périmètre partagé des fenêtres mais ne simulent pas le gestionnaire de fenêtres Windows.
- La mesure de performance release a été interrompue pendant la compilation optimisée et n’a pas produit de résultat pour cette version.

## Validation des thèmes supplémentaires et des fenêtres Windows

- Suite complète : **68 tests réussis**, quatre tests spécialisés ignorés par défaut. Les nouveaux scénarios couvrent les six palettes, les trois présentations de légende, les trois palettes de légende, les anciennes préférences, le focus texte en plein écran et la priorité de la molette sur les popups. Les contrôles de formatage et Clippy avec `-D warnings` réussissent.
- Captures des six thèmes MCD et des cartes dans les trois palettes relues, y compris après défilement vers les explications des clés. La capture compacte à 500 px vérifie aussi que toutes les pastilles et l’explication UQ restent intégralement visibles. Les références actualisées ont été contrôlées visuellement.
- Contrôle Windows natif avec `.test-data/demo.db` et préférences isolées dans `target/native-theme-validation` puis `target/native-theme-complete`. Fenêtres complète, de sélection et de groupe ouvertes et fermées indépendamment ; zooms indépendants ; placement partagé vérifié par disposition espacée dans une fenêtre et annulation dans la principale. Maximisation, restauration, redimensionnement, plein écran, sortie par bouton et Échap vérifiés. Les choix Émeraude et Cartes sont restaurés au redémarrage.
- Trois défauts révélés en natif ont été corrigés avec régressions automatisées : Échap quittant le plein écran depuis un champ texte, défilement d’une légende zoomant le canvas derrière elle et pastilles compactes comprimées. Un contrôle supplémentaire garantit la hauteur naturelle des légendes compacte/détaillée et la visibilité complète de UQ. Le premier Échap libère désormais le champ ; le second quitte le plein écran.
- Binaire natif final : `target/debug/kortex-supervisor.exe`, SHA-256 `3abd16e1def36acb00084add0018a91d0eda8705f08272cf5b9020fc017fccbb`. Le correctif de focus a aussi été vérifié sur la construction `d3ae69c426af7fb0c594b51574d04bbd13a309994cfd9e5cd9c8aa0404721d82`, avant la dernière correction de hauteur de légende. Les empreintes et journaux sont conservés dans `target/native-theme-*-binary.json` et les journaux de test correspondants.
- Mesure avec le profil release normal (`lto = "thin"`), 200 tables/500 relations, 120 frames CPU, canvas 1400×900, menus fermés : **20,77 ms au p95**, sous le budget de **33 ms**. Une première exécution avait mesuré 17,50 ms. La compilation optimisée a été menée à son terme ; aucune modification du seuil ou du scénario. Le dernier ajustement de hauteur ne concerne que les menus ouverts. Journaux : `target/themes-performance-release.log` et `target/themes-final-performance-release.log`. Ces mesures excluent la présentation GPU.
- Un seul écran est exposé à cette session Windows (zone de 1536×864). L’affichage à son échelle actuelle a été contrôlé ; les déplacements entre moniteurs et les autres facteurs DPI restent non vérifiés faute de configuration disponible. Les tests PostgreSQL/TLS spécialisés n’ont pas été relancés pour ces changements de présentation.

## Corrections d’affichage du 30 octobre 2026

- Suite finale : **91 tests réussis**, quatre tests spécialisés ignorés par défaut. Commande : `cargo test --workspace --locked -j 1`, avec `TEMP`/`TMP` dirigés vers `target` dans ce sandbox. Journal : `target/ui-fixes-final-tests.log`. `cargo fmt --all -- --check` et Clippy sur tous les targets avec `-D warnings` réussissent (`target/ui-fixes-clippy.log`). Les avertissements de copie du cache incrémental proviennent du système de fichiers, pas du code Clippy.
- TDD : les tests ont d’abord reproduit les badges déplacés et la police dépendante du zoom, le menu fermé après changement de style, le logo absent du parcours de test, l’inspecteur sans relations et le glissement bloqué après un passage par le haut. Les régressions finales vérifient les deux extrémités sur les quatre côtés, pendant le geste et après undo/redo, avec et sans points manuels. Les tests d’interaction attendent jusqu’à 120 frames pour laisser le worker asynchrone se stabiliser sous charge ; leurs assertions restent inchangées.
- Géométrie et sauvegarde : projection aux coins, offsets locaux, 16 combinaisons de côtés, obstacles proches, corridor étroit, autoréférences aux ancrages confondus, conservation des points manuels et lecture des anciennes routes vérifiées. Les tests de badges couvrent taille fixe, collisions avec priorité sélection/survol, masquage/réapparition, noms externes séparés et insertion des points malgré les coudes générés.
- Interface : changements successifs de styles et palettes par clics, maintien et fermeture de la légende, suivi du thème et protection du zoom derrière le menu vérifiés. L’inspecteur couvre relations entrantes/sortantes, composites, autoréférences, cible absente et tables homonymes. Les références des six thèmes MCD, légendes, groupes, bandeau et inspecteur ont été relues ; le bandeau est testé à 1000 × 650 avec le vrai logo de 64 points.
- Performance release : **13,89 ms au p95**, 120 frames CPU, 200 tables/500 relations, canvas 1400 × 900, profil normal avec LTO thin. Seuil conservé : 33 ms. Journal : `target/ui-fixes-performance.log`. Cette mesure exclut la présentation GPU et ne constitue pas une mesure de latence réseau.
- Une exécution intermédiaire du processus de captures a produit `STATUS_ACCESS_VIOLATION` pendant le lancement simultané de l’application native ; la suite complète relancée seule réussit. Les contrôles GPU et les lancements natifs sont désormais effectués séparément pendant cette validation.
- Contrôle Windows partiel : les binaires debug et release ont été lancés avec la base de démonstration et des préférences isolées sous `target/native-ui-fixes*`. Le démarrage, le logo agrandi et le chargement du catalogue de démonstration ont été observés. L’automatisation a ensuite été arrêtée par l’utilisateur avec la touche physique Échap. Les glissements natifs complets, le redimensionnement, les fenêtres détachées et la restauration au redémarrage ne sont donc pas revendiqués comme validés manuellement pour cette livraison ; leurs scénarios automatisés restent couverts. Les autres écrans et facteurs DPI n’ont pas été contrôlés.
- Livraison portable : reconstruite avec `scripts/Build-Release.ps1` ; exécutable et DLL dans `dist/Kortex-Supervisor/`, archive et empreinte dans `dist/Kortex-Supervisor-windows-x64.zip` et son fichier `.sha256`. Les empreintes locales des binaires sont consignées dans `target/ui-fixes-delivery.json`.

## Pagination, chemins locaux et installateur du 30 octobre 2026

- TDD : les nouveaux tests ont reproduit la pagination hors panneau à 1000 × 650 (bouton situé entre y=649 et y=674), le clic Suivant inaccessible et l’absence de setup. Après correction, les contrôles restent visibles avec 200 lignes, 12 colonnes, filtres ouverts et éditeur actif dans un panneau de 340 × 260. Les changements de page et les blocages pendant chargement/brouillon sont vérifiés.
- `cargo fmt --all -- --check`, Clippy `--workspace --all-targets --locked -- -D warnings` et `cargo test --workspace --locked -j 1` réussissent : **98 tests réussis, quatre tests spécialisés ignorés**. Les fichiers temporaires sont isolés sous `target/installer-test-temp` pour permettre les remplacements atomiques dans ce sandbox. Journaux : `target/installer-clippy.log` et `target/installer-workspace-tests.log`.
- Chemins : fichiers absents, chemins longs, SQLite et WAL, CA PostgreSQL, lecture seule, navigation F1 et conservation des profils sont couverts. Une configuration JSON invalide ou de version inconnue reste consultable par son chemin et n’est pas écrasée. Les captures du tableau, du panneau réduit, des chemins et des raccourcis ont été relues.
- Contrôle natif avec une base SQLite jetable de 450 lignes et `KORTEX_CONFIG_DIR` isolé sous `target/installer-native` : affichage des pages 1–200, 201–400 et 401–450, bouton Suivant désactivé à la fin, retour à la page précédente, chemins effectifs et ouverture des raccourcis par F1 vérifiés. La fermeture native réussit. Le coffre Windows refuse l’accès depuis le sandbox (erreur 5) ; aucun test de stockage de secret n’est revendiqué pour cette livraison.
- Build release et compilation Inno Setup 7.1.1 réussis. Le compilateur officiel a été préparé en mode portable après vérification SHA-256 et Authenticode. `Test-Package.ps1` vérifie la version **0.1.0**, les **20 fichiers** du manifeste, les DLL, le contenu du ZIP et les empreintes du ZIP et du setup. Les espaces de remplissage des ressources de version Windows sont retirés avant comparaison. Le binaire release livré a aussi été lancé avec ses DLL dans un dossier isolé, après comparaison de son empreinte, et sa fenêtre s’affiche. Les métadonnées de livraison sont conservées dans `target/installer-delivery.json`.
- L’aperçu du setup a été lancé sans installer le produit. L’outil de contrôle Windows a expiré lors de l’autorisation d’accès à sa fenêtre : la revue visuelle complète de l’assistant reste non vérifiée. Aucun environnement Windows jetable n’est disponible localement pour installer, réinstaller ou désinstaller réellement le produit dans les deux modes. Le script `scripts/Test-Installer.ps1 -DisposableEnvironment` et son étape CI préparent ces vérifications, y compris les raccourcis, la reprise du dossier précédent et la conservation des fichiers utilisateur. Cette CI n’a pas été exécutée à distance.

Pour tester le cycle d’installation, utiliser **uniquement une VM Windows jetable**, avec les droits nécessaires au mode commun :

```powershell
./scripts/Build-Release.ps1 -Installer
./scripts/Test-Installer.ps1 -DisposableEnvironment
```

Le script refuse de démarrer sans cette option et refuse d’écraser une installation ou des raccourcis préexistants. Les tests PostgreSQL/TLS et les benchmarks spécialisés n’ont pas été relancés pour ces modifications de présentation et de packaging.

## Limites explicites du MVP

- Les fichiers `.db` doivent être des bases SQLite non chiffrées. SQLCipher et les autres moteurs utilisant cette extension ne sont pas pris en charge.
- Les tables sans identité unique non nullable et les vues sont modifiées via SQL. Les colonnes générées restent non éditables dans le tableur.
- Les contraintes CHECK SQLite sont consultables dans la définition SQL complète ; leurs expressions ne sont pas réécrites ni interprétées par un parseur propriétaire.
- Les fenêtres MCD partagent leur disposition. Les groupes ne possèdent pas de géométrie indépendante par fenêtre.
- La complétion SQL se présente comme un menu d’insertion des tables et colonnes ; il n’y a pas de serveur de langage SQL ni de validation syntaxique exhaustive côté client.
- `COPY ... FROM STDIN` interactif, outils de sauvegarde/restauration, extensions SQLite chargées dynamiquement et éditeur binaire spécialisé ne font pas partie du MVP.
- Les données changées hors de la page visible ne sont pas mises en cache. La fraîcheur dépend de la cadence réglée, du serveur et du réseau.
- Le plafond SQL borne le nombre de lignes conservées, pas la taille d’une cellule individuelle ; les très gros BLOB/textes peuvent nécessiter de la mémoire.
- Les préférences et dispositions sont persistées ; les onglets SQL, résultats, historique SQL et brouillons ne sont pas restaurés après fermeture.
- Les écritures dont l’issue est inconnue après une coupure ne sont jamais répétées automatiquement. Vérifier l’état de la base avant une nouvelle tentative.

## Publication

La CI et la fabrication de l’archive sont préparées ; leur présence ne signifie pas qu’une exécution distante ou une publication a eu lieu. La livraison reste locale. Le choix de licence du projet et la signature de l’exécutable restent à décider avant diffusion publique.
