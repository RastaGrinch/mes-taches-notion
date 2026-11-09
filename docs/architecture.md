# Architecture

## Composants

L’interface React appelle des commandes Tauri exposées par le backend Rust. Le frontend n’accède jamais directement au jeton Notion.

Le backend Rust gère :

- la base SQLite locale ;
- la création et la modification des tâches ;
- la file d’attente de synchronisation ;
- la lecture et l’écriture du jeton dans le coffre système ;
- les requêtes HTTPS vers l’API Notion.

## Cycle d’une modification

1. L’utilisateur crée ou modifie une tâche.
2. La tâche est immédiatement enregistrée dans SQLite avec l’état `dirty`.
3. L’interface est mise à jour sans attendre Notion.
4. Lors d’une synchronisation, le backend envoie les tâches `dirty` à Notion.
5. Les pages distantes sont ensuite relues et fusionnées dans le cache local.
6. En cas d’erreur réseau, la tâche reste dans la file locale avec le détail de l’erreur.

## Données locales

- **SQLite** : tâches, paramètres non sensibles et état de synchronisation.
- **Coffre système** : jeton Notion.
- **localStorage du WebView** : thèmes visuels choisis pour chaque vue.

## API Notion

Le client utilise la version `2026-03-11` de l’API et les endpoints `data_sources`. Il lit le schéma, interroge les pages de la source et crée ou met à jour les pages avec les propriétés attendues.

## Absence de serveur intermédiaire

L’application communique directement avec Notion depuis l’ordinateur. Il n’existe pas de backend hébergé, de compte propre à l’application ni de télémétrie.

