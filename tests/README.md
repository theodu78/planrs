# Tests transverses

Paquet interne `planrs-tests` (non publié). Les tests unitaires restent dans chaque crate ; ce dossier regroupe ce qui traverse les crates :

| Dossier | Cible | Rôle |
|---|---|---|
| `rules/` | `cargo test -p planrs-tests --test rules` | un test par règle de calcul `R-xxx` |
| `fixtures/` | (données) | plannings MSPDI de test et résultats attendus figés |
| `oracle/` | `cargo test -p planrs-tests --test oracle` | comparaison avec MS Project (coquille, ADR-0004) |
| `corpus/` | `cargo xtask corpus fetch` puis `cargo test -p planrs-tests --test corpus` | fichiers MSPDI publics recalculés et comparés aux valeurs de MS Project (manifeste, planchers, rapport) |

Les interdits de la règle 10 (`unwrap`, `expect`, `panic!`, indexation…) sont levés dans les tests par un `#![allow(...)]` en tête de fichier, commenté : un test qui échoue doit paniquer.
