# Publication des versions (J8, ADR-0016 D11)

Contrats posés par le lot L0 ; pipeline `.github/workflows/release.yml` et `cargo xtask release-check` écrits par le lot L1 (« à blanc » dès la vague 2), porte du web en L13, vérification finale en L16.

## 1. Artefacts

Une release `vX.Y.Z[-pre]` porte exactement cinq fichiers (C-REL-08) :

| Fichier | Contenu |
|---|---|
| `planrs-<v>-macos-universal.dmg` | `planrs.app` universel (x86_64 + arm64, signé ad hoc) avec `planrs-desktop` ; la CLI `planrs` (CLI et serveur MCP) dans `cli/planrs` du DMG |
| `planrs-<v>-windows-x86_64.zip` | `planrs-desktop.exe` (sous-système fenêtré), `planrs.exe` (console), `README.txt` ; non signé (SmartScreen) |
| `planrs-<v>-linux-x86_64.tar.gz` | `planrs-desktop`, `planrs`, `planrs.desktop`, icône ; construit en conteneur `debian:bookworm` (glibc ≤ 2.36) |
| `planrs-<v>-web.tar.gz` | `dist` de trunk (`index.html`, `.wasm`, JS de colle généré par wasm-bindgen) |
| `SHA256SUMS` | Empreintes SHA-256 des quatre fichiers ci-dessus |

Tags `v*-test.*` ⇒ brouillon (non public) ; autres pré-versions ⇒ pré-version ; jetons `contents: write`, `id-token: write`, `attestations: write` au seul job `publish`.

## 2. Contrats (C-REL)

| Id | Contrat | Lot |
|---|---|---|
| C-REL-01 | Tag `vX.Y.Z[-pre]` = `version` du workspace = première section de `CHANGELOG.md` | L1 |
| C-REL-02 | macOS : `lipo -archs` des deux binaires = « x86_64 arm64 » ; `codesign --verify --deep --strict` vert ; `hdiutil verify` vert | L1 |
| C-REL-03 | `planrs --version` et `planrs-desktop --version` impriment `planrs <version>` sans fenêtre, sur les 3 OS | L1 |
| C-REL-04 | Linux : plus haute version `GLIBC_` requise (`objdump -T`) ≤ 2.36 | L1 |
| C-REL-05 | Web : `.wasm` compressé gzip -9 ≤ 3 Mio | L1 (mesure), L13 (porte) |
| C-REL-06 | Web : `chromium --headless` sur `dist` servi en HTTP ⇒ attribut `data-planrs-started="1"` dans le DOM (posé par Rust au démarrage) | L1, L13 |
| C-REL-07 | `sha256sum -c SHA256SUMS` vert sur les artefacts téléchargés de la release | L1 |
| C-REL-08 | `gh release view --json assets` liste exactement les cinq fichiers du § 1 ; `gh attestation verify` vert sur l'un d'eux | L1 |

## 3. Budget du web

Mesure du lot L0 (sonde `cdylib` wasm32, `opt-level = "s"`, LTO) : krilla seule 509 Kio gzip -9, `raster-images` + 73 Kio ; polices Inter du dépôt 2 × 87 Kio (non compressées). Le budget de C-REL-05 décide du logo du PDF (L7).
