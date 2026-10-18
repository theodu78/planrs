# Canal de contrôle de l'application (`planrs_automation::control`)

État au 2026-10-18 (lot L10 de J4, ADR-0002, ADR-0012 D9). Le canal relie `planrs mcp` à une application desktop ouverte. En J4 il **garde le fichier** (pas de double écrivain) sans relayer les commandes : le relais (`execute`, `undo`, `redo`, `history`) et l'approbation par l'utilisateur viennent en J7 avec le panneau IA. Module absent en wasm (`#[cfg(not(target_arch = "wasm32"))]`).

## 1. Déroulé

1. L'application crée un jeton (`new_token` : 32 octets de `getrandom`, 64 chiffres hexadécimaux), lance l'écouteur (`spawn_hello_listener(token, summary)`) et publie son rendez-vous (`publish(&handle.rendezvous(canonical_project(path)))`). Elle tient `summary: Arc<Mutex<Option<SummaryView>>>` à jour (`None` : aucun document).
2. `planrs mcp`, avant d'ouvrir un fichier, appelle `find_live_for(path)`. Un rendez-vous vivant qui tient ce fichier ⇒ erreur `file.open_in_app` (code de `error::codes`, levé par le lot MCP/CLI, pas par ce module).
3. À la fermeture, `RendezvousGuard` supprime le fichier et `ListenerHandle` arrête le fil et supprime la socket (`Drop`). Après un plantage, le prochain `find_live_for` nettoie.

## 2. Dossier d'état et rendez-vous

| Plateforme | Dossier (`state_dir()`) |
|---|---|
| Linux et autres Unix | `$XDG_STATE_HOME/planrs/control`, à défaut `~/.local/state/planrs/control` |
| macOS | `~/Library/Application Support/planrs/control` |
| Windows | `%LOCALAPPDATA%\planrs\control` |

