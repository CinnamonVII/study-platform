# Validation et état de livraison

Vérifications exécutées sur cette machine le 3 novembre 2026, avec Rust 1.99.1, eframe 0.33.3, Axum 0.8.9, Typst 0.15.1 et Trunk 0.22.1. Les versions transitives sont figées dans Cargo.lock.

## Tests automatisés

| Groupe | Résultat | Périmètre |
| --- | --- | --- |
| Domaine | 3 tests réussis | SM-2, note invalide, protection de la provenance générée |
| Pipeline | 21 tests réussis | Import manuscrit simulé, modifications, devoirs, fiche et questions, réemploi d’une fiche validée, doublons, absence de changement, reprise, formule incertaine, éditions manuelles, concurrence, retrait, classification, citations, historique |
| Connecteurs | 6 tests réussis | Refus du dépôt public, panne GitHub, écriture Notion ambiguë, suppression de pièces jointes après succès ou interruption, archive Notion immuable |
| API | 5 tests réussis | Accès privé, no-store, CORS, contrat commun Linux/web, refus de progression sur question non validée |

Total : 35 tests, exécutés par groupes ciblés. Le scénario de bout en bout utilise quatre documents fictifs représentatifs des matières de première, un générateur extractif de test, le stockage et les exports. Les tests de reconnaissance manuscrite simulent la réponse du modèle ; ils ne mesurent pas sa précision réelle.

La revue indépendante a conduit à ajouter des régressions pour les pannes au début d’un lot, les anciennes citations, la conservation des classifications et l’inventaire d’un premier passage interrompu. Les contenus de toutes les sources changées sont retirés de l’entraînement avant le premier appel de modèle.

## Compilations et exécutions locales

- Compilation native Linux et cible wasm32-unknown-unknown réussies ; construction Trunk sous `/study-platform/` et vérification des assets statiques.
- Application graphique native démarrée dans la session KDE et rendu observé. AppImage construite avec linuxdeploy, lancée avec extraction intégrée et restée active jusqu’à l’arrêt prévu du test, sans erreur fatale dans son journal. Pas d’essai sur un autre ordinateur CachyOS.
- Site exécuté dans Chromium local : chargement WASM sans erreur JavaScript, captures 1160×820 et 375×812, navigation et correction visibles. Les débordements constatés sur le premier rendu mobile ont été corrigés.
- Connexion réelle de l’interface au backend local authentifié : HTTP 200, quatre documents et quatre questions fictifs, sans erreur JavaScript. La correction d’une question est affichée après action dans l’interface connectée. Ce test n’utilise aucune donnée scolaire réelle.
- Quatre PDF A4 d’une page produits avec Typst, texte extrait avec Poppler et chaque page rendue et inspectée. Exports Markdown, LaTeX et Typst présents. Le contrôle n’atteste pas la qualité d’une future génération IA sur de vrais manuscrits.
- Recherche du jeton d’essai dans les assets Pages : aucune occurrence. Le script de préparation retire uniquement le verrou temporaire Trunk puis refuse les formats non statiques, les PDF et bases SQLite.

## Non validé en production

Aucun appel payant au modèle ni synchronisation REST sur les documents réels n’a été exécuté. Les pages d’organisation Notion ont été consultées avec la connexion interactive, qui n’est pas un token du moteur autonome. Les connecteurs d’écriture ont été testés avec des serveurs simulés. Aucun dépôt distant n’a été créé, aucun commit poussé, aucune PR réelle ouverte, aucun site public déployé et aucune tâche ChatGPT de 7 h confirmée.

Les workflows sont prêts à être configurés, mais n’ont pas de résultat GitHub Actions réel dans cette session. L’AppImage locale n’est pas une publication AUR ni une preuve de portabilité sur toutes les versions de glibc.

## Fonctionnalités restant à étendre

- Audit complet clavier/lecteur d’écran du canvas web ; aucune conformité WCAG globale revendiquée.
- Rendu mathématique LaTeX avancé dans le lecteur et annotations graphiques des régions ; les étapes et régions sont conservées sous forme structurée.
- Import local via CLI ; pas de téléversement/glisser-déposer dans le navigateur.
- Dédoublonnage sémantique par embeddings ; la version actuelle utilise une similarité lexicale explicite.
- Gestion multiutilisateur, progression privée hors ligne et verrou distribué entre hôtes.
- Historique conservé dans SQLite mais sans écran de restauration détaillé. Les propositions se prévisualisent, s’appliquent ou s’écartent ; une PR non fusionnée s’annule depuis GitHub. Pas de suppression automatique d’une page source Notion.
- Actualisation en place d’une ancienne page Notion remplacée par des versions additives, afin de préserver les sources et pièces jointes.

Les actions humaines restantes sont détaillées dans DEPLOYMENT.md, NOTION_SETUP.md et CHATGPT_SCHEDULED_TASK.md.
