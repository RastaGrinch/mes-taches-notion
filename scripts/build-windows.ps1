$ErrorActionPreference = "Stop"

Write-Host "Vérification des prérequis..."
foreach ($command in @("node", "npm", "rustc", "cargo")) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
        throw "Commande manquante : $command. Consultez https://v2.tauri.app/start/prerequisites/"
    }
}

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "Installation exacte des dépendances JavaScript..."
npm ci

Write-Host "Exécution des tests Rust..."
cargo test --manifest-path src-tauri/Cargo.toml

Write-Host "Construction de l'installateur Windows..."
npm run tauri build

Write-Host "Terminé. Les installateurs se trouvent dans src-tauri\target\release\bundle\."
