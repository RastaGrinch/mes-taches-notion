# Sécurité

## Signaler une vulnérabilité

Ne publiez pas de jeton, de base locale ou de détail exploitable dans une issue publique. Utilisez de préférence un signalement privé GitHub ou contactez le mainteneur du dépôt avant de rendre le problème public.

Indiquez la version concernée, la plateforme, les étapes de reproduction et l’impact observé. N’incluez aucun jeton réel.

## Stockage des secrets

Le jeton Notion est conservé par le coffre natif du système d’exploitation. Il n’est pas enregistré dans le frontend, la base SQLite ou les journaux de l’application.

## Périmètre d’accès Notion

Une connexion doit recevoir uniquement les capacités **Read content**, **Update content** et **Insert content**, puis être ajoutée à la page de la base concernée. Aucun accès aux informations utilisateur n’est requis.

