# Révisions

Plateforme personnelle de première générale, écrite en Rust. L’application Linux et le site WebAssembly partagent le même code d’interface egui, les mêmes questions JSON et le même algorithme de répétition espacée. Le backend Axum garde les documents privés et orchestre Notion, l’API d’IA et les propositions GitHub.

La démonstration est utilisable sans compte : quatre documents fictifs, quatre fiches et quatre questions couvrent mathématiques, SES, HGGSP et français. Aucun document scolaire réel n’est embarqué dans le site.

## Essayer

Prérequis : Rust stable récent (édition 2024), bibliothèques Linux Wayland/X11, OpenGL et libxkbcommon. Sur CachyOS : `sudo pacman -S --needed rustup base-devel libxkbcommon libx11 wayland mesa`. Pour les PDF, installer [Typst](https://github.com/typst/typst/releases) et la police DejaVu Sans.

```sh
cargo run --locked -p study-ui
```

L’interface Linux utilise egui et des contrôles dessinés en Rust, pas des widgets Qt. Ce choix suit la demande de migration complète vers Rust.

## Démonstration du pipeline

Utiliser un répertoire d’état réservé aux essais :

```sh
STUDY_STATE=state/demo cargo run --locked -p study-engine -- demo
STUDY_STATE=state/demo cargo run --locked -p study-engine -- export
```

`demo` utilise un générateur extractif déterministe, **pas une API d’IA**. Il refuse d’exporter un état contenant des identifiants réels. Les exports se trouvent dans `state/demo/exports/<id>/fiche.{md,tex,typ,pdf}`. Le PDF est produit par Typst ; le `.tex` est une version textuelle alternative à compiler avec XeLaTeX.

## Utiliser les données privées

1. Copier `config.example.json` vers `config.json` et autoriser précisément les pages sources et destinations existantes, selon [NOTION_SETUP.md](docs/NOTION_SETUP.md).
2. Configurer les variables de [.env.example](.env.example) dans l’environnement du backend. Créer le jeton d’accès avec `openssl rand -hex 32`. Ne jamais le commiter.
3. Lancer `cargo run --locked -p study-server` avec `STUDY_STATE` et `STUDY_CONFIG` correctement renseignés.
4. Dans Synchronisation, saisir l’adresse du serveur et le jeton, puis « Charger mes données ». Le jeton reste en mémoire jusqu’à la déconnexion.
5. Synchroniser ; corriger les transcriptions et incertitudes ; valider les sources, puis les fiches et les questions. Les nouveaux contenus restent à valider. Les modifications humaines sont conservées et les mises à jour sont proposées séparément.

Importer un fichier sur l’hôte du moteur :

```sh
cargo run --locked -p study-engine -- import /chemin/cahier.png Mathématiques Suites cours 1 'Photo de mon cahier personnel'
cargo run --locked -p study-engine -- generate
```

Les PNG/JPG/PDF sont envoyés directement au modèle multimodal configuré. Les DOCX, Markdown et TXT sont extraits localement. La provenance inconnue s’écrit `unknown` ; le niveau 1 exige une justification explicite.

## Version web

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.22.1 --locked
cd crates/ui
trunk serve --port 8088
```

Le shell statique est une PWA. Les documents privés ne sont jamais mis en cache par le service worker. La consultation privée exige le backend authentifié HTTPS. Sans backend, seule la démonstration fonctionne. Quelques lignes JavaScript servent au chargement WASM et au service worker ; toute la logique applicative est en Rust.

## Tester

Les groupes sont séparés pour garder les vérifications ciblées :

```sh
cargo test --locked -p study-domain
cargo test --locked -p study-engine --test pipeline
cargo test --locked -p study-engine --lib connectors
cargo test --locked -p study-server --test api
cargo check --locked -p study-ui --target wasm32-unknown-unknown
```

Voir [VALIDATION.md](docs/VALIDATION.md) pour les preuves réellement obtenues et leurs limites.

## Installer et automatiser

[DEPLOYMENT.md](docs/DEPLOYMENT.md) décrit l’installation native, AppImage, le backend privé et GitHub Pages. [STATELESS_AUTOMATION.md](docs/STATELESS_AUTOMATION.md) décrit les tâches hébergées par GitHub : récupération horaire des références Git et contrôle quotidien de l’accès aux pages Notion explicitement autorisées. Elles ne nécessitent aucun stockage persistant ni machine personnelle allumée. Le pipeline privé `daily-sync.yml`, qui nécessite un état durable, reste manuel et sans planification.

Aucune tâche ChatGPT n’est créée par ces workflows. Le contrôle Notion nécessite les secrets documentés ; il ne synchronise ni ne génère de contenu.

[FULL_CLOUD_PIPELINE.md](docs/FULL_CLOUD_PIPELINE.md) présente le déploiement préparé pour la routine complète sur runners jetables avec un dépôt de données privé durable. Ce modèle reste à activer après confirmation du stockage, des sources, du compte API et des capacités de publication Notion. Il ne remplace pas le contrôle visuel obligatoire d'un export PDF natif Notion.

## Organisation

- `crates/domain` : contrats Serde/JSON Schema et SM-2.
- `crates/engine` : ingestion, archives immuables, IA, validation, exports, synchronisation et connecteurs.
- `crates/server` : API privée Axum et autorisations côté serveur.
- `crates/ui` : interface Rust partagée Linux/WebAssembly.
- `fixtures/demo` : documents fictifs et instantané commun.
- `schemas`, `prompts`, `docs`, `packaging`, `.github/workflows` : contrats versionnés, pédagogie, installation et automatisation.

[ARCHITECTURE.md](docs/ARCHITECTURE.md) · [SECURITY.md](docs/SECURITY.md) · [UI_DESIGN.md](docs/UI_DESIGN.md)
