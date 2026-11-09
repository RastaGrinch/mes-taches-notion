# Contribuer

Les corrections et propositions d’amélioration sont bienvenues.

En proposant une contribution, vous acceptez qu’elle soit distribuée sous la licence [GPL-3.0-only](LICENSE) du projet.

## Préparer une modification

1. Créer une branche à partir de `main`.
2. Installer les dépendances avec `npm ci`.
3. Lancer l’application avec `npm run tauri dev`.
4. Limiter chaque proposition à un changement cohérent et documenté.

## Vérifier avant une pull request

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

Pour une modification d’interface, vérifier au minimum un thème clair, un thème sombre et une fenêtre proche de la largeur minimale.

## Secrets et données personnelles

Ne jamais ajouter au dépôt :

- un jeton Notion ;
- une base SQLite locale ;
- un identifiant personnel de source de données ;
- des tâches ou captures contenant des données privées ;
- les caches et dossiers de compilation.

Utiliser des UUID fictifs dans les tests et la documentation.
