# Pipeline privé sur runners jetables

La routine complète a besoin d'un état durable : inventaire, empreintes, corrections humaines, propositions, tentatives de publication et octets originaux. Le modèle préparé ici utilise un dépôt GitHub **privé** distinct ; aucun serveur personnel permanent n'est nécessaire. Ce modèle n'est pas activé par sa présence dans le dépôt public.

## Activation

1. Confirmer la création du dépôt privé dédié `CinnamonVII/study-platform-data`. Ne pas réutiliser un dépôt public ni un projet sans rapport.
2. Y installer `packaging/private-daily.yml` sous `.github/workflows/daily.yml`, `packaging/private-data.gitignore` sous `.gitignore`, et une configuration `config.json` basée sur `config.example.json`.
3. Sélectionner explicitement chaque page source et sa provenance réelle. `level: null` conserve une provenance inconnue ; ne pas déclarer un contenu IA comme original de niveau 1. Associer chaque destination à un chapitre ou une méthode existante sous Révisions. Une page enfant nécessite sa propre entrée. Les sources et pièces jointes restent intactes.
4. Configurer dans ce dépôt privé les secrets `NOTION_TOKEN`, `OPENAI_API_KEY`, et la variable `OPENAI_MODEL`. Le token Notion doit lire les sources et pouvoir insérer des pages sous les destinations autorisées si la publication Notion est souhaitée. Une connexion en lecture seule ne peut pas publier. Le token GitHub automatique reste limité à ce dépôt privé.
5. Autoriser la création de pull requests par GitHub Actions dans les réglages Actions du dépôt. Définir `STUDY_CODE_REF` sur le SHA complet d'une version revue du code. Définir `ENABLE_STUDY_PIPELINE=true` seulement après validation de ces réglages.
6. Lancer manuellement et inspecter un premier passage, puis une reprise identique : les documents inchangés ne doivent pas être régénérés. Les passages suivants sont prévus à 7 h et 7 h 30 Europe/Paris.

## État et reprise

Le runner travaille depuis le dépôt privé avec le chemin relatif `state`, identique entre passages. Il restaure `state/checkpoint.sqlite`. Le script de checkpoint utilise la sauvegarde cohérente SQLite, puis commite les archives, exports et checkpoint dans le dépôt privé. Il n'utilise ni cache évictable ni artifact public pour stocker les données.

Le moteur invoque ce même checkpoint après chaque document traité et avant les écritures de publication. Si le push échoue, le traitement est interrompu avant de passer au document suivant ou à l'écriture externe. Une réponse ambiguë reste à réconcilier selon les marqueurs du moteur. Un arrêt entre une réponse IA et sa sauvegarde durable peut encore entraîner la répétition de cet appel ; les documents précédents déjà sauvegardés sont conservés. Les échecs de traitement sont également sauvegardés. Ne pas connecter simultanément un backend qui modifie une autre copie d'état : la concurrence GitHub sérialise les passages de cette routine, pas des copies indépendantes.

Les archives complètes et les PDF augmentent la taille de l'historique Git. Respecter les limites de taille de GitHub ; passer à un stockage privé dédié si le volume l'exige. Une suppression du dernier arbre Git ne supprime pas les anciennes versions de l'historique.

## Résultat attendu et limites

Le moteur importe les pages explicitement configurées et leurs pièces jointes prises en charge, détecte les modifications/retraits après une lecture complète, génère des fiches et questions **à valider**, puis exporte et publie les contenus **déjà acceptés**. Les corrections humaines sont préservées. La routine ne valide pas ses propres sorties. Les PDFs et propositions GitHub restent privés ; la publication GitHub est une PR brouillon, sans fusion automatique.

Le prompt Notion courant exige deux versions composées dans Notion et une inspection visuelle du PDF réellement exporté depuis Notion. Le renderer Typst existant est un export technique distinct et ne satisfait pas à lui seul cette exigence. Aucun PDF produit par cette routine ne doit être annoncé comme export Notion validé. L'export natif Notion et son contrôle visuel restent à connecter avant de promettre l'automatisation complète de cette étape.

Ce modèle constitue une préparation de déploiement. Une compilation ou un test local ne prouve pas l'import, la génération API, l'export PDF ou la publication réels. Les accès, le stockage privé et la configuration du modèle doivent être confirmés avant activation.
