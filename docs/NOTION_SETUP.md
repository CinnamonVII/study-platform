# Accès Notion

Le 3 novembre 2026, les pages suivantes ont été lues via la connexion Notion autorisée, avec `instructions_version="0.0.6"` :

- [Guide](https://app.notion.com/p/40d8a5b36c39816d93c1d1d1c59cd398).
- [Révisions](https://app.notion.com/p/40d8a5b36c39811bac06fd4980c98cca).
- [Présentation commune](https://app.notion.com/p/40d8a5b36c3981309367d3dbdcfe14d4).
- [SES](https://app.notion.com/p/40d8a5b36c3981b9835af0591a0269f4) et [EC3](https://app.notion.com/p/40d8a5b36c398195aea8dcff36c26483).

Aucune page ni base n’a été créée, déplacée ou modifiée pendant le développement. Les autres matières doivent être sélectionnées dans leurs emplacements existants avant toute écriture. Ne pas inclure Memories, About, secrets ou historiques de conversation parmi les sources de révision. Ne pas confondre ce pipeline de 7 h avec la routine de mémoire de 19 h mentionnée dans le guide.

La connexion interactive ChatGPT ne fournit pas de token au programme autonome. Créer une intégration interne Notion, l’inviter seulement aux pages nécessaires et définir `NOTION_TOKEN` sur l’hôte privé et dans l’environnement GitHub Actions autorisé. Lecture des blocs pour l’ingestion, insertion de pages pour la publication. Le moteur n’a besoin d’aucune création de base de données.

Le paramètre `instructions_version` concerne les outils MCP qui le proposent. L’API REST utilise un en-tête distinct `Notion-Version: 2025-09-03` ; ne pas lui envoyer la version du guide.

## Configuration explicite

Copier `config.example.json` vers un fichier privé. Exemple de forme, avec IDs à sélectionner réellement :

```json
{
  "notion_sources": [{
    "page_id": "ID_PAGE_SOURCE_EXISTANTE",
    "subject": "SES",
    "chapter": "EC3",
    "kind": "methode",
    "level": 1,
    "reason": "Document rédigé personnellement, classement confirmé",
    "references": []
  }],
  "notion_destinations": {"SES/EC3": "ID_CHAPITRE_OU_METHODE_EXISTANT"},
  "github_repository": "CinnamonVII/study-platform-data"
}
```

Ne pas utiliser cet exemple pour classer arbitrairement les fiches SES actuelles en niveau 1. Il illustre le format. Une source dérivée doit être de niveau 2 avec ses références : `document_id`, `page`, `quote`, `region` (null si inconnue). Une provenance non vérifiée reste `null`.

Chaque page autorisée est lue entièrement, avec ses blocs descendants ; les pages enfants exigent leur propre entrée. Cela évite d’élargir implicitement les accès. Une page vide retirée ou une pièce jointe disparue est marquée supprimée après une lecture complète. Une erreur 403/404 est une erreur d’accès à vérifier, pas une preuve de suppression.

Les pièces jointes Notion hébergées sur les domaines Notion/S3 reconnus sont acceptées. Un hébergement externe inhabituel exige un import local explicite. Les sorties Notion sont de nouvelles versions identifiées : titres rouges, définitions vertes, explications bleues, comparaisons en tableau, liens et citations conservés. Les versions précédentes et leurs pièces jointes restent intactes.

Le prompt de présentation embarqué dans `prompts/presentation.md` est une copie du texte Notion consulté. Reconsulter la page de référence avant une évolution du modèle de fiche ; le moteur autonome ne réécrit jamais ses propres instructions depuis un document source.
