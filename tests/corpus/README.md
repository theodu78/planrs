# Corpus public

Fichiers MSPDI réels (modèles, supports de formation, exemples publics) : chacun contient les dates que MS Project a calculées. Le harnais (J2, ADR-0004) efface les valeurs calculées, laisse le moteur recalculer et compare ; les tâches en planification manuelle sont exclues de la comparaison.

## Règles

- Les fichiers ne sont **pas** versionnés (droits d'auteur) : ils vont dans `tests/corpus/files/`, ignoré par git.
- Seul [`manifest.toml`](manifest.toml) est versionné. Une entrée `[[file]]` par fichier :

| Clé | Obligatoire | Contenu |
|---|---|---|
| `name` | oui | nom du fichier dans `files/` |
| `source` | oui | provenance lisible |
| `url` | oui | adresse de téléchargement |
| `date` | oui | date de récupération, `AAAA-MM-JJ` |
| `sha256` | oui | empreinte SHA-256, 64 hexadécimaux minuscules |
| `note` | non | remarque (conversion, particularités) |

- Le manifeste est **figé** : on ajoute des lignes, on n'en retire pas sans ADR. Le plancher `planrs_core::parity::FLOOR` compte des cas sur cette liste.
- Un `.mpp` peut être converti en MSPDI avec MPXJ **comme outil externe uniquement** ; son code et sa documentation ne sont jamais lus (règle 1). Le noter dans `note`.

`cargo test -p planrs-tests --test corpus` vérifie aujourd'hui le format du manifeste.
