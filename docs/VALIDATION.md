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
