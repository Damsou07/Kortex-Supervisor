# Utilisation

## Connexions et navigation

Utiliser **Nouvelle connexion** pour PostgreSQL, ou **Ouvrir un fichier SQLite** pour un fichier `.db`, `.sqlite` ou `.sqlite3`. Tester un serveur distant avec TLS ; pour une autorité privée, sélectionner son certificat PEM. Les tunnels SSH sont créés extérieurement, puis utilisés comme une connexion locale.

Les profils sont conservés, mais ne se reconnectent pas automatiquement au démarrage. Un profil peut être modifié lorsqu’il n’est pas ouvert. **Oublier** retire le profil et son secret mémorisé ; fermer d’abord ses onglets. Pour changer de base PostgreSQL, le menu **Bases disponibles** ouvre un profil distinct afin de préserver le contexte des onglets existants.

Cliquer sur une table pour ouvrir ses données. Les onglets permettent de revenir instantanément aux pages déjà chargées. Leur infobulle et le bandeau de contexte indiquent la connexion utilisée. L’inspecteur affiche les détails de la table sélectionnée.

Déplacer les onglets de panneaux EXPLORATEUR, ESPACE DE TRAVAIL, INSPECTEUR et ACTIVITÉ pour réorganiser la fenêtre. Tirer les séparateurs pour redimensionner. **Disposition initiale** restaure l’organisation par défaut.

## Tableur

Double-cliquer une cellule, saisir sa valeur dans l’éditeur puis **Valider la cellule**. Cette validation prépare un changement local. **Appliquer** enregistre l’ensemble du brouillon de cet onglet ; une erreur de contrainte annule tout le lot. **Annuler les modifications** efface le brouillon.

Le menu de l’éditeur distingue **Valeur**, **NULL** et **DEFAULT**. Une valeur texte vide reste distincte de NULL. Les colonnes générées ne sont pas éditables. Les BLOB SQLite acceptent une chaîne hexadécimale.

**+ Ligne** prépare une insertion. Le bouton × du numéro de ligne prépare une suppression ; ↶ la retire du lot. Une clé primaire ou unique non nullable est nécessaire aux mises à jour/suppressions graphiques. Sans clé fiable, utiliser SQL. Les vues sont consultables et se modifient via SQL.

Le premier clic d’en-tête trie par ordre croissant ; un autre inverse le tri. Maj+clic ajoute un critère. Les filtres s’additionnent avec AND ; « contient » traite `%` et `_` comme des caractères littéraux. Les changements de page/tri sont bloqués lorsqu’un brouillon existe.

Si un autre outil modifie les lignes, le brouillon reste affiché. À l’application, PostgreSQL vérifie l’identité et la version MVCC ; SQLite compare la ligne originale pendant une transaction d’écriture. Un conflit exige une relecture et une décision de l’utilisateur.

## SQL

Chaque onglet SQL possède sa propre connexion. Ctrl+Entrée ou **Exécuter** lance le texte sélectionné, sinon le script entier. **Insérer un objet** propose les tables et colonnes connues avec échappement des identifiants. L’historique est limité à la session courante.

Les instructions sont transmises au moteur, sans découpage naïf par point-virgule. Utiliser `BEGIN`, `COMMIT` et `ROLLBACK` pour les transactions explicites. Les effets transactionnels d’un script suivent les règles du moteur. Fermer l’onglet ferme sa session et annule toute transaction non validée.

Le plafond concerne le total des lignes **affichées** dans les différents résultats ; au maximum 128 jeux de résultats sont conservés. Le reste est consommé sans être accumulé. Ce plafond ne transforme pas le SQL et ne limite pas les lignes modifiées. **Arrêter** demande l’annulation au moteur ; une opération déjà validée ne peut pas être annulée rétroactivement. Une coupure réseau ne provoque jamais la répétition d’une écriture.

## MCD

- Glisser l’en-tête d’une table pour la déplacer ; Maj+clic pour sélectionner plusieurs tables.
- Glisser le fond pour déplacer la vue ; molette pour zoomer autour du pointeur ; **Cadrer** pour recentrer la sélection ou l’ensemble.
- Cliquer un lien pour sélectionner son tracé. Double-cliquer un segment ajoute un point de passage. Glisser un point pour le déplacer ; double-cliquer ce point le retire.
- Les poignées rouges des extrémités permettent de déplacer l’ancrage le long des côtés d’une table. Les colonnes reliées restent inchangées ; leur nom est disponible dans l’infobulle du lien.
- Sélectionner des tables, nommer le domaine puis **Encadrer la sélection**. Déplacer l’en-tête du groupe déplace ses tables ; la poignée inférieure redimensionne le cadre sans changer ses membres.
- Le menu **Groupes** renomme, sélectionne ou modifie explicitement les membres d’un groupe et ouvre sa fenêtre.
- **Ouvrir la sélection ↗** ouvre une fenêtre native supplémentaire. Les positions sont partagées ; zoom et cadrage sont indépendants. Les relations sortantes sont signalées.
- ↶ et ↷ annulent/rétablissent les changements de placement, de tracé et de groupes. Les cardinalités et relations de la base ne sont pas éditables dans cette vue.

PK indique une clé primaire, FK une clé étrangère, UQ l’appartenance à une clé unique, et NULL une colonne nullable. Pour une clé composite, UQ sur une colonne ne signifie pas qu’elle est unique à elle seule : consulter l’inspecteur.

Une FK donne une cardinalité 0..1 ou 1..1 vers sa référence et 0..1 ou 0..N en sens inverse selon l’unicité. Une obligation métier « au moins un enfant » n’est pas déduite sans contrainte qui la prouve.
