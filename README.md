# Révisions · première

**Architecture de référence : full stack Rust.** L’application Linux, le client web WebAssembly, le backend privé et le moteur documentaire poursuivent le workspace `crates/`. Voir le [cahier des charges de référence](docs/CAHIER_DES_CHARGES.md), confirmé le 4 novembre 2026.

La variante Python/PySide6 + React/TypeScript développée dans la PR nº 1 est conservée avec ses tests et son AppImage. Elle reste une variante ; elle ne remplace pas la cible Rust. Les commandes et workflows Python/React présentés ci-dessous concernent cette variante. Les workflows de cette branche restent à isoler ou à réaligner avant de modifier le déploiement principal Rust.

Les sources Notion restent privées. GitHub Pages publie uniquement l’interface statique et une démonstration fictive. Aucun secret de connexion ChatGPT n’est automatiquement réutilisé par l’application.

## Plateforme principale Rust

- `crates/domain` : contrats et pédagogie partagés, Serde/Schemars et répétition espacée.
- `crates/engine` : moteur documentaire, connecteurs et exports.
- `crates/server` : API privée Axum.
- `crates/ui` : interface egui/eframe commune Linux et WebAssembly.

La suite du développement doit améliorer ces composants en priorité. Le [plan Rust existant](docs/superpowers/plans/2026-11-03-platform.md) reste disponible. La documentation détaillée Python/Qt/React de cette branche décrit la variante conservée.

## Variante Python/Qt + React conservée

## Essayer sur CachyOS

Installer Python, Node.js 24 et les bibliothèques graphiques de son système. Depuis un clone du dépôt :

```sh
python -m venv .venv
source .venv/bin/activate
python -m pip install -e '.[desktop,test]'
python -m apps.desktop.main
```

Le bureau propose quatre matières, quatre fiches et douze questions fictives, sans clé API. Il utilise de vrais widgets Qt, pas un navigateur intégré. Sur KDE, les contrôles suivent le thème Qt disponible.

```sh
npm ci --prefix apps/web
npm run dev --prefix apps/web
```

Ouvrir l’URL affichée, avec le préfixe `/study-platform/`. La démonstration mémorise uniquement les notes SM-2 fictives dans le navigateur. En mode privé, contenus et jeton restent en mémoire, la progression est enregistrée par l’API.

## Chaîne verticale complète

```sh
STUDY_STATE=state/demo-python python -m engine.cli demo
STUDY_STATE=state/demo-python python -m engine.cli export
```

Ce générateur de démonstration est **déterministe et extractif**, sans prétendre reconnaître des manuscrits. Les exports Markdown, LaTeX et PDF A4 avec texte sélectionnable se trouvent dans `state/demo-python/exports/<fiche-id>/`.

Pour ses propres documents, configurer une API multimodale puis importer :

```sh
export STUDY_STATE="$HOME/.local/share/study-platform/private"
python -m engine.cli import /chemin/cahier.png --subject Mathématiques --chapter Suites --kind cours --level 1 --reason 'Photo de mon cahier personnel'
python -m engine.cli generate
```

Le niveau 1 exige une justification. Un document non classé garde `level: null`. Un fichier importé reste archivé sans modification. Les formules incertaines empêchent la génération jusqu’à leur correction et validation. Les contenus générés restent en attente.

## Connecter les données privées

1. Copier `config.python.example.json` vers un `config.json` privé et remplir l’allowlist selon [NOTION_SETUP.md](docs/NOTION_SETUP.md). Aucun parcours global du workspace n’est effectué.
2. Configurer les variables de `.env.python.example` dans l’environnement du serveur. Ce fichier d’exemple n’est pas chargé automatiquement. Générer un jeton personnel avec `openssl rand -hex 32` et ne jamais le commiter.
3. Lancer l’API :

```sh
python -m uvicorn backend.api.main:app --host 127.0.0.1 --port 8787
```

4. Dans Synchronisation, saisir `http://127.0.0.1:8787` pour un essai local, ou l’adresse HTTPS du serveur, puis son jeton. Charger les données.
5. Prévisualiser, synchroniser, corriger les transcriptions et valider les sources, puis les fiches et questions. Relancer pour publier les contenus validés dans les destinations configurées.

Les éditeurs avancés utilisent des champs JSON validés côté serveur. Ils permettent classification, corrections et statuts `pending`, `validated`, `rejected`. La validation utilise une révision optimiste : une modification concurrente impose de recharger avant d’écraser quoi que ce soit.

## Tester et compiler

```sh
python -m pytest -q
python -m engine.cli schema
node scripts/web-schema.mjs
npm run test --prefix apps/web
npm run build --prefix apps/web
python scripts/check-public.py
```

Les scénarios couvrent les manuscrits, changements de source, devoirs, fiches validées, doublons, reprise, formules incertaines, corrections humaines, confidentialité, verrou, écritures externes et contrats communs. [VALIDATION.md](docs/VALIDATION.md) distingue les vérifications réalisées et celles encore nécessaires avec des accès réels.

## Installer et automatiser

- [DEPLOYMENT.md](docs/DEPLOYMENT.md) : AppImage, paquet Arch expérimental, API HTTPS, GitHub Pages et runner privé.
- [CHATGPT_SCHEDULED_TASK.md](docs/CHATGPT_SCHEDULED_TASK.md) : prompt et étapes pour créer la tâche à 7 h, Europe/Paris.
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) : cycle documentaire, dépendances, reprise et formats.
- [SECURITY.md](docs/SECURITY.md) : données privées et permissions minimales.

L’API et le runner doivent partager **le même répertoire `STUDY_STATE` sur le même hôte persistant**. Ils utilisent le même verrou interprocessus. Le workflow GitHub de 7 h fonctionne indépendamment de ChatGPT. Celui de 7 h 30 réconcilie les changements encore absents sans régénérer les jobs déjà réussis.

La présence du code ne crée ni serveur hébergé, ni secrets, ni tâche ChatGPT. Le dépôt public refuse les contenus scolaires privés : utiliser un dépôt **de données privé distinct** pour les sorties GitHub, ou désactiver ce connecteur.
