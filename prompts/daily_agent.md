Chaque jour à 7 h Europe/Paris, suis ma plateforme de révision de première générale : mathématiques, SES, HGGSP, français.

Commence par le guide https://app.notion.com/p/40d8a5b36c39816d93c1d1d1c59cd398, l’entrée Révisions https://app.notion.com/p/40d8a5b36c39811bac06fd4980c98cca et le prompt de présentation https://app.notion.com/p/40d8a5b36c3981309367d3dbdcfe14d4. Utilise instructions_version="0.0.6" pour les outils qui l’acceptent. Respecte les exclusions et les destinations privées. Ne crée aucune base ni arborescence supplémentaire ; ne modifie pas la routine mémoire de 19 h.

Dépôts dédiés à confirmer lors de la configuration : CinnamonVII/study-platform (code) et CinnamonVII/study-platform-data (contenu privé). N’interviens dans aucun autre dépôt.

A. Consulte le dernier passage réussi et les runs en cours du workflow daily-sync.yml. Compare depuis ce passage, sans limite arbitraire de 24 h. Les checkpoints et l’inventaire du moteur font autorité pour les ajouts, modifications, retraits et classifications. Si le passage de 7 h est déjà terminé, consulte ses résultats. S’il tourne encore, ne lance pas un second traitement indépendant.

B. Si nécessaire et si l’app GitHub l’autorise, déclenche workflow_dispatch sur daily-sync.yml, dans la branche approuvée. Ce workflow exécute le moteur Rust sur l’hôte privé et le même état que l’API ; n’invente pas une autre chaîne sans verrou partagé. Sans cette capacité ou si une confirmation est requise, fournis le lien d’exécution manuelle et indique exactement la limite.

C. Vérifie le rapport réel : originaux de niveau 1 explicitement justifiés, transcription multimodale fidèle, conservation des octets sources, passages douteux à valider. Ne transforme pas un passage illisible en connaissance certaine et ne reclasse jamais une fiche IA en original.

D. Les fiches de niveau 2 suivent le modèle Notion commun et citent leurs originaux. Les exports Markdown, PDF A4 sélectionnable, Typst et LaTeX restent privés. Les questions couvrent rappel, compréhension, application et raisonnement, en vérifiant les originaux et les corrections. Une fiche dérivée validée réinjecte ses notions sans boucle de régénération.

E. Ne valide pas les nouvelles sorties à ma place. Vérifie la présence des propositions et des incertitudes dans l’application. Pour les contenus déjà acceptés et techniquement valides, le moteur peut préparer des versions Notion additives et une PR brouillon dans le dépôt privé. Aucun secret, transcription ni PDF privé dans les artifacts Pages. Aucune fusion, suppression de source, modification de partage ou publication publique automatique.

F. Confirme la mise à disposition réelle des données par l’API privée et, si nécessaire, le succès réel du workflow Pages. Un build local n’est pas un déploiement. Ne prétends jamais qu’une API, un workflow, une publication ou une tâche a réussi sans résultat observable.

G. Produis un bref compte rendu : documents détectés, fiches proposées ou acceptées, questions ajoutées ou obsolètes, incertitudes, erreurs, prochaine action humaine et liens exacts des runs/PR/pages. Si aucun changement : termine sans doublon. Signale les autorisations manquantes et la différence entre suivi ChatGPT et pipeline exécuté.
