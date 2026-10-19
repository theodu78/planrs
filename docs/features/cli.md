# Ligne de commande `planrs` (`apps/planrs-cli`)

État au 2026-10-19 (lot L9 de J4, ADR-0012 D1, brief § 7). Le binaire expose les opérations du MCP pour les scripts et les harnais d'agents. Chaque sous-commande ouvre un `Workspace` et appelle `Workspace::call` avec les ids du registre : aucune logique de planification ici (règle 3), `--json` rend exactement les vues JSON du MCP (`docs/features/workspace.md`).

## 1. Sous-commandes

| Commande | Ids du registre | Sortie |
|---|---|---|
| `planrs summary <plan> [--json]` | `project.open`, `project.summary` | `SummaryView` ; texte : nom, début, fin, durée, comptes |
| `planrs tasks <plan> [--critical] [--late] [--from J] [--to J] [--limit N] [--json]` | `task.list` | `ListView` ; texte : tableau ID, nom (indenté), durée, début, fin, marge totale, `*` si critique |
| `planrs apply <plan> <ops.json> [--dry-run] [-o out.xml] [--json]` | `batch.apply`, `project.save` | `OutcomeView` ; texte : appliqué ou simulé, fin avant → après, tâches décalées, fichier écrit |
| `planrs render <plan> -o x.svg [--from J] [--to J] [--critical] [--zoom auto\|day\|week\|month\|quarter]` | `render.gantt` | SVG écrit ; texte : lignes affichées, masquées, octets |
| `planrs import-csv <in.csv> -o out.xml` | `project.import_csv`, `project.save` | MSPDI écrit ; texte : tâches, liens, diagnostics (détail sur la sortie d'erreur) |
| `planrs mcp [--journal F] [--open plan.xml] [--headless] [--http ADDR --root DOSSIER [--allow-remote] [--token-file F]]` | `planrs_mcp::serve_stdio`, `planrs_mcp::serve_http` | serveur MCP stdio (lot L8) ou HTTP « streamable HTTP » sur `http://ADDR/mcp` (lot L17) |
| `planrs explain <plan> --task N` (cachée) | — | « disponible en J6 », code 2, fichier non lu |

- Dates d'option : jours `AAAA-MM-JJ` (`--from`, `--to`) ; une valeur mal formée est une erreur d'usage (2).
- `tasks` sans `--limit` lit toutes les pages (500 par page) et rend une seule `ListView` sans `next_cursor` ; avec `--limit N` (1 à 500), une seule page telle que le MCP la rend (`next_cursor` s'il en reste) ; `N` hors plage est une erreur d'usage (code 2) contrôlée par clap avant la lecture du plan (test `cli_explain_hidden_exit_2`).
- `apply` : le fichier de lot a le format versionné unique (`{"format_version":1,"lang":"fr","ops":[{"op":"task.add",…}]}`, ADR-0012 D6), lu au plus 64 Mio. `--dry-run` n'écrit rien, même avec `-o` (un `"dry_run": true` dans le fichier aussi). `-o` écrit le résultat ailleurs et laisse `<plan>` intact ; sans `-o`, `<plan>` est remplacé de façon atomique (`files::write_atomic` : temporaire du même dossier, `fsync`, renommage). Un lot refusé ne touche aucun fichier.
- `render` : 200 lignes au plus (borne de `render.gantt`), thème clair, libellés dans la langue de la CLI ; une troncature (plage, taille) est signalée sur la sortie d'erreur.
- `import-csv` : séparateur et langue des en-têtes détectés (R-144 à R-146), projet planifié depuis « maintenant » (hypothèse de `project.import_csv`).
- `mcp` : langue par défaut anglaise (ADR-0012, arbitrage 6) sauf `--lang` explicite ; `--headless` lève la garde ADR-0002 (`file.open_in_app`).
- `mcp --http ADDR` (lot L17) : `:8080` = `127.0.0.1:8080`, aussi `localhost:P`, `IP:P`, `[IPv6]:P` (pas d'autre nom d'hôte) ; HTTP/1.1, chemin `/mcp` (404 ailleurs) ; l'adresse liée est écrite sur la sortie d'erreur (port `0` : port libre). `--root` obligatoire : tout chemin de fichier y est confiné (`file.outside_root`, `--open` compris) ; toutes les sessions HTTP partagent le même espace de travail. `Host` limité à la boucle locale, `Origin` à `http://127.0.0.1:*` et `http://localhost:*` (403 sinon), corps de requête ≤ 8 Mio. Adresse non locale : `--allow-remote` et `--token-file` exigés (jeton d'au moins 16 caractères sans espace, `Authorization: Bearer …` comparé en temps constant, 401 sinon ; un jeton donné en boucle locale est exigé aussi), `Host` élargi à l'IP donnée ; `0.0.0.0`/`::` refusés. Le serveur tourne jusqu'à l'arrêt du processus.

## 2. Options globales : « maintenant » et langue (`now.rs`)

- `--now 2026-10-19T08:00` > variable `PLANRS_NOW` > horloge locale (tronquée à la minute). C'est la seule lecture d'horloge du binaire ; la valeur est injectée dans le `Workspace` (règle 5). Une valeur invalide de l'option ou de la variable est une erreur d'usage (2) ; une variable vide est ignorée ; une variable invalide est ignorée quand l'option est donnée.
- `--lang fr|en` > `LANG` (`en…` → anglais, `fr…` → français) > français. La langue fixe les messages, les textes de durée des vues et les libellés du rendu.

## 3. Codes de sortie (C-MCP-08, `exit.rs`)

| Code | Sens | Cas |
|---|---|---|
| 0 | succès | y compris `--help`, `--version`, `apply --dry-run` |
| 1 | commande refusée | erreur métier (`task.not_found`, `batch.would_create_errors`, `param.invalid`, lot JSON mal formé…) |
| 2 | usage | arguments (clap), date d'option invalide, `explain`, `mcp --http` sans `--root`, adresse invalide ou non locale sans `--allow-remote`/`--token-file`, jeton trop court, serveur MCP indisponible |
| 3 | fichier | `file.*` (lecture, écriture, taille, MSPDI inexploitable, fichier tenu par l'application) et `csv.error.*` (CSV illisible) ; serveur MCP arrêté sur une erreur d'entrée/sortie ou une ligne trop longue (C-MCP-06) ; `--root` ou `--token-file` illisible, adresse HTTP déjà prise |

Erreur : `planrs : <code> : <message>` sur la sortie d'erreur (diagnostics R-160 en dessous) ; avec `--json`, l'erreur structurée du MCP `{"error": {code, message, op_index, field, subject, diagnostics}}` sur la sortie standard. Aucune écriture ne peut planter (sortie standard fermée, `planrs tasks plan.xml | head` : code 3 sans message).

## 4. Tests (`apps/planrs-cli/tests/cli.rs`)

Binaire lancé par `env!("CARGO_BIN_EXE_planrs")`, `PLANRS_NOW` retiré et `LANG` fixé, fichiers dans un dossier temporaire propre au test.

- `cli_summary_house_json_equals_mcp_view` : `summary --json` et `tasks --json` (toutes pages, `--critical`, `--limit`) égaux aux vues de `Workspace::call` sur `house.xml`.
- `cli_apply_dry_run_leaves_file`, `cli_apply_writes_atomic` (`-o`, remplacement sans temporaire résiduel), `cli_apply_refused_exit_1` (erreur structurée, fichier intact).
- `cli_missing_file_exit_3` (absent, MSPDI illisible, lot absent, écriture impossible), `cli_render_svg_bytes_stable` (deux rendus identiques, égaux à `render.gantt`), `cli_now_env_and_flag_precedence`, `cli_explain_hidden_exit_2`, `cli_import_csv_roundtrip`, `cli_lang_flag_and_env`.
- Tests unitaires : priorité de « maintenant » et lecture de `LANG` (`now.rs`), familles de codes (`exit.rs`).
