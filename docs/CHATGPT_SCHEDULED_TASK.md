# Tâche ChatGPT à 7 h, Europe/Paris

La tâche n’a pas été créée pendant cette implémentation. Le prompt prêt à copier est `prompts/daily_agent.md`. Il faut lui fournir les liens du guide et du modèle visuel approuvés ainsi que les destinations privées effectivement autorisées. Ne pas inclure de secrets dans le prompt.

## Créer et vérifier

1. Dans une conversation ChatGPT ayant accès à Notion et au dépôt `CinnamonVII/study-platform`, tester manuellement le prompt.
2. Vérifier que l’agent peut lire le dépôt privé de données et l’état/rapport nécessaires. Une connexion à un dépôt public ne donne pas implicitement l’accès à un dépôt privé.
3. Si un outil de déclenchement de workflow ou d’appel au backend est disponible et autorisé, le tester. Sinon, laisser GitHub Actions exécuter le moteur et demander à ChatGPT d’en assurer le suivi. Ne pas copier le jeton serveur dans le prompt ou le dépôt.
4. Demander la création d’une tâche quotidienne à **07:00 Europe/Paris** avec ce prompt. Vérifier le fuseau et la confirmation de création dans la plateforme.
5. Ouvrir les premières exécutions et contrôler les autorisations, les PR et les validations. La disponibilité des outils, des tâches et des écritures dépend du compte et de la plateforme.

Récurrence indicative :

```text
BEGIN:VEVENT
DTSTART;TZID=Europe/Paris:20261105T070000
RRULE:FREQ=DAILY;BYHOUR=7;BYMINUTE=0;BYSECOND=0
END:VEVENT
```

La date initiale est un exemple pour le lendemain de l’implémentation ; l’adapter à la date réelle de création. L’heure locale doit rester 7 h aux changements été/hiver.

## Ne pas confondre les deux exécuteurs

| Exécuteur | Responsabilité | Configuration |
| --- | --- | --- |
| ChatGPT | orchestration autorisée, vérifications, suivi et rapport | connexions disponibles dans la conversation/tâche |
| GitHub Actions | moteur autonome, génération API et publications configurées | runner privé persistant et secrets propres |
| API privée | même moteur depuis les applications | authentification serveur et même `STUDY_STATE` |

Le verrou de l’état privé protège le moteur. La concurrence GitHub complète ce verrou mais ne le remplace pas. Si ChatGPT ne peut pas utiliser la même voie, il ne doit pas produire en parallèle une autre banque de questions.

Documentation vérifiée le 4 novembre 2026 : https://learn.chatgpt.com/docs/automations et https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#schedule. Les tâches ChatGPT web ne garantissent pas la conservation d’un dossier de travail local entre les exécutions.
