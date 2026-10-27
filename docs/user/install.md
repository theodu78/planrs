# Installer planrs

Chaque release (`https://github.com/theodu78/planrs/releases`) porte cinq fichiers : un par plateforme et `SHA256SUMS`. Chaque archive contient deux programmes :

- `planrs-desktop` : l'application (fenêtre) ;
- `planrs` : la ligne de commande et le serveur MCP (`planrs mcp`), pour piloter un planning depuis un agent IA.

`planrs --version` et `planrs-desktop --version` impriment `planrs <version>`.

La première version publique est la pré-version `1.0.0-rc.1` (marquée « Pre-release » sur GitHub) : `<version>` vaut alors `1.0.0-rc.1` dans les noms de fichiers ci-dessous.

Les binaires ne sont ni notarisés (macOS) ni signés par un certificat (Windows) : chaque système demande une confirmation au premier lancement, détaillée ci-dessous.

## Vérifier le téléchargement

Placer les fichiers téléchargés et `SHA256SUMS` dans un même dossier, puis :

- macOS : `shasum -a 256 -c SHA256SUMS --ignore-missing`
- Linux : `sha256sum -c SHA256SUMS --ignore-missing`
- Windows (PowerShell) : `Get-FileHash planrs-<version>-windows-x86_64.zip -Algorithm SHA256`, à comparer à la ligne correspondante de `SHA256SUMS`.

Avec la CLI GitHub, la provenance (fichier construit par le workflow `release.yml` du dépôt) se vérifie par `gh attestation verify <fichier> --repo theodu78/planrs`.

## macOS (11 Big Sur ou plus récent, Apple Silicon et Intel)

Fichier : `planrs-<version>-macos-universal.dmg`.

1. Ouvrir le DMG et glisser `planrs.app` dans `Applications`.
2. Premier lancement (macOS 15 et plus récents) : ouvrir `planrs.app` ; macOS refuse, car l'application n'est pas notarisée. Ouvrir **Réglages Système → Confidentialité et sécurité**, descendre jusqu'au message sur planrs et cliquer **« Ouvrir quand même »**, puis confirmer. Les lancements suivants sont normaux. (Sur les versions plus anciennes, clic droit sur l'application puis « Ouvrir » suffit.)
3. Ligne de commande : copier `cli/planrs` du DMG, par exemple dans `/usr/local/bin`, puis lever la quarantaine : `xattr -d com.apple.quarantine /usr/local/bin/planrs`.

L'application est signée « ad hoc » (signature locale, sans identité Apple) : elle est intègre, mais Gatekeeper ne connaît pas son auteur.

## Windows (10 et 11, x86_64)

Fichier : `planrs-<version>-windows-x86_64.zip`.

1. Extraire le zip (clic droit → « Extraire tout… »).
2. Lancer `planrs-desktop.exe`. Microsoft Defender SmartScreen affiche « Windows a protégé votre ordinateur » : cliquer **« Informations complémentaires »**, puis **« Exécuter quand même »**.
3. `planrs.exe` se lance depuis un terminal (PowerShell ou Invite de commandes).

`planrs-desktop.exe --version` n'affiche rien dans un terminal (programme fenêtré) : utiliser `planrs.exe --version`, ou `planrs-desktop.exe --version | more`.

## Linux (x86_64)

Fichier : `planrs-<version>-linux-x86_64.tar.gz`.

Prérequis :
- **glibc 2.36 ou plus récente** (Debian 12, Ubuntu 22.10, Fedora 37 et suivantes) ; `ldd --version` l'indique ;
- une session Wayland, ou X11 avec `libxkbcommon-x11` ;
- un pilote **Vulkan ou OpenGL** (paquets Mesa de la distribution) ;
- pour « Ouvrir… » et « Enregistrer sous… » : le **portail XDG** (`xdg-desktop-portal` et l'implémentation du bureau, présents sur GNOME et KDE).

Installation pour l'utilisateur courant :

```sh
tar xzf planrs-<version>-linux-x86_64.tar.gz
cd planrs-<version>-linux-x86_64
install -Dm755 planrs-desktop planrs -t ~/.local/bin
install -Dm644 planrs.desktop -t ~/.local/share/applications
install -Dm644 planrs.png ~/.local/share/icons/hicolor/256x256/apps/planrs.png
```

`~/.local/bin` doit être dans le `PATH` pour que le lanceur trouve `planrs-desktop`.

## Web

Fichier : `planrs-<version>-web.tar.gz` (dossier `dist` : `index.html`, `.wasm` et le JS de colle généré).

Les fichiers se servent **en HTTP** (un navigateur refuse de charger le `.wasm` depuis `file://`) :

```sh
tar xzf planrs-<version>-web.tar.gz
python3 -m http.server -d dist 8080
```

puis ouvrir `http://localhost:8080` dans un navigateur récent (WebGPU ou WebGL2). La version web ne relie pas `planrs mcp` et ne garde pas de fichiers récents ; le planning ouvert reste dans l'onglet.
