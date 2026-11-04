# Vérifications de la pile Python/Qt/React

Date : 4 novembre 2026. Les résultats concernent cette branche, pas un déploiement de production.

## Résultats obtenus

- Python 3.12.14 ; PySide6 6.12.0 ; Pydantic 2.14.1 ; OpenAI SDK 2.54.0 ; FastAPI 0.162.0 ; Vite 8.3.5. Versions npm exactes dans package-lock.json, versions Python observées dans requirements-tested.txt.
- `python -m pytest -q` : **40 tests passés**, y compris import/suppression GitHub, reprise de publication Notion et préservation des modifications manuelles.
- `npm run test --prefix apps/web` : **6 tests passés**, dont parcours interactif réponse/correction/progression et absence de connexion privée simulée.
- `npm run build --prefix apps/web` : compilation TypeScript et bundle Vite réussis, service worker statique généré.
- `python scripts/check-public.py` : frontière publique vérifiée, uniquement fixture fictive et aucun chemin de contenu privé dans les assets.
- Application Qt lancée avec plateforme offscreen : cinq vues rendues et parcours question/correction testé.
- Chaîne fictive de première : quatre sources, quatre fiches et douze questions, références et IDs communs aux clients.
- AppImage x86_64 construite (environ 95 MiB), démarrée hors écran pendant 8 secondes sans erreur Python, puis arrêtée par timeout contrôlé.
- Quatre PDF A4 générés ; texte sélectionnable extrait avec PyMuPDF. Rendu SES inspecté visuellement : couleurs, quadrillage, flèches, blocs et références lisibles.

Les connecteurs Notion/GitHub sont testés avec des transports simulés. Aucune clé API de modèle ou intégration REST personnelle n’a été utilisée pour transcrire un vrai devoir. Le générateur de fixtures est explicitement extractif.

## Matrice des scénarios

| Scénario | Preuve automatisée |
| --- | --- |
| Manuscrit original, archivage | test_handwritten_level_one_preserves_original |
| Source modifiée, ID stable | test_modify_source_regenerates_stable_fiche |
| Devoir nouveau | test_homework_generates_without_unjustified_level_one |
| Fiche et questions, quatre matières | test_end_to_end_four_subjects_schema_exports |
| Réutilisation d’une fiche validée | test_validated_level_two_reused_with_originals |
| Réinjection sans boucle | test_accepting_generated_fiche_reinjects_once_without_loop |
| Doublons | test_semantic_duplicate_filter |
| Aucun changement | test_no_change_no_duplicate_no_regeneration |
| Interruption/reprise | test_resume_after_interruption |
| Mauvaise formule reconnue | test_formula_uncertainty_blocks_questions |
| Corrections manuelles | test_manual_question_preserved_proposal_created, test_manual_transcription_preserved |
| Aucune donnée privée publique | test_public_github_repository_rejected_before_any_write, test_private_fixture_cannot_enter_public_build |
| Exécutions concurrentes | test_collision_no_partial_mutation |
| Écriture externe échouée | test_failed_publication_retried_without_regeneration, test_notion_failed_write_not_checkpointed |
| Même banque Linux/web | test_desktop_and_web_share_identical_questions, data.test.ts |

CI GitHub également réussie : https://github.com/CinnamonVII/study-platform/actions/runs/41812467336 (commit 6fb670c12c286b4dee3062a78499108cb3f8dcab).

AppImage livrée : SHA-256 `e74e4393b13db50a55a0d1c5dd865a472cc9116529e23f7b54dad8a065a263c4`.

## À vérifier avec de vrais accès

- Création/mise à jour dans la vraie destination Notion et téléchargement d’une pièce jointe du workspace.
- Modèle multimodal sélectionné, qualité réelle des formules manuscrites, coûts et limites API.
- Authentification HTTPS et reverse proxy sur l’hôte personnel.
- Runner privé durable et exécution réellement planifiée ; création confirmée de la tâche ChatGPT.
- Déploiement GitHub Pages après fusion de la PR ; aucune nouvelle URL n’est présumée déployée.
- Paquet Arch sur CachyOS, thème Breeze/Wayland et lancement AppImage sur la machine cible.
- Inspection visuelle web dans un vrai navigateur : l’accès au serveur de prévisualisation local n’a pas abouti dans cet environnement, malgré la compilation et les tests DOM réussis.

## Fonctions qui restent limitées

La déduplication utilise une similarité lexicale, pas des embeddings. L’éditeur est un éditeur JSON avancé, pas encore un formulaire riche par notion. L’authentification est monoutilisateur par jeton ; OAuth et comptes multiples ne sont pas implémentés. Les formules complexes restent en notation textuelle dans Qt/PDF ; le web rend `$...$` via KaTeX. Le mapping de propriétés Notion personnalisées n’est pas encore implémenté. La suppression d’une publication GitHub déjà fusionnée et la publication publique volontaire de contenus scolaires ne sont pas automatisées.
