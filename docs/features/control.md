# Canal de contrôle de l'application (`planrs_automation::control`)

État au 2026-10-21 (lot L10 de J4, ADR-0002, ADR-0012 D9 ; relais J7 aux lots L2 et L3, ADR-0015 D9 à D11). Le canal relie `planrs mcp` à une application desktop ouverte : `planrs mcp` s'y **rattache** et lui relaie ses appels d'outils ; l'application les exécute sur la session de son document, **toute écriture après validation par l'utilisateur** (bandeau). En `--headless`, le canal garde le fichier (pas de double écrivain, `file.open_in_app`). Module absent en wasm (`#[cfg(not(target_arch = "wasm32"))]`) : la version web n'a pas de canal.

## 1. Déroulé

1. L'application crée un jeton (`new_token` : 32 octets de `getrandom`, 64 chiffres hexadécimaux), une file bornée (`sync_channel(RELAY_QUEUE)`, 64), lance l'écouteur du relais (`spawn_listener_in(dir, token, inbox, wake)`, `wake` = `ctx.request_repaint`) et publie son rendez-vous (`publish(&handle.rendezvous(canonical_project(path)))`). Le récepteur de la file est remis à `App::control_inbox`.
2. `planrs mcp`, avant d'ouvrir un fichier, appelle `find_live_for(path)`. Un rendez-vous vivant qui tient ce fichier ⇒ rattachement (`RelayClient`, lot L2) ; en `--headless`, erreur `file.open_in_app`.
3. À la fermeture, `RendezvousGuard` supprime le fichier et `ListenerHandle` arrête le fil et supprime la socket (`Drop`). Après un plantage, le prochain `find_live_for` nettoie.
4. **Document neuf** (autre numéro d'ouverture : autre fichier, même fichier rouvert, « Nouveau ») : l'application change le jeton de l'écouteur (`ListenerHandle::rotate_token`) puis republie son rendez-vous. Un agent rattaché au document précédent voit son `hello` refusé : `control.document_changed`, rien n'est fait, le serveur MCP se détache (`open_project` du fichier voulu rattache de nouveau). « Enregistrer sous » (même ouverture) garde le jeton. Relecture finale J7, relais F2 : un seul écouteur et un seul jeton servaient tous les documents, si bien que les appels d'un agent rattaché à A (UID compris) s'exécutaient en silence sur B après l'ouverture de B dans l'application (tests `relay_rotated_token_refuses_previous_document`, `relay_document_changed_detaches`, `desktop_new_document_rotates_token`).

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
- Rendez-vous qui désigne `path` : sondé par `probe` (`hello` avec son jeton, délai 2 s). Réponse `ok` ⇒ rendu. Fichier supprimé seulement sur une preuve de mort : refus du jeton (rendez-vous périmé), socket absente ou sans écouteur (`ECONNREFUSED`). Tout autre échec (délai, `control.busy` parce que l'application sert déjà ses 8 connexions, connexion fermée sans réponse, réponse illisible) ⇒ application tenue pour vivante, rendez-vous gardé et rendu : la garde ADR-0002 n'est jamais levée par une application occupée (test `control_busy_app_kept_live`).
- Rendez-vous d'un autre fichier : sous Unix, supprimé si sa socket n'existe plus (nettoyage au passage, sans connexion) ; sinon laissé.
- **Windows (hypothèse J4, à remplacer en J7)** : pas d'écouteur (le named pipe demanderait un runtime côté interface) ; un rendez-vous est tenu pour vivant si son fichier a été écrit il y a moins de 24 h. Conséquence connue : une application plantée bloque son fichier jusqu'à 24 h, ou jusqu'à suppression manuelle du rendez-vous.

## 3. Protocole

JSON par lignes (`\n`, un `\r` final toléré), 8 Mio au plus par ligne (`MAX_LINE_BYTES`) : `read_line_bounded` refuse dès que la borne est dépassée, sans garder plus en mémoire. Requête `Frame` (`deny_unknown_fields`) :

```json
{"v":1,"id":1,"method":"hello","params":{"token":"<64 hex>"}}
```

`params` absent vaut `null`. Méthodes : `hello`, `query`, `execute`, `ui_state`, `screenshot` (J7 : `undo`, `redo`, `history` retirées, remplacées par `execute { tool: "history.undo" }`). Réponses `Reply` (enum externe, `snake_case`) :

```json
{"ok":{"id":1,"result":{"v":1,"app":"planrs","version":"0.1.0","pid":1234,"methods":["hello","query"]}}}
{"err":{"id":2,"error":{"code":"control.bad_token","message":"jeton refusé"}}}
{"pending_approval":{"id":3}}
```

`pending_approval` (J7) annonce une écriture en attente de validation : la réponse finale (`ok` ou `err`) suit sur la même connexion, avec le même `id`. Une erreur d'outil passe avec tous ses champs (`code`, `message`, `op_index`, `field`, `subject`, `diagnostics`, aplatis dans `WireError::details`) : même objet que l'erreur structurée du serveur MCP local.

### Relais (J7, lot L3)

| Méthode | `params` | Appel poussé dans la file |
|---|---|---|
| `query` | `null`, ou `{ command?, params? }` (défaut `project.summary`, `{}`) ; `command` du genre `Read` ou `Analysis` seulement, sinon `control.unsupported` | `Execute { tool: command, params }` |
| `execute` | `{ tool, params?, approval_timeout_ms? }` (`params` absent : `{}` ; `approval_timeout_ms` : délai de validation voulu par le client, entier > 0) | `Execute { tool, params }` |
| `ui_state` | `null` ou `{}` | `UiState` |
| `screenshot` | `{ scale_down? }` | `Screenshot { scale_down }` |

Écouteur (`spawn_listener_in`, réglages `ListenerOptions` par `spawn_listener_with`) :
- l'écouteur ne touche jamais la session : un fil par connexion pousse un `RelayCall { id, kind, state: Arc<CallState>, reply }` par `try_send`, réveille l'interface (`Waker`), puis attend les réponses sur `reply` ; une requête à la fois par connexion ;
- file pleine ⇒ `control.busy`, connexion maintenue ; application partie (file fermée) ⇒ `control.app_gone` ; plus de 8 connexions ⇒ `control.busy` (id 0) puis fermeture : le client ne se détache pas (relecture finale J7, relais F4 ; une fermeture muette se lisait « application partie ») ; `hello` attendu 5 s au plus, puis 30 s d'inactivité ;
- délai `approval_timeout` (120 s par défaut, `APPROVAL_TIMEOUT`) depuis la mise en file, ou `approval_timeout_ms` du client s'il est plus court (relecture finale J7, relais F1 : avec `--approval-timeout 30`, l'agent apprenait « rien n'a été appliqué » à 40 s alors que le bandeau restait acceptable jusqu'à 120 s, d'où un doublon à la relance) : `CallState::expire()` ; s'il réussit, `Cancel(id)` est poussé dans la file avec l'état de l'appel abandonné (l'application retire **ce** bandeau, pas celui d'un autre agent au même id) et l'agent reçoit `control.approval_timeout` (après `pending_approval`) ou `control.app_gone` (aucune réponse) ; s'il échoue, l'application vient de prendre l'appel et sa réponse est attendue `reply_grace` (10 s) au plus. La course Accepter/délai est donc tranchée par `CallState` (`Pending` → `Taken` ou `Expired`, une seule fois) : rien n'est appliqué après l'expiration. Pendant l'attente, la connexion est surveillée (lecture non bloquante tous les 100 ms) : client parti (délai du client écoulé, processus mort) ⇒ appel expiré et `Cancel`, plus rien n'est acceptable.

Application (`planrs-ui-egui/src/relay.rs`, file vidée en **début d'image**, avant le dessin) :
- appel déjà expiré dans la file : ignoré ; `Cancel(id)` : écriture `id` **de même état** retirée (deux `planrs mcp` numérotent leurs appels de la même façon, relecture finale J7, relais F3) ; Accepter et Refuser ne visent que le bandeau affiché (tête de file) ;
- `app.ui_state` (ou méthode `ui_state`) ⇒ `{ ui_state, selection (UID triés), active, view, visible_rows: [première, dernière], dirty, generation, opening }` ; `ui_state` est l'`UiState` **sans** `recent` (les chemins des fichiers récents sont d'autres projets de l'utilisateur, jamais donnés à l'agent : relecture finale J7, relais F7) ;
- `app.ui` ⇒ actions d'interface (`select`, `scroll_to`, `zoom`, `view`, `theme`, `language`), appliquées en fin d'image, sans historique ; au moins un champ ; UID absent de la vue ⇒ `task.not_found` ; réponse `{ applied }` ;
- `app.screenshot` (ou méthode `screenshot`) ⇒ capture de la fenêtre (desktop, J7 lot L14, C-CTL-08) : `{ mime_type, data (PNG en base64), width, height, scale_down, bytes }`, réduite de moitié `scale_down` fois (0 à 2) puis encore, deux fois au plus, tant que le base64 dépasse 6 Mio (sinon `control.screenshot_too_large`) ; aucune image en 5 s, ou capture non activée (kittest, web) ⇒ `control.screenshot_unavailable` (`docs/features/ui.md` § 1.21) ;
- id inconnu ⇒ `param.invalid` ; `project.open`, `project.new`, `project.import_csv` ⇒ `control.not_relayable` (C-CTL-05) ; aucun document ⇒ `session.none` ;
- lectures (genres `Read`, `Analysis`, `history.list`, et toute écriture en `dry_run: true`) ⇒ `workspace::call_with` sur la session du document, réponse immédiate : même JSON que `Workspace::call` local (C-CTL-09) ;
- écritures (genre `Write`, `batch.apply`, `project.save`, `project.export_mspdi`, `history.undo`, `history.redo`) ⇒ validation (voir § 3.1).

#### 3.1 Validation d'une écriture (H-UI-27, C-CTL-02 à C-CTL-04, C-CTL-07)

1. `project.save`/`project.export_mspdi` avec un `path` tenu par une autre application (rendez-vous vivant d'un autre pid) ⇒ `file.open_in_app` (garde ADR-0002 côté application : le serveur rattaché ne la vérifie plus).
2. Lot ou écriture unitaire : `call_with` avec `dry_run: true` ; refus ⇒ erreur rendue aussitôt, aucun bandeau. Sinon `pending_approval`, et `PendingApproval { id, tool, params, preview: ApprovalSummary, generation, opening, now }` entre dans la file des bandeaux. `ApprovalSummary { changed_tasks, finish_delta (minutes), finish_delta_text (texte du moteur), critical_changed, warnings }` est lu dans le diff du `dry_run` (R-163) : l'interface ne calcule rien. Enregistrer, exporter, annuler, rétablir : sans aperçu (outil et chemin seulement).
3. Bandeau : « L'agent propose sur « maison.xml » : task.update — 3 tâches modifiées, fin +2j, chemin critique modifié », Accepter / Refuser, « N autres propositions en attente » ; un seul à la fois. Le bandeau ne prend pas le focus clavier (relecture finale J7 : un Entrée ou un Espace tapé au moment où il paraît validait l'écriture sans lecture) : Ctrl/Cmd+Entrée accepte (hors champ de texte), Échap refuse quand un bouton du bandeau a le focus ; sinon, la souris. Le focus revient à la table quand la file est vide.
4. Accepter : appel expiré ⇒ rien ; document fermé ou remplacé (numéro d'ouverture) ⇒ `control.rejected` ; génération changée depuis l'aperçu ⇒ nouvel aperçu, rien d'appliqué, le bandeau reste (C-CTL-07) ; sinon `CallState::claim()` puis `call_with` avec `source: Agent` et le « maintenant » de la demande (règle 5) : une entrée d'historique d'origine `Agent` (« Annuler : agent — … » au menu Édition), puis `ok`.
5. Refuser ⇒ `control.rejected` ; projet, historique, génération et drapeau modifié intacts (C-CTL-03).

### Écouteur J4 (`spawn_hello_listener_in`, gardé pour les tests des surfaces)

- Socket Unix `<pid>-<n>.sock` en 0600 dans le dossier d'état ; un fil `planrs-control` accepte, un fil par connexion, 8 connexions simultanées au plus (au-delà, `control.busy` puis fermeture). `hello` attendu 5 s au plus, puis connexion inactive fermée après 30 s ; écriture bornée à 5 s.
- Première requête : `hello` avec le jeton. Comparaison en temps constant (décodage hexadécimal puis boucle XOR sur 32 octets ; un jeton mal formé suit la même boucle). Majuscules acceptées.
- `query` : `params` nul, `{}` ou `{"command":"project.summary"}` ⇒ `SummaryView` du document (ou `null`). Toute autre lecture ⇒ `control.unsupported`.
- `execute`, `ui_state`, `screenshot` ⇒ `control.unsupported`, connexion maintenue.

| Code | Cas | Connexion |
|---|---|---|
| `control.bad_token` | `hello` sans jeton ou jeton faux | fermée |
| `control.unauthenticated` | requête avant un `hello` accepté | fermée |
| `control.line_too_long` | ligne de plus de 8 Mio (id 0) | fermée |
| `control.invalid_frame` | JSON invalide, champ ou méthode inconnus, `params` mal formés ; `id` repris s'il se lit | fermée avant `hello`, maintenue après |
| `control.unsupported_version` | `v` ≠ 1 | idem |
| `control.unsupported` | méthode non servie par l'écouteur J4, `query` d'une commande qui n'est pas une lecture (relais) | maintenue |
| `control.busy` | file de l'application pleine (relais) ; plus de 8 connexions (id 0) | maintenue ; fermée (connexions) |
| `control.internal` | sérialisation impossible du résumé | maintenue |

Les codes sont dans `control::codes` (propres au fil, distincts de `error::codes` qui couvre les commandes). Codes J7 des commandes relayées (`error::codes`) : `control.rejected`, `control.approval_timeout`, `control.not_relayable`, `control.not_attached`, `control.app_gone`, `control.screenshot_unavailable`, `control.screenshot_too_large` ; relecture finale J7 : `control.busy` (application occupée, rattachement gardé), `control.document_changed` (jeton du document précédent refusé, détachement).

## 4. Sécurité

- Seul l'utilisateur lit le dossier (0700) et le jeton (0600) ; la socket est elle aussi en 0600. Le jeton protège en plus contre une socket réutilisée par un autre processus au même chemin.
- Le jeton n'est jamais journalisé (`Debug` masqué).
- Pas de TCP en J4 (ADR-0002 : seulement sur demande explicite, lié à 127.0.0.1).

## 5. Tests

`crates/planrs-automation/tests/control.rs` : `control_frame_roundtrip`, `control_line_too_long_refused` (lecteur et socket), `control_rendezvous_mode_0600` (Unix), `control_stale_rendezvous_removed`, `control_hello_wrong_token_refused`, `control_find_live_matches_canonical_path` (détour `sub/..` et lien symbolique), `control_unsupported_methods_in_j4` ; relais J7 : `c_ctl_01_bad_token_closes`, `relay_queue_full_busy`, `c_ctl_04_listener_expires_and_cancels`, `relay_methods_become_calls`, `relay_call_with_marks_agent_source` ; relecture finale J7 : `relay_asked_timeout_shorter_wins`, `relay_client_hangup_expires_call`, `relay_connections_full_busy`, `relay_rotated_token_refuses_previous_document` ; hors Unix, `control_find_live_by_age_without_listener` (compilé pour Windows). Tests unitaires du jeton dans `control/mod.rs`.

`crates/planrs-ui-egui/tests/ui/e2e.rs` (J7 lot L16, Unix) : `relay_app_real_mcp`, application kittest avec écouteur réel et rendez-vous publié dans un dossier temporaire (`PLANRS_CONTROL_DIR`), **vrai binaire** `planrs mcp` rattaché au démarrage (un seul rendez-vous vivant), scénario `tests/mcp/scenarios/relay_j7.jsonl` (11 appels : écriture acceptée, refusée, édition de l'utilisateur pendant une attente puis double Accepter (C-CTL-07), `ui_state`, `ui_action`, `screenshot` ⇒ `control.screenshot_unavailable`, `open_project` du fichier tenu ⇒ résumé rattaché, `undo` relayé, application fermée ⇒ `control.app_gone` puis `session.none` (C-CTL-06)) ; résultats et journal du serveur égaux au scénario ; P47 contre un rejeu local. Binaire : `PLANRS_MCP_BIN`, sinon à côté de `deps/` ; `PLANRS_RELAY=required` en CI (job `ui`).

`crates/planrs-ui-egui/tests/ui/relay.rs` (application réelle, file en mémoire, et socket réelle sous Unix) : `c_ctl_02_pending_then_ok` (et `…_over_socket`), `c_ctl_03_reject_leaves_state`, `c_ctl_04_timeout_then_accept_does_nothing` (et `c_ctl_04_timeout_over_socket`, `c_ctl_04_client_timeout_shorter_than_listener`), `relay_cancel_same_id_other_agent_kept`, `c_ctl_05_open_not_relayable`, `c_ctl_07_stale_generation_new_preview`, `c_ctl_09_read_same_as_local`, `h_ui_27_banner_keyboard`, `h_ui_27_banner_enter_or_space_never_accepts`, `relay_agent_entry_single_undo`, `relay_ui_state_and_ui_actions`, `relay_expired_in_queue_is_skipped`, `p47_relay_replays_like_local` (30 appels). Fluidité : `approval_banner` dans `tests/perf.rs` (release, ignoré par défaut).

## 6. Limites et suites

- Capture (`app.screenshot`) : faite en lot L14 (§ 1.21 de `ui.md`) ; non vérifiée sur une fenêtre réelle par un test automatique (la CI n'a pas d'écran ; sous kittest, `control.screenshot_unavailable`, vérifié par `relay_app_real_mcp`). Panneau IA complet (diff détaillé, aperçu `what_if`) : J8 ; le bandeau J7 est minimal.
- `open_project` relayé : le fichier tenu garde le rattachement (résumé relayé, `attached: true`) ; un autre fichier détache ; `control.not_relayable` ne s'observe que par `execute project.open` sur le fil (lot L16, ADR-0015). Le résumé relayé est demandé avec des paramètres `{}` (et non `null`, refusé par `NoParams` côté application : défaut du lot L2 trouvé et corrigé par `relay_app_real_mcp`).
- Essai réel n° 4 (Claude rattaché à l'application ouverte) : hors lot, session principale.
- La garde d'un enregistrement relayé sonde les rendez-vous (`find_live_for_in`, 2 s au plus par application) sur le fil de l'interface, avant l'affichage du bandeau.
- Windows : écouteur sur named pipe et sonde réelle en J7 ; d'ici là, règle des 24 h ci-dessus.
- Le rendez-vous ne dit qu'un projet par fichier ; l'application J7 n'a qu'un document à la fois (un jeton par document, changé à chaque ouverture). Une application à plusieurs documents demanderait un écouteur (ou un jeton) par document.
