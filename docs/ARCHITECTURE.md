# Architecture Python / Qt / React, schéma 2.0

## Composants

- `apps/desktop` : widgets Qt natifs PySide6 ; requêtes réseau dans QThreadPool.
- `apps/web` : React 19, TypeScript, Vite, KaTeX, PWA statique. Validation JSON Schema via Ajv 2020.
- `backend/api` : FastAPI, authentification réelle par jeton personnel du propriétaire, autorisations et validations côté serveur.
- `engine` : ingestion, appels multimodaux Responses API, génération structurée Pydantic, classification, règles, export, synchronisation.
- `schemas/snapshot-v2.json` : schéma généré depuis `engine/models.py`. `scripts/web-schema.mjs` génère les types TS. Les deux applications lisent le même JSON.
- `fixtures/demo/snapshot-v2.json` : seules données embarquées dans les clients publics, entièrement fictives.

Le backend Python est choisi plutôt qu’un Worker pour partager directement le moteur, les PDF, les fichiers immuables et le verrou. Aucun ensemble de microservices ni base cloud supplémentaire n’est nécessaire. Il peut être placé derrière un reverse proxy HTTPS ou un tunnel authentifié, sans exposer son port d’origine.

## Données et provenance

Le type documentaire est indépendant du niveau. `level=null` signifie non attribué ; le niveau 1 requiert `provenance_reason`. `derived=true` impose le niveau 2 et au moins une source. L’original binaire est archivé sous son SHA-256, distinct de sa transcription modifiable. Les versions précédentes restent disponibles sur le serveur.

Un document Notion est importé seulement s’il figure dans l’allowlist. Ses blocs imbriqués sont lus avec pagination ; les pages enfants exigent leur propre entrée. Les pièces jointes prises en charge sont archivées et deviennent des documents séparés. PNG/JPEG/PDF sont soumis directement au modèle ; DOCX, TXT et Markdown sont extraits localement. URL signée et texte original restent privés.

## Cycle

1. Lecture complète de l’inventaire autorisé, sans fenêtre arbitraire de 24 h.
2. Empreinte des blocs et des fichiers ; différences de contenu, classification et suppressions explicites.
3. Acquisition du verrou `flock`, import et checkpoint atomique avec `fsync` puis `replace`.
4. Transcription structurée ; tout passage incertain reste bloqué pour la génération.
5. Regroupement par matière et chapitre ; ID de fiche stable, empreinte des dépendances et version du générateur.
6. Génération de fiche/questions en attente. Vérification des références et extraits. Déduplication lexicale normalisée et similarité de chaînes ; ce n’est pas un moteur d’embeddings sémantiques complet.
7. Réinjection des fiches acceptées comme documents dérivés de niveau 2. Leur empreinte exclut les dates de publication ; les originaux restent dans le contexte.
8. Publication uniquement des contenus validés dont les sources sont validées, actuelles et sans incertitude.
9. Rapport privé ; `last_success` avance seulement après absence d’erreur. Les jobs terminés ne sont pas rejoués.

Une erreur pendant la découverte interrompt le scan : un 403/404 ne constitue jamais une preuve de suppression. Seuls l’archivage explicite et l’absence dans un inventaire complet réussi autorisent un tombstone local. Les originaux archivés ne sont jamais supprimés.

## Reprise et corrections humaines

Chaque étape de génération est un job indexé par empreinte. Les transcriptions sont checkpointées avant la génération. Une publication échouée reste rejouable sans régénérer. Notion retrouve une page créée avant un crash grâce à son titre stable, puis réconcilie les blocs gérés. Une modification manuelle détectée devient une proposition et n’est pas écrasée. Les mises à jour plus courtes demandent de revoir les blocs en trop.

Dans l’état local, les corrections humaines ont `manual=true`. Un changement de source conserve la transcription manuelle et ouvre une incertitude. Les fiches et questions corrigées restent en place ; les nouvelles versions deviennent des propositions. Des sorties affectées sont marquées `stale` et retirées de l’entraînement jusqu’à revalidation.

Le connecteur GitHub exige un dépôt de données privé, construit des blobs et un arbre, puis un commit atomique et une branche déterministe. Il ouvre une PR brouillon ; il ne fusionne pas automatiquement. Les branches modifiées humainement sont conservées et signalées. Aucun document original binaire n’est ajouté au dépôt.

## Répétition espacée

SM-2, notes 0 à 5. Échec sous 3 : remise à 1 jour et zéro répétition. Réussites : 1 jour, 6 jours, puis intervalle multiplié par le facteur de facilité, borné à 1,3. L’implémentation Python et celle de démonstration TS sont comparées sur des vecteurs communs. Les réponses libres sont autoévaluées avec la correction et la grille ; aucune notation IA non configurée n’est simulée.

## Limites assumées

Monoutilisateur. Le jeton du propriétaire donne accès à tout son espace autorisé ; aucun partage/multitenant. L’état est un snapshot JSON atomique, avec verrou global, adapté à une utilisation personnelle. Les exports LaTeX reproduisent le texte et les tableaux avec échappement sûr ; les expressions `$...$` sont rendues par KaTeX sur le web, tandis que Qt et le PDF Python gardent leur notation textuelle. Les formules LaTeX complexes nécessitent encore un moteur de rendu mathématique PDF/Qt dédié.
