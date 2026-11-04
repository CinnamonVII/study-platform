# Installation et déploiement

## Linux / CachyOS

Depuis le dépôt, créer un venv puis installer `python -m pip install -e '.[desktop]'`. Lancer `python -m apps.desktop.main`. L’application est un vrai client Qt et peut utiliser une API distante. La démonstration fonctionne sans serveur.

Sur CachyOS/Arch, les dépendances système usuelles sont Python, Node.js/npm, bibliothèques Qt/X11/Wayland et polices DejaVu. Le paquet Qt de pip inclut Qt ; le thème Breeze de KDE dépend des plugins présents. Ne pas mélanger pip dans le Python système : utiliser le venv. `packaging/PKGBUILD-python` propose un paquet Arch expérimental à construire depuis `packaging/` ; il n’a pas été exécuté sur CachyOS dans cette session.

### AppImage

Le script utilise PyInstaller puis appimagetool. Télécharger une version d’appimagetool depuis son dépôt officiel et vérifier son empreinte, puis :

```sh
python -m pip install -e '.[desktop,package]'
APPIMAGETOOL=/chemin/appimagetool-x86_64.AppImage bash scripts/appimage-python.sh
chmod +x dist/Revisions-0.2.0-x86_64.AppImage
./dist/Revisions-0.2.0-x86_64.AppImage
```

Si FUSE est indisponible, utiliser `--appimage-extract-and-run`. La construction actuelle cible x86_64 et la glibc de l’hôte de build ; CachyOS récent est visé. Pour distribuer à des Linux plus anciens, construire dans une image avec une glibc de référence plus ancienne. Aucun certificat de signature éditeur n’est configuré.

## API privée persistante

Choix livré : FastAPI sur un serveur Linux persistant, avec reverse proxy HTTPS. Cloudflare Workers n’est pas nécessaire. Ne pas publier le port 8787 directement.

1. Créer un utilisateur système `study`, un checkout `/srv/study-platform/code`, un venv `/srv/study-platform/venv` et un répertoire `/srv/study-platform/private` possédé par cet utilisateur, mode 700.
2. Installer `pip install -e /srv/study-platform/code` dans le venv.
3. Écrire `/srv/study-platform/config.json` selon NOTION_SETUP, mode 600.
4. Écrire `/srv/study-platform/secrets.env`, mode 600, à partir de `.env.python.example`. Ce sont des variables système, jamais des assets web.
5. Installer `packaging/study-python.service`, adapter les chemins, puis activer le service. La configuration est un exemple d’administration à appliquer sur son propre serveur, pas une action réalisée ici.
6. Configurer Caddy/nginx devant `127.0.0.1:8787`, TLS valide et limite de requêtes. Ajouter exactement l’origine GitHub Pages à `allowed_origins`, sans chemin.
7. Vérifier `/health`, puis l’accès authentifié `/v1/snapshot` et le refus d’une requête sans jeton.

| Variable | Où | Autorisation / usage |
| --- | --- | --- |
| `STUDY_STATE` | serveur et runner | même répertoire durable privé |
| `STUDY_CONFIG` | serveur | chemin du fichier d’allowlist |
| `STUDY_API_TOKEN` | serveur | valeur aléatoire d’au moins 32 caractères ; `openssl rand -hex 32` recommandé |
| `NOTION_TOKEN` | serveur et secret Actions | intégration interne limitée aux sources/destinations |
| `OPENAI_API_KEY` | serveur et secret Actions | compte API avec facturation/crédits et modèle autorisé |
| `OPENAI_MODEL` | serveur et variable Actions | modèle acceptant images/PDF et sorties structurées, choisi explicitement |
| `STUDY_CONTENT_TOKEN` | serveur et secret Actions | fine-grained PAT du seul dépôt privé de données, Contents read/write et Pull requests read/write |

Un abonnement ChatGPT et une connexion MCP ne constituent pas automatiquement des crédits API ou des secrets de runner. Vérifier les accès réels dans ses comptes.

## GitHub de données

Le dépôt de code `CinnamonVII/study-platform` est public. Créer ou sélectionner **séparément** un dépôt privé de données, puis renseigner son nom dans la configuration. Aucun dépôt supplémentaire n’est créé par ce code. Laisser `github_repository` vide pour désactiver la publication GitHub tout en utilisant Notion et l’API.

Les sorties validées sont écrites dans une branche déterministe et une PR brouillon. Les sources privées originales ne sont pas copiées. La fusion reste une action humaine. Une sortie non validée se refuse via son statut ou sa proposition ; elle n’est pas publiée. Une PR déjà publiée se ferme/revoit sur GitHub, les données publiques ne sont jamais révoquées automatiquement.

## GitHub Pages

```sh
npm ci --prefix apps/web
npm run build --prefix apps/web
python scripts/check-public.py
```

Dans Settings → Pages, choisir GitHub Actions. Le workflow `deploy-pages.yml` publie `apps/web/dist` après fusion dans `main` ou déclenchement manuel. Aucun secret Notion/GitHub/IA n’est disponible dans ce job. `STUDY_WEB_BASE` ne contient que le préfixe du dépôt. L’URL habituelle attendue est `https://cinnamonvii.github.io/study-platform/` ; elle ne prouve pas qu’une nouvelle version a été déployée.

La PWA précache uniquement une liste de fichiers statiques construits, jamais les réponses privées. Elle retire ses anciens caches. En mode privé, une connexion réseau au backend reste nécessaire. La déconnexion retire le jeton et les contenus privés de l’interface. Les réponses libres privées ne sont pas enregistrées dans localStorage.

## Actions quotidiennes

Configurer un runner Linux **dédié et persistant** avec les labels `self-hosted`, `linux`, `study-private`, sous le même utilisateur/volume que l’API. Le snapshot et le verrou ne doivent jamais être neufs à chaque exécution. Ne pas y exécuter des jobs de PR publiques ou du code non fiable. Pour une exploitation avec plusieurs contributeurs, préférer un dépôt d’automatisation privé séparé et durcir l’isolation du runner.

Créer l’environnement GitHub `study-private`, puis y mettre les secrets `STUDY_CONFIG_JSON`, `NOTION_TOKEN`, `OPENAI_API_KEY`, `STUDY_CONTENT_TOKEN` si activé, et les variables `STUDY_STATE`, `OPENAI_MODEL`. La valeur JSON contient l’allowlist privée. Les logs du pipeline restent sur le serveur. Les logs d’installation de dépendances ne contiennent pas les documents.

Le workflow utilise `cron: '0 7 * * *'`, `timezone: 'Europe/Paris'`, puis un passage à 7 h 30 et `workflow_dispatch`. La documentation GitHub actuelle prend en charge le champ timezone. Les horaires peuvent être retardés par GitHub : ce n’est pas un ordonnanceur temps réel. Le traitement compare toujours l’inventaire et les jobs déjà réussis ; le second passage récupère les changements manqués et ne régénère pas les mêmes contenus.

## Sauvegarde / exploitation

Sauvegarder de façon chiffrée `STUDY_STATE`, la configuration et les secrets séparément. Les fichiers originaux sont immuables mais le disque ne se sauvegarde pas tout seul. Surveiller le dernier succès et les jobs échoués dans Synchronisation. Un verrou bloqué par un processus vivant impose d’attendre ou d’inspecter le processus ; ne pas supprimer aveuglément l’état. `flock` se libère à la sortie du processus.