- `PLANRS_CONTROL_DIR` (chemin absolu) remplace le dossier : tests d'intégration de bout en bout, installations portables. Une valeur relative est ignorée, comme pour les variables XDG.
- Les fonctions `publish_in`, `find_live_for_in`, `spawn_hello_listener_in` prennent le dossier en paramètre (tests, sans toucher à l'environnement).
- Dossier créé en 0700 (parents compris) ; s'il existait avec d'autres droits, ils sont resserrés à 0700.
- Un fichier `<pid>-<n>.json` par document, en 0600 (`OpenOptionsExt::mode`), écrit dans `<pid>-<n>.tmp` puis renommé : un lecteur ne voit jamais un rendez-vous à moitié écrit.
- Contenu (`Rendezvous`, `deny_unknown_fields`) : `{"v":1,"pid":1234,"endpoint":"/…/1234-0.sock","token":"<64 hex>","project":"/chemin/canonique.xml"}`. `project` est le chemin rendu par `std::fs::canonicalize` (`canonical_project`) ; un chemin non UTF-8 ne se publie pas. Le `Debug` de `Rendezvous` et de `ListenerHandle` masque le jeton.

### `find_live_for(path)`

- Canonicalise `path` (fichier absent ⇒ `None`), lit les `.json` du dossier dans l'ordre de leur nom (64 Kio au plus chacun).
- Fichier de version 1 illisible : supprimé. Autre version, ou sans `v` : ignoré et **conservé** (appartient à une autre version de l'application). `.tmp` : ignoré.
- Rendez-vous qui désigne `path` : sondé par `probe` (`hello` avec son jeton, délai 2 s). Réponse `ok` ⇒ rendu ; échec (socket absente, refus du jeton, délai) ⇒ fichier supprimé.
- Rendez-vous d'un autre fichier : sous Unix, supprimé si sa socket n'existe plus (nettoyage au passage, sans connexion) ; sinon laissé.
- **Windows (hypothèse J4, à remplacer en J7)** : pas d'écouteur (le named pipe demanderait un runtime côté interface) ; un rendez-vous est tenu pour vivant si son fichier a été écrit il y a moins de 24 h. Conséquence connue : une application plantée bloque son fichier jusqu'à 24 h, ou jusqu'à suppression manuelle du rendez-vous.

## 3. Protocole

JSON par lignes (`\n`, un `\r` final toléré), 8 Mio au plus par ligne (`MAX_LINE_BYTES`) : `read_line_bounded` refuse dès que la borne est dépassée, sans garder plus en mémoire. Requête `Frame` (`deny_unknown_fields`) :

```json
{"v":1,"id":1,"method":"hello","params":{"token":"<64 hex>"}}
```

`params` absent vaut `null`. Méthodes : `hello`, `query`, `execute`, `undo`, `redo`, `history`, `ui_state`, `screenshot`. Réponses `Reply` (enum externe, `snake_case`) :

```json
{"ok":{"id":1,"result":{"v":1,"app":"planrs","version":"0.1.0","pid":1234,"methods":["hello","query"]}}}
{"err":{"id":2,"error":{"code":"control.bad_token","message":"jeton refusé"}}}
{"pending_approval":{"id":3}}
```

`pending_approval` est réservé à J7 (jamais émis en J4).

### Écouteur J4

- Socket Unix `<pid>-<n>.sock` en 0600 dans le dossier d'état ; un fil `planrs-control` accepte, un fil par connexion, 8 connexions simultanées au plus (au-delà, fermées sans réponse). Connexion inactive fermée après 30 s ; écriture bornée à 5 s.
- Première requête : `hello` avec le jeton. Comparaison en temps constant (décodage hexadécimal puis boucle XOR sur 32 octets ; un jeton mal formé suit la même boucle). Majuscules acceptées.
- `query` : `params` nul, `{}` ou `{"command":"project.summary"}` ⇒ `SummaryView` du document (ou `null`). Toute autre lecture ⇒ `control.unsupported`.
- `execute`, `undo`, `redo`, `history`, `ui_state`, `screenshot` ⇒ `control.unsupported`, connexion maintenue.

| Code | Cas | Connexion |
|---|---|---|
| `control.bad_token` | `hello` sans jeton ou jeton faux | fermée |
| `control.unauthenticated` | requête avant un `hello` accepté | fermée |
| `control.line_too_long` | ligne de plus de 8 Mio (id 0) | fermée |
| `control.invalid_frame` | JSON invalide, champ ou méthode inconnus, `params` mal formés ; `id` repris s'il se lit | fermée avant `hello`, maintenue après |
| `control.unsupported_version` | `v` ≠ 1 | idem |
| `control.unsupported` | méthode ou lecture réservée à J7 | maintenue |
| `control.internal` | sérialisation impossible du résumé | maintenue |

Les codes sont dans `control::codes` (propres au fil, distincts de `error::codes` qui couvre les commandes).

## 4. Sécurité

- Seul l'utilisateur lit le dossier (0700) et le jeton (0600) ; la socket est elle aussi en 0600. Le jeton protège en plus contre une socket réutilisée par un autre processus au même chemin.
- Le jeton n'est jamais journalisé (`Debug` masqué).
- Pas de TCP en J4 (ADR-0002 : seulement sur demande explicite, lié à 127.0.0.1).

## 5. Tests

`crates/planrs-automation/tests/control.rs` : `control_frame_roundtrip`, `control_line_too_long_refused` (lecteur et socket), `control_rendezvous_mode_0600` (Unix), `control_stale_rendezvous_removed`, `control_hello_wrong_token_refused`, `control_find_live_matches_canonical_path` (détour `sub/..` et lien symbolique), `control_unsupported_methods_in_j4` ; hors Unix, `control_find_live_by_age_without_listener` (compilé pour Windows, non exécuté en J4). Tests unitaires du jeton dans `control/mod.rs`.

## 6. Limites et suites

- Relais des commandes, approbation, `ui_state`, `screenshot` : J7.
- Windows : écouteur sur named pipe et sonde réelle en J7 ; d'ici là, règle des 24 h ci-dessus.
- Le rendez-vous ne dit qu'un projet par fichier ; une application à plusieurs documents publie un rendez-vous par document (même écouteur, `handle.rendezvous(...)` appelé plusieurs fois).
