# Règles de calcul (`R-xxx`)

Chaque règle reproduisant un comportement de MS Project est consignée ici avant ou avec son implémentation. Format :

```
## R-001 — Titre court
- **Formulation** : énoncé précis, avec un exemple chiffré.
- **Source** : lien vers la page Microsoft (support.microsoft.com / learn.microsoft.com, schéma MSPDI) ou « hypothèse ».
- **Statut** : `hypothèse` | `vérifié contre MS Project` (avec la référence du cas de corpus ou d'oracle).
- **Tests** : chemins des tests portant `r001` dans leur nom.
```

Les identifiants ne sont jamais réattribués. Une règle abandonnée garde son numéro avec le statut `retirée` et la raison.

## Règles

Sources citées (pages Microsoft uniquement ; « Q » = Microsoft Q&A, communautaire, jamais suffisant pour le statut « vérifié ») : la table des références S1 à S15 et Q1 à Q3 figure dans l'ADR-0008 (`docs/decisions.md`) et sa spécification. Repère : 1984-01-01 est un dimanche ; `Moment(2026-10-19T08:00)` = 22 510 560.

### Point fixe et temps

## R-001 — Arrondi unique
- **Formulation** : toute division vers un entier (minutes, 1/10 000, centièmes) arrondit au plus proche, la moitié loin de zéro. Le calcul se fait en `i128` et renvoie une erreur en cas de débordement ou de dénominateur nul ou négatif. Exemples : 0,01 j = 4,8 min → 5 ; 0,33 j = 158,4 → 158 ; 0,3125 h = 18,75 → 19 ; 0,5 min → 1 ; −0,5 min → −1 ; −2,5 min → −3 ; 1,5 j = 720 exact.
- **Source** : hypothèse (ADR-0005).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r001_rounding_half_away_from_zero.rs` (`r001_*`) ; propriété P12 dans `crates/planrs-core/tests/fixed_properties.rs`.

## R-002 — Unités, pourcentages et coûts en point fixe
- **Formulation** : unités d'affectation en 1/10 000, de 0 à 600 000 000 000 (60 000 000 unités) ; avancement de 0 à 10 000 ; décalage en % dans ±1 000 000 (±10 000 %) ; coût en centièmes, |v| ≤ 99 999 999 999 999. Conversion depuis un décimal par R-001 ; hors bornes : erreur. Exemples : `Units::from_fraction(1,5)` = 15 000 ; `Units::from_percent(150)` = 15 000 ; `Percent::from_percent(33,33)` = 3 333, `(33,335)` = 3 334, `(101)` → erreur ; `Money::from_decimal(12,345)` = 1 235, `(−0,005)` = −1 ; `Units::from_fraction(60 000 001)` → erreur.
- **Source** : support.microsoft.com, « Specifications for Microsoft Project » (S11 : unités ≤ 60 000 000, coût ≤ 999 999 999 999,99), ADR-0005 ; borne du décalage en % : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r002_fixed_point_quantities.rs` (`r002_*`).

## R-003 — Domaine des dates et résolution à la minute
- **Formulation** : un instant est un nombre entier de minutes depuis 1984-01-01 00:00, dans [1984-01-01 00:00 ; 2150-01-01 00:00]. Cette dernière valeur est la fin du 31/12/2149 (24:00). Hors domaine : `TimeError::OutOfDomain`. Exemples : 2026-10-19T08:00 = 22 510 560 ; 1983-12-31T23:59 refusé ; 2150-01-01T00:00 accepté (= `Moment::MAX` = 87 308 640) ; 2150-01-01T00:01 refusé ; le jour 0 est un dimanche ; `Day::MAX` = 2149-12-31, un mercredi.
- **Source** : support.microsoft.com, « Specifications for Microsoft Project » (S11 : dates du 1er janvier 1984 au 31 décembre 2149).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r003_date_domain.rs` (`r003_*`) ; conversion exhaustive des 60 631 jours contre chrono dans `crates/planrs-core/src/time.rs`.

## R-004 — Secondes tronquées à l'import
- **Formulation** : `Moment::from_naive_floor` tronque les secondes (plancher) et signale la troncature ; `Moment::from_naive` (saisie stricte) refuse des secondes non nulles (`NonZeroSeconds`). Exemples : 2026-10-19T08:00:30 → 08:00, troncature signalée ; 08:00:59 → 08:00 ; 08:00:00 → 08:00 sans signal ; `from_naive(08:00:30)` → `NonZeroSeconds`.
- **Source** : hypothèse (MS Project affiche à la minute).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r004_seconds_truncated.rs` (`r004_*`).

## R-005 — Heure locale sans fuseau ni heure d'été
- **Formulation** : chaque jour dure 1 440 minutes ; aucun fuseau, aucun changement d'heure. Exemples : `Moment(2026-03-30 00:00) − Moment(2026-03-29 00:00)` = 1 440 (jour du passage à l'heure d'été en France) ; sur le calendrier 24 heures, `working_between(2026-03-29 00:00, 2026-03-30 00:00)` = 1 440.
- **Source** : brief § 3 ; hypothèse pour MS Project.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r005_local_time_no_dst.rs` (`r005_*`), partie temps civil ; le cas sur le calendrier 24 heures s'ajoute avec les conversions ouvrées.
