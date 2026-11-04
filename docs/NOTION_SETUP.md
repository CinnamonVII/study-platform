# Configuration Notion et données privées

Le guide et le modèle visuel désignés dans la mission ont été lus le 4 novembre 2026. Le point d’entrée existant classe les révisions par matière, puis chapitre/méthode. La configuration livrée ne crée aucune base et ne parcourt pas tout le workspace. Les identifiants privés doivent être placés dans le fichier de configuration privé, pas dans les assets web.

## Connexion REST du moteur

La connexion Notion de ChatGPT permet à un agent autorisé de lire/écrire dans ChatGPT ; elle ne fournit pas automatiquement un token à Python ou à GitHub Actions. Créer une intégration Notion interne pour ce moteur et lui partager uniquement les pages nécessaires. Autoriser la lecture ; ajouter insertion/mise à jour seulement pour publier les fiches dans des destinations existantes. Aucun accès aux utilisateurs ou à d’autres espaces n’est nécessaire.

Le moteur utilise l’API REST Notion avec `Notion-Version: 2026-03-11`. Le paramètre `instructions_version: "0.0.6"` appartient aux outils MCP qui le proposent : il figure dans le prompt quotidien, pas dans un champ REST non reconnu.

Copier `config.python.example.json` dans un fichier privé, puis configurer les sources :

```json
{
  "notion_sources": [
    {
      "page_id": "ID_PAGE_SOURCE_EXISTANTE",
      "subject": "SES",
      "chapter": "Marché concurrentiel",
      "kind": "cours",
      "level": null,
      "provenance_reason": "",
      "status": "pending"
    }
  ],
  "notion_destinations": {"SES": "ID_DESTINATION_EXISTANTE_SES"},
  "excluded_pages": ["ID_PAGE_A_EXCLURE"],
  "github_repository": "CinnamonVII/NOM_DEPOT_DE_DONNEES_PRIVE",
  "allowed_origins": ["https://cinnamonvii.github.io"],
  "attachment_hosts": ["prod-files-secure.s3.us-west-2.amazonaws.com", "s3.us-west-2.amazonaws.com"],
  "max_document_bytes": 20000000
}
```

Remplacer les exemples par des pages effectivement partagées avec l’intégration. Ajouter chaque page source séparément : une page enfant n’est pas implicitement autorisée. Ajouter une autre matière uniquement si sa destination existe et a été approuvée. Les exclusions priment sur l’allowlist.

Attribuer manuellement le niveau 1 avec une justification si la source est bien personnelle. Une fiche de l’enseignant n’est pas automatiquement un original personnel ; une synthèse IA reste niveau 2. Les transcriptions et classifications peuvent être corrigées dans les deux interfaces. Aucune nouvelle propriété Notion n’est créée ; la classification de l’application vit dans le snapshot privé. La lecture d’une propriété Notion de classification personnalisée nécessite encore un mapping spécifique, non implémenté.

## Pièces jointes et contrôle

Les originaux PNG/JPG/PDF/DOCX/TXT/Markdown sont téléchargés sans redirection, depuis les seuls hôtes HTTPS autorisés, avec une limite de taille. Si le stockage Notion utilise un autre hôte, vérifier le vrai domaine puis l’ajouter explicitement ; ne pas mettre de wildcard global. Les fichiers conservés sur le PC, seulement mentionnés par un chemin dans Notion, ne deviennent pas accessibles au serveur : les joindre à une page ou les importer avec la CLI.

Lancer d’abord une prévisualisation :

```sh
python -m engine.cli daily --preview
```

Puis une synchronisation complète :

```sh
python -m engine.cli daily
```

Corriger les incertitudes et valider les sources avant leurs fiches/questions. Une fiche publiée utilise une page stable `Fiche [fiche-id]` dans la destination existante ; ses blocs sont mis à jour en conservant les modifications humaines détectées. Aucun déplacement, réorganisation ou partage de page n’est exécuté.

## Vérification nécessaire avec l’intégration réelle

Les tests utilisent des connecteurs simulés. Vérifier une lecture paginée, une pièce jointe, la création de fiche, puis une mise à jour et une erreur d’accès dans une destination d’essai existante et autorisée. Ce dépôt ne prétend pas que la configuration REST privée est déjà active.

## Documents GitHub en entrée

Optionnel : `github_sources` est une liste de fichiers explicitement autorisés, par exemple :

```json
{"repository":"CinnamonVII/DEPOT_PRIVE","path":"sources/suites.md","ref":"main","subject":"Mathématiques","chapter":"Suites","kind":"cours","level":1,"provenance_reason":"Mes notes personnelles"}
```

Le `STUDY_CONTENT_TOKEN` doit avoir accès en lecture à ces seuls dépôts. Le moteur vérifie un arbre Git complet, puis le blob exact ; un arbre tronqué ou un problème d’accès interrompt le scan sans prétendre que les sources ont été supprimées. Les originaux restent archivés par empreinte. Une source retirée de l’allowlist est exclue du prochain inventaire, avec invalidation locale de ses dérivés.
