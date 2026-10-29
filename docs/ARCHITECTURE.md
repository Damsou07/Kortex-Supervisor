# Architecture

## Dépendances

`kortex-domain` contient les modèles sérialisables et les règles pures : cardinalités, clés d’identité, filtres paramétrés, pagination et dispositions. Il n’importe aucun pilote ni bibliothèque graphique.

`kortex-application` définit les ports `DatabaseBackend`, `CancelQuery`, `SecretStore` et l’état du brouillon de tableur. `GridState::receive` protège les modifications en attente. Les identifiants de requêtes empêchent une réponse périmée de remplacer une vue actuelle.

`kortex-infrastructure` implémente PostgreSQL (`tokio-postgres` + Rustls), SQLite (`rusqlite` embarqué), les workers et la persistance. Les appels bloquants de SQLite restent dans un worker dédié. Les connexions PostgreSQL sont pilotées par un runtime Tokio partagé. Les sessions SQL sont séparées de la connexion d’exploration.

`kortex-supervisor` assure la présentation egui, les panneaux `egui_tiles` et les fenêtres natives. `mcd::show` reçoit seulement un catalogue et une disposition : aucune interface de mutation SQL ne lui est fournie. Il retourne des intentions de navigation, d’inspection et d’ouverture de fenêtre.

## Géométrie et présentation du MCD

`domain::geometry` définit `LayoutInput`, `LayoutResult`, les rectangles et chemins en coordonnées monde. Le placement utilise les composantes fortement connexes, des couches ordonnées par barycentre, la compaction et le rangement des composantes indépendantes. Les couches trop longues sont repliées pour éviter des diagrammes excessivement étendus. Les groupes qui partagent des tables forment un bloc de placement commun sans modifier leurs appartenances.

Les tailles des cartes tiennent compte des colonnes et du nombre de ports. Les ports des relations automatiques sont séparés de 16 unités et restent proches des colonnes concernées lorsque l’espace le permet. Le routage orthogonal utilise une grille d’obstacles avec une marge de 12 unités ; son coût pénalise longueur, coudes, segments superposés et croisements. Les intersections restantes produisent des ponts visuels. Cette heuristique ne promet pas un optimum global ni un graphe sans croisements dans tous les cas.

Le port applicatif `DiagramExecutor` échange des requêtes/réponses identifiées par révision et transmet le profil de placement compact ou espacé. `DiagramWorker` fournit un worker CPU et une file bornée, séparés des workers SQL. Le cache de chaque vue ignore les résultats dont l’entrée a changé ; zoom et panoramique ne font pas partie de cette entrée. Le rendu reste disponible pendant les calculs. L’initialisation et l’ajout de tables calculent uniquement les positions ; le routage complet se fait dans le worker.

Les opérations applicatives sur les groupes utilisent un `GroupDraft`. Valider un groupe, supprimer son cadre ou appliquer une réorganisation constitue une seule entrée d’historique. Les cadres affectés sont recalculés avec le déplacement des membres dans la même entrée, y compris pour les groupes qui se chevauchent. Les fenêtres partagent le modèle et l’historique, mais gardent leur propre vue. Les préférences d’apparence sont rétrocompatibles et distinctes de la géométrie. La présentation sépare le panneau des groupes, le cache géométrique, le dessin commun des colonnes et les onglets. Le registre des raccourcis alimente leur exécution et leur catalogue.

Le format persistant `Diagram` reste inchangé : positions, groupes et retouches manuelles de `Route`. Les chemins automatiques et les états transitoires de sélection/brouillon ne sont pas sérialisés. La réconciliation conserve les positions existantes et place les nouvelles tables dans un espace libre. Aucun changement du catalogue ou des données n’est déclenché par ces fonctions.

## Concurrence et volumes

Les files de commandes et de résultats sont bornées. Chaque requête possède un identifiant ; les vues de données n’acceptent que leur réponse attendue. Le rendu n’attend pas les requêtes. Les résultats SQL arrivent par blocs de 200 lignes ; les pages du tableur sont limitées à 200 lignes par défaut.

Le tri est complété par une clé unique stable lorsqu’elle existe. Le curseur lexicographique est utilisé si toutes ses composantes sont non NULL ; les autres cas utilisent le décalage. Le rendu des lignes est virtualisé. Le MCD élimine du dessin les tables hors écran et conserve les coordonnées entre actualisations.

Le polling PostgreSQL ne dépend ni de statistiques de modifications potentiellement retardées, ni de triggers installés sur la base. Il relit la page visible et les métadonnées à cadence réglable. La cadence est ralentie lorsque la fenêtre principale est inactive. Les sessions SQL ne font jamais partie du polling.

## Écritures et sécurité

Le tableur envoie un `ChangeSet` atomique. SQLite acquiert un verrou d’écriture avec `BEGIN IMMEDIATE` et compare les valeurs originales, types compris, en collation binaire. PostgreSQL compare clé et `xmin`. Chaque opération doit affecter exactement une ligne ; sinon tout le lot échoue.

Les filtres et les modifications utilisent des paramètres liés. Les identifiants SQL sont cités séparément. Les conversions PostgreSQL emploient les types obtenus du catalogue. Les scripts saisis par l’utilisateur sont exécutés tels quels avec ses droits ; le logiciel n’est pas une frontière de permissions supplémentaire.

TLS vérifie le certificat et le nom du serveur. Aucun mécanisme de désactivation silencieuse de cette validation n’est prévu. Les secrets ne sont jamais sérialisés dans les profils ; leur stockage optionnel passe par le coffre natif du système.

Les préférences sont écrites dans un fichier temporaire du même dossier, synchronisées puis remplacées atomiquement. Une configuration mal formée ou d’une version inconnue est conservée et sa réécriture automatique est désactivée.

## Travail futur

Les interfaces permettent d’autres moteurs et plateformes. La publication, la licence du projet, la signature Windows, les mises à jour automatiques, le tunnel SSH intégré et la modification graphique du schéma restent des décisions futures explicites.
