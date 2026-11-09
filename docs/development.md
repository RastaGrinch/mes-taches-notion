# Développement et compilation

## Technologies

- React 19 et TypeScript pour l’interface ;
- Vite pour le développement et la compilation du frontend ;
- Tauri 2 pour l’application de bureau ;
- Rust pour la logique locale et les appels Notion ;
- SQLite pour le cache et la file d’attente ;
- coffre système natif pour le jeton Notion.

## Prérequis

Installer Node.js LTS, npm et Rust stable, puis suivre les [prérequis Tauri](https://v2.tauri.app/start/prerequisites/) de la plateforme.

Sous Windows, installer Visual Studio Build Tools 2022 avec **Développement Desktop en C++**. WebView2 est généralement déjà présent sur Windows 10 et 11.

Sous macOS, installer les outils de ligne de commande Xcode :

```bash
xcode-select --install
```

## Lancer le projet

```bash
git clone https://github.com/RastaGrinch/mes-taches-notion.git
cd mes-taches-notion
npm ci
npm run tauri dev
```

`npm run dev` ouvre uniquement l’aperçu du frontend dans un navigateur. La synchronisation, SQLite et le coffre système nécessitent `npm run tauri dev`.

## Vérifications

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

## Construire sous Windows

Depuis PowerShell :

```powershell
.\scripts\build-windows.ps1
```

L’installateur est créé dans `src-tauri\target\release\bundle\`.

## Construire sous macOS

```bash
./scripts/build-macos.sh
```

Le paquet est créé dans `src-tauri/target/release/bundle/`.

## Compilation croisée Windows depuis macOS

Le script `scripts/build-windows-from-macos.sh` utilise LLVM, NSIS et `cargo-xwin`. Cette méthode est pratique pour produire un premier installateur, mais Tauri la considère comme expérimentale. Tester l’installateur final sur Windows.

## Fichiers générés

Les dossiers suivants ne doivent pas être versionnés :

- `node_modules/` ;
- `dist/` ;
- `src-tauri/target/` ;
- `.cargo-home/`, `.npm-cache/` et `.xwin-cache/` ;
- les bases SQLite et les installateurs compilés.

