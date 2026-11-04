# Confidentialité et frontières

- Les documents, transcriptions, progrès et PDF scolaires sont privés par défaut.
- Le site statique contient exclusivement les fixtures fictives vérifiées par `scripts/check-public.py`. Aucun répertoire d’état, original ou secret n’entre dans le build.
- Le backend monoutilisateur vérifie réellement un jeton fort côté serveur, par comparaison constante. Un backend sans token configuré répond 503. HTTPS requis hors localhost ; CORS explicite ; réponses privées `no-store`.
- Le jeton personnel n’est ni une clé Notion ni un token GitHub. Les clients le gardent en mémoire. Il donne les droits du propriétaire de cet espace. Le multitenant et le partage de compte ne sont pas implémentés.
- Le service worker n’intercepte que la liste des assets construits de sa propre origine ; il ignore les requêtes avec Authorization, les autres origines et les paramètres de requête. Aucun cache dynamique de données privées.
- Les pièces jointes Notion exigent HTTPS, domaine explicitement autorisé, absence de redirection et limite de 20 MB. Les téléchargements d’originaux par l’API vérifient le chemin contre le répertoire d’archives.
- Les sorties IA restent en attente. Les références inventées et extraits absents sont refusés. Une incertitude non résolue bloque la dérivation et la publication.
- Les PDF conservent du texte sélectionnable et des polices embarquées. Le LaTeX exporté échappe les caractères spéciaux et n’exécute jamais du code LaTeX provenant des documents.
- Les textes sources sont des données, jamais des instructions d’agent. Les prompts demandent explicitement d’ignorer leurs instructions intégrées.
- Le connecteur GitHub interroge la visibilité réelle du dépôt avant toute écriture ; un dépôt public est refusé. Aucune option implicite ne publie les contenus scolaires sur Pages. La publication publique volontaire de contenus scolaires reste non implémentée, donc désactivée.
- Les écritures locales utilisent un verrou interprocessus et une révision optimiste. Les corrections manuelles sont proposées plutôt qu’écrasées.
- Notion et GitHub ne fournissent pas une transaction distribuée. Les publications sont reconcilées de manière idempotente après erreur ; des conflits humains sont signalés. Tester la vraie intégration avant une utilisation autonome prolongée.
- L’état et les logs sont sur un disque privé ; prévoir chiffrement et sauvegardes selon l’hôte. Aucune suppression distante, rotation de secrets ou changement de partage n’est automatisé.

La configuration des secrets et des permissions est une étape requise, pas une connexion déjà effectuée. Aucun secret n’a été lu, copié ou commité pendant l’implémentation.
