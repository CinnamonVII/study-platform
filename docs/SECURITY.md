# Confidentialité

Les sources, transcriptions, corrections, PDF et progression restent dans `STUDY_STATE`, hors du dépôt de code et des artifacts Pages. Tous les endpoints de l’API vérifient le jeton côté serveur avec une comparaison constante des empreintes. Les réponses privées portent `Cache-Control: no-store, private`. CORS autorise une origine exacte ; il complète l’authentification et ne la remplace pas.

Le client reçoit un jeton personnel uniquement saisi par l’utilisateur. Il n’est ni embarqué dans WebAssembly, ni stocké dans localStorage, ni transmis dans une URL. Utiliser HTTPS et renouveler le jeton depuis le gestionnaire de secrets. L’API est prévue pour une seule personne de confiance : pas de gestion multiutilisateur, de récupération de compte ou de facturation. Pour l’exposer sur Internet, placer un proxy TLS et un contrôle d’accès réseau/Cloudflare Access devant le service, avec limitation de débit et de taille des requêtes. Ne pas exposer directement le port HTTP.

`OPENAI_API_KEY`, `NOTION_TOKEN`, `GITHUB_TOKEN` et `STUDY_TOKEN` sont des secrets du service et du runner privé. La construction Pages n’en utilise aucun. Les programmes n’affichent pas les corps des erreurs d’API externes susceptibles de contenir des notes. Le serveur écrit des diagnostics opérationnels dans son journal privé ; protéger aussi ces journaux.

Les documents autorisés et leurs pièces jointes sont transmis au fournisseur d’IA lorsque vous déclenchez la génération ou activez la routine. Le paramètre Responses `store=false` est utilisé ; il ne constitue pas une promesse de rétention nulle du fournisseur. Choisir le modèle et la politique de données du compte avant d’activer les sources réelles.

Le connecteur GitHub vérifie que le dépôt choisi est privé avant une écriture. Il produit une PR brouillon et ne fusionne rien. Le dépôt de code peut être public puisqu’il contient seulement les fixtures. Garder un dépôt de contenu privé séparé. La publication publique des cours n’est pas implémentée : elle reste désactivée, même si un document porte `private=false`.

Les contenus pédagogiques sont traités comme des données, jamais comme des instructions autorisant une action. Les citations sont vérifiées dans les sources courantes à la génération et à l’acceptation. Les incertitudes bloquent l’entraînement et la publication. Le mécanisme est une aide à la validation, pas une preuve de justesse d’une réponse IA.

## Sauvegarde et reprise

Sauvegarder les octets originaux, la base SQLite avec sa sauvegarde cohérente (commande `.backup` ou service arrêté), et le fichier de configuration. Les archives sont immuables par convention applicative ; un administrateur du disque peut toujours les modifier. Le serveur et le runner utilisent le même utilisateur ou un groupe privé autorisé et le même répertoire persistant local. Le verrou ne s’étend pas à plusieurs hôtes ou à un stockage réseau.

Une tentative Notion ambiguë reste dans `publications` avec le statut `attempting`. Vérifier les enfants de la destination et leur marqueur avant toute reprise administrative. Ne pas effacer ce marqueur pour tenter aveuglément une nouvelle création. Une PR GitHub non validée s’annule en fermant la PR sans fusion ; le moteur n’a aucune commande destructive automatique sur les sources Notion.
