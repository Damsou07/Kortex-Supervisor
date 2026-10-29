# Utilisation

## Connexions et navigation

Utiliser **Nouvelle connexion** pour PostgreSQL, ou **Ouvrir un fichier SQLite** pour un fichier `.db`, `.sqlite` ou `.sqlite3`. Tester un serveur distant avec TLS ; pour une autorité privée, sélectionner son certificat PEM. Les tunnels SSH sont créés extérieurement, puis utilisés comme une connexion locale.

Les profils sont conservés, mais ne se reconnectent pas automatiquement au démarrage. Un profil peut être modifié lorsqu’il n’est pas ouvert. **Oublier** retire le profil et son secret mémorisé ; fermer d’abord ses onglets. Pour changer de base PostgreSQL, le menu **Bases disponibles** ouvre un profil distinct afin de préserver le contexte des onglets existants.

Cliquer sur une table pour ouvrir ses données. Les onglets permettent de revenir instantanément aux pages déjà chargées. Leur infobulle et le bandeau de contexte indiquent la connexion utilisée. L’inspecteur affiche les détails de la table sélectionnée.

Déplacer les onglets de panneaux EXPLORATEUR, ESPACE DE TRAVAIL, INSPECTEUR et ACTIVITÉ pour réorganiser la fenêtre. Tirer les séparateurs pour redimensionner. **Réinitialiser les panneaux** restaure l’organisation de la fenêtre. Cette commande est indépendante de la disposition du MCD.

Le titre d’un onglet et sa croix appartiennent au même cadre. Le point jaune indique un brouillon, même lorsque le titre est tronqué. Ctrl+Tab et Ctrl+Maj+Tab changent d’onglet ; Ctrl+W ferme l’onglet actif avec les protections habituelles des brouillons et sessions SQL.

## Tableur

Les en-têtes affichent le nom et le type à droite. Les types connus sont raccourcis pour l’affichage (`varchar(255)`, `timestamptz`) ; l’infobulle conserve le type déclaré complet. **Légende** explique les couleurs par famille : nombres bleus, textes verts, booléens violets, dates orange, UUID turquoise, JSON jaune, binaires bruns et autres types gris. Le même code est utilisé dans le MCD et l’inspecteur.

Les badges PK (jaune), FK (cyan) et UQ (mauve) occupent des emplacements séparés. Ils peuvent se cumuler. Une couleur de type ne remplace pas une indication de modification, suppression, NULL ou erreur.

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
- En mode **Déplacement** (H), glisser le fond pour déplacer la vue. En mode **Sélection** (V), tracer un rectangle pour sélectionner les tables qu’il touche ; Maj ajoute à la sélection. Le bouton du milieu déplace la vue dans les deux modes. Les relations ne sont jamais membres d’une sélection rectangulaire.
- Molette pour zoomer autour du pointeur ; **Cadrer** (F) pour recentrer la sélection ou l’ensemble. Les colonnes sont dessinées entre 12 % et 250 %. Sous 60 %, survoler une table affiche une fiche lisible et défilante avec les types complets et contraintes.
- Cliquer un lien pour sélectionner son tracé. Double-cliquer un segment ajoute un point de passage. Glisser un point pour le déplacer ; double-cliquer ce point le retire.
- Les poignées rouges des extrémités permettent de déplacer l’ancrage le long des côtés d’une table. Les colonnes reliées restent inchangées ; leur nom est disponible dans l’infobulle du lien.
- Ouvrir **Groupes → + Nouveau groupe** (ou Ctrl+G depuis le canvas), nommer le groupe puis sélectionner les tables au rectangle ou dans la liste filtrable. **Valider le groupe** crée le cadre ; **Annuler** abandonne le brouillon. Un nom non vide et au moins une table présente sont nécessaires.
- Le panneau **Groupes** recherche, sélectionne, modifie ou supprime un cadre et ouvre sa fenêtre. Les modifications sont validées en une seule étape annulable. Supprimer le cadre conserve ses tables. Déplacer son en-tête déplace ses membres ; la poignée inférieure redimensionne le cadre.
- **Disposition par défaut** rapproche les tables liées et les membres des groupes, espace les arrivées des relations, évite les tables et réduit les croisements. Les ponts visuels distinguent les croisements restants. Les noms, couleurs et membres des groupes sont conservés ; les ajustements manuels des liens concernés sont réinitialisés. La commande est annulable. Les préférences déjà enregistrées ne sont pas réorganisées automatiquement.
- **Ouvrir la sélection ↗** ouvre une fenêtre native supplémentaire. Les positions sont partagées ; zoom et cadrage sont indépendants. Les relations sortantes sont signalées.
- ↶ et ↷ (Ctrl+Z/Ctrl+Y sur le canvas) annulent/rétablissent les changements de placement, de tracé et de groupes. Ctrl+A sélectionne toutes les tables visibles ; Échap annule le rectangle ou le brouillon de groupe. Les raccourcis du canvas respectent la saisie dans les champs.

Dans une fenêtre détachée, **Disposition par défaut** concerne ses tables visibles ; les autres restent fixes. Les groupes partageant des tables conservent leurs appartenances : le placement les rapproche dans un bloc commun. Les ancrages visuels peuvent être décalés par rapport à la colonne référencée pour distinguer les flèches ; l’infobulle du lien précise les colonnes réellement reliées.

PK indique une clé primaire, FK une clé étrangère, UQ l’appartenance à une clé unique, et NULL une colonne nullable. Pour une clé composite, UQ sur une colonne ne signifie pas qu’elle est unique à elle seule : consulter l’inspecteur.

Une FK donne une cardinalité 0..1 ou 1..1 vers sa référence et 0..1 ou 0..N en sens inverse selon l’unicité. Une obligation métier « au moins un enfant » n’est pas déduite sans contrainte qui la prouve.

## Raccourcis centralisés

**F1** ouvre **Paramètres → Raccourcis clavier**. Le catalogue se filtre par touche, action ou catégorie et répertorie les commandes générales, SQL, MCD, tableur, champs texte et gestes souris. Les combinaisons sont consultables ; elles ne sont pas personnalisables.
