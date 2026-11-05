# Automatisation sans stockage persistant

Les workflows utilisent des runners `ubuntu-latest` jetables hébergés par GitHub. Aucun service ni runner n'est installé sur le Steam Deck. Aucun cache, base de données ou artifact scolaire n'est créé. Les journaux et résumés ordinaires de GitHub Actions restent consultables sur GitHub.

## Références Git

`fetch-status.yml` s'exécute chaque heure à la minute 17 et à la demande. Il récupère les branches distantes et indique leurs commits d'avance/de retard par rapport à la branche par défaut. Il ne fusionne rien et ne met pas à jour une copie de travail personnelle. Les références récupérées disparaissent avec le runner ; seul le rapport GitHub est conservé.

## Accès Notion

`notion-check.yml` s'exécute à 7 h et 7 h 30, fuseau `Europe/Paris`, et à la demande. Il effectue uniquement `GET /v1/pages/{id}` sur chaque page explicitement autorisée. Il ne parcourt pas les pages enfants ni les liens et ne télécharge pas les pièces jointes. Il ne conserve pas les réponses et ne journalise ni titres, ni IDs, ni URLs privées. Le résumé contient uniquement le nombre de pages contrôlées et le nombre d'échecs.

Configurer dans Settings → Secrets and variables → Actions du dépôt :

- Secret de dépôt `NOTION_TOKEN` : token d'une connexion interne Notion, avec lecture seule. Ne pas l'ajouter seulement à l'environnement `study-private`, que ce workflow n'utilise pas.
- Secret de dépôt `NOTION_CHECK_PAGE_IDS` : tableau JSON non vide d'IDs de pages autorisées. Autoriser chaque page sélectionnée via ses connexions Notion. La liste n'élargit pas les permissions du token.

Exemple : `["aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"]`. Remplacer cet ID fictif par celui de chaque page choisie. Chaque page est contrôlée individuellement.

Sans les deux secrets, le résultat indique explicitement `NOT CONFIGURED` sans prétendre avoir contrôlé Notion. Une configuration invalide, un accès refusé, une page archivée ou une erreur réseau fait échouer le contrôle. Une erreur d'accès n'est pas interprétée comme une suppression de source.

Ce contrôle ne détecte pas les changements depuis un passage précédent, ne synchronise pas les documents et ne crée aucune fiche. Génération, validation, exports et publication nécessitent les checkpoints du moteur Rust ; leur workflow `daily-sync.yml` reste manuel, sans cron. Les contenus et pièces jointes Notion restent intacts.

## Vérification et gestion

Les deux workflows peuvent être lancés depuis l'onglet Actions → workflow → Run workflow. Une exécution manuelle réussie vérifie l'exécution présente ; la ponctualité des futurs passages reste dépendante de GitHub. Une erreur configurée apparaîtra comme un run échoué dans Actions. Les contrôles Notion ne produisent aucune notification dans ChatGPT.

Ne pas déclencher le pipeline privé pour remplacer le contrôle Notion : il exige un runner privé et un état partagé durable. Il n'existe aucune tâche ChatGPT créée par ces fichiers.

Références : [planification GitHub](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onschedule), [lecture de métadonnées Notion](https://developers.notion.com/reference/retrieve-a-page).
