# Préparer la tâche ChatGPT de 7 h

Aucune tâche n’a été créée par ce projet. Copier `prompts/daily_agent.md` dans une conversation ChatGPT et demander : « Crée une tâche récurrente chaque jour à 7 h, fuseau Europe/Paris, avec ces instructions. » Vérifier ensuite la confirmation de la plateforme, la prochaine exécution et le fuseau dans la liste des tâches. L’heure est une heure locale qui suit le changement été/hiver.

Connecter seulement les pages Notion et les dépôts dédiés nécessaires. La possibilité d’utiliser des apps et de déclencher un workflow dépend de la surface ChatGPT et de ses permissions : un prompt n’ajoute aucune capacité. Le programme Rust n’hérite pas de ces connexions ; son intégration Notion et son compte API sont configurés séparément.

L’orchestrateur doit d’abord consulter le dernier run GitHub et le journal privé si son accès le permet. Un déclenchement manuel `daily-sync.yml` réutilise le même état que le passage programmé. Si l’app GitHub n’expose pas le déclenchement ou si une écriture exige validation, il doit le dire et fournir le lien du workflow au lieu de prétendre avoir effectué l’action. Le workflow de 7 h et sa reprise de 7 h 30 assurent le traitement indépendamment de ChatGPT, tant que le runner privé est en ligne.

En l’absence de changement, aucun doublon ne doit être créé. Un bref compte rendu de suivi est permis. Les sorties incertaines restent à valider dans l’application ; elles ne sont ni acceptées ni publiées automatiquement par ChatGPT.

La [documentation officielle des tâches](https://learn.chatgpt.com/docs/dots/tasks-and-memory) décrit leur gestion. Il faut confirmer les capacités disponibles dans le compte au moment de l’activation ; aucune exécution future n’a été testée ici.
