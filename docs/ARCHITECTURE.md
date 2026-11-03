# Architecture

La demande initiale Python/React a été remplacée par une architecture Rust à la demande de l’utilisateur. Quatre crates évitent la duplication entre interfaces. Serde refuse les champs inconnus des contenus pédagogiques ; Schemars exporte le contrat 1.0. SQLite conserve l’état privé. Axum expose une API JSON authentifiée à egui natif et à egui compilé en WebAssembly. Typst, lui aussi développé en Rust, produit les PDF sélectionnables à partir d’une source `.typ` échappée.

## Flux

Source explicitement autorisée → copie immuable et empreinte → transcription multimodale avec incertitudes → fiche de niveau 2 et questions en attente → correction humaine → validation → proposition GitHub privée et version Notion additive.

L’ID d’une source locale dépend de son chemin canonique ; les révisions de ses octets sont archivées sous leur empreinte. Déplacer un fichier nécessite de conserver son identifiant avec l’API moteur, sinon l’import représente un nouveau document. Les pages et pièces jointes Notion utilisent leurs IDs stables. Le type documentaire et le niveau de provenance sont indépendants. Aucun classement automatique en niveau 1.

Les sources textuelles Notion sont archivées en JSON par empreinte dans `originals/`. Les pièces jointes sont téléchargées sans remplacer les originaux. Les pages enfants ne sont pas parcourues implicitement : chaque page doit être autorisée et classée dans la configuration. Le connecteur parcourt les blocs imbriqués et la pagination. Une lecture incomplète échoue ; elle n’est jamais interprétée comme une suppression. L’inventaire d’une lecture complète permet de retirer les sources et pièces jointes disparues.

## Fiabilité

Le verrou de fichier du noyau couvre lecture Notion et traitement sur un même hôte. Un processus vivant conserve son verrou ; un crash le libère. Les checkpoints sont enregistrés après transaction des résultats, par document. Les nouvelles sources sont enregistrées et les anciennes sorties invalidées avant les appels IA : une panne ne laisse pas une ancienne correction considérée comme actuelle. Une relance reprend les documents inachevés sans fenêtre arbitraire de 24 h.

Les corrections et classifications manuelles sont protégées. Les propositions entrantes restent dans `candidate-*`. Leur application conserve l’ancienne version dans `history-*` et exige une nouvelle validation. Les versions obsolètes restent consultables pour audit, mais quittent l’entraînement. Une fiche validée de niveau 2 doit être vérifiée dans ses originaux acceptés de niveau 1 ; le moteur transmet également ces textes au générateur. Les sorties du moteur ne sont pas automatiquement redécouvertes comme de nouvelles sources, ce qui évite les boucles.

Notion : nouvelles pages versionnées uniquement sous des destinations existantes, jamais de suppression de blocs ou pièces jointes. Un marqueur déterministe retrouve une écriture déjà réussie. Une tentative ambiguë reste bloquée tant que la présence de la page n’est pas confirmée. GitHub : instantané dans une branche déterministe et PR brouillon ; aucun écrasement de branche et aucune fusion automatique. Le dépôt de contenu doit être privé.

## Répétition espacée

SM-2 : facilité initiale 2,5 ; première réussite à 1 jour, deuxième à 6 jours, suivantes multipliées par la facilité. Une note inférieure à 3 reprend à 1 jour ; facilité minimale 1,3. Les notes 0 à 5 sont une autoévaluation après affichage de la correction ; l’application ne prétend pas noter automatiquement une dissertation ou une preuve. La progression privée est partagée par l’API. En démonstration elle reste temporaire.

## Limites actuelles

Application personnelle à un seul compte et un seul hôte d’état ; pas de verrou distribué entre machines. Le dédoublonnage lexical utilise les mots normalisés et une similarité Jaccard de 0,9 ; il ne remplace pas une recherche sémantique par embeddings. Les régions sources sont conservées dans les citations, mais aucun éditeur d’annotation graphique n’est fourni. Les formules sont conservées comme texte dans le lecteur et l’export principal : pas de moteur complet de rendu LaTeX dans egui. Les imports locaux passent par la CLI, pas par un glisser-déposer web. Le lecteur d’écran web du canvas nécessite encore un audit dédié. Le service worker conserve le shell public, jamais les notes privées hors ligne.
