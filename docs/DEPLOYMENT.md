# Installation et déploiement

## État réel

Le code est développé dans un nouveau dépôt local `/home/deck/study-platform`. Aucun autre projet n’a été modifié. Aucun dépôt distant, serveur Internet, GitHub Pages ou tâche ChatGPT n’est supposé créé. Les URL ci-dessous sont des destinations à configurer, pas des déploiements vérifiés.

## Linux / CachyOS

```sh
cargo build --locked --release -p study-ui -p study-engine -p study-server
install -Dm755 target/release/study-ui "$HOME/.local/bin/study-ui"
install -Dm755 target/release/study-engine "$HOME/.local/bin/study-engine"
install -Dm755 target/release/study-server "$HOME/.local/bin/study-server"
install -Dm644 packaging/revisions.desktop "$HOME/.local/share/applications/revisions.desktop"
install -Dm644 packaging/revisions.svg "$HOME/.local/share/icons/hicolor/scalable/apps/revisions.svg"
```

`study-ui` démarre en démonstration ; Synchronisation permet de connecter le serveur. Pour le moteur, installer Typst et DejaVu Sans, puis configurer `.env.example` dans le gestionnaire de secrets ou le service. Rust n’est nécessaire qu’à la compilation. SQLite est embarqué.

`scripts/appimage.sh` construit l’application graphique avec [linuxdeploy](https://github.com/linuxdeploy/linuxdeploy/releases). Télécharger le binaire officiel, vérifier son empreinte, puis exécuter :

```sh
APPIMAGE_EXTRACT_AND_RUN=1 LINUXDEPLOY=/chemin/linuxdeploy-x86_64.AppImage bash scripts/appimage.sh
```

Le paquet Arch `packaging/PKGBUILD` est préparé pour un futur tag `v0.1.0` du dépôt dédié ; il ne doit pas être présenté comme publié dans AUR. Compiler l’AppImage sur la distribution la plus ancienne visée : AppImage n’efface pas les exigences de version de glibc. Les essais locaux ont lieu sur cette machine SteamOS, pas sur un CachyOS distant.

## Backend privé

Créer un utilisateur de service `study`, copier le binaire dans `/opt/study-platform`, les secrets en mode 0600 dans `/etc/study-platform/secrets.env`, et la configuration des sources dans `/etc/study-platform/config.json`. Le service fourni utilise `/var/lib/study-platform` comme état persistant. Installer Typst dans un répertoire lisible par le service et renseigner `TYPST_BIN` si nécessaire.

Le fichier `packaging/study-platform.service` est un modèle systemd à adapter. Le backend écoute par défaut sur `127.0.0.1:8787`. Le publier derrière un proxy HTTPS, configurer `STUDY_ORIGIN=https://cinnamonvii.github.io` pour Pages ou l’origine réellement utilisée. Les jetons d’API et les données ne doivent jamais être placés sur Pages.

Exemple de test local : `STUDY_BIND=127.0.0.1:8787`, `STUDY_ORIGIN=http://127.0.0.1:8088`, un `STUDY_TOKEN` aléatoire et le même `STUDY_STATE` que le moteur. Dans l’application, saisir cette adresse et ce jeton. Ne pas employer un jeton d’exemple en production.

## Dépôts GitHub dédiés

Créer `CinnamonVII/study-platform` pour le code et, si les notes doivent être archivées sur GitHub, `CinnamonVII/study-platform-data` **privé**. Initialiser ce dernier avec au moins un commit, sans source manuscrite. Ne pas sélectionner un autre projet existant.

Configurer le remote du nouveau dépôt local puis pousser le code. Activer Settings → Pages → GitHub Actions. Le workflow Pages compile seulement `crates/ui` et ses fixtures ; `check-static.sh` refuse les formats privés dans `dist`. La disponibilité de Pages depuis un dépôt privé dépend du compte GitHub : conserver la séparation code public / contenu privé si nécessaire.

Le workflow produit l’URL effective dans son environnement `github-pages`. Il faut attendre un vrai run réussi avant de déclarer cette URL déployée.

## Routine quotidienne indépendante de ChatGPT

**Planification actuelle :** seuls les fetchs Git et les contrôles d'accès Notion sans stockage sont automatisés sur runners GitHub hébergés. Voir [STATELESS_AUTOMATION.md](STATELESS_AUTOMATION.md). Le pipeline ci-dessous reste une option **manuelle**, non activée automatiquement.

Une alternative sans machine personnelle permanente est préparée dans [FULL_CLOUD_PIPELINE.md](FULL_CLOUD_PIPELINE.md) : runner jetable, état durable dans un dépôt privé, checkpoints avant publication. Cette alternative exige la confirmation du stockage privé et une configuration API réelle avant activation.

Installer un runner GitHub Actions **privé, persistant**, étiqueté `study-private`, sur le même hôte que l’API. Il doit accéder au même `STUDY_STATE`, à Rust, Typst, aux polices et au réseau des API. Restreindre le runner au dépôt dédié et aux branches approuvées ; les tests de pull requests utilisent les runners hébergés et ne doivent pas exécuter de code non approuvé sur cet hôte.

Créer l’environnement GitHub `study-private` avec ces secrets :

| Secret | Usage minimal |
| --- | --- |
| `NOTION_TOKEN` | Lecture des sources autorisées et insertion sous les destinations autorisées |
| `OPENAI_API_KEY` | API du modèle multimodal, distincte de l’abonnement ChatGPT |
| `STUDY_CONTENT_TOKEN` | Token GitHub limité au dépôt de contenu privé : Contents et Pull requests en écriture |
| `STUDY_CONFIG_JSON` | Contenu JSON de la configuration privée des pages |

Variables : `STUDY_STATE` (chemin absolu commun, préexistant) et `OPENAI_MODEL` (modèle compatible avec Responses, images/PDF et JSON Schema, choisi dans le compte API). Le workflow expose uniquement `workflow_dispatch`, sans cron. Les validations humaines permettent une publication ultérieure même sans nouvelles sources.

La tâche ChatGPT utilise les connexions qu’elle possède réellement ; pour lancer un traitement, elle doit déclencher ce même workflow si l’app GitHub le permet. Elle ne doit pas lancer un deuxième moteur avec un autre état. Le verrou local couvre les appels API et le workflow. Les retards de planification de la plateforme restent possibles.

## Références vérifiées

[Planification GitHub Actions et fuseaux](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onschedule), [lecture paginée des blocs Notion](https://developers.notion.com/reference/get-block-children), [sorties structurées Responses](https://developers.openai.com/api/docs/guides/structured-outputs), [eframe natif et web](https://docs.rs/eframe/0.33.3/eframe/).
