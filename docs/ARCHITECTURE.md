# Architecture

## Dépendances

`kortex-domain` contient les modèles sérialisables et les règles pures : cardinalités, clés d’identité, filtres paramétrés, pagination et dispositions. Il n’importe aucun pilote ni bibliothèque graphique.

`kortex-application` définit les ports `DatabaseBackend`, `CancelQuery`, `SecretStore` et l’état du brouillon de tableur. `GridState::receive` protège les modifications en attente. Les identifiants de requêtes empêchent une réponse périmée de remplacer une vue actuelle.

`kortex-infrastructure` implémente PostgreSQL (`tokio-postgres` + Rustls), SQLite (`rusqlite` embarqué), les workers et la persistance. Les appels bloquants de SQLite restent dans un worker dédié. Les connexions PostgreSQL sont pilotées par un runtime Tokio partagé. Les sessions SQL sont séparées de la connexion d’exploration.

`kortex-supervisor` assure la présentation egui, les panneaux `egui_tiles` et les fenêtres natives. `mcd::show` reçoit seulement un catalogue et une disposition : aucune interface de mutation SQL ne lui est fournie. Il retourne des intentions de navigation, d’inspection et d’ouverture de fenêtre.

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
