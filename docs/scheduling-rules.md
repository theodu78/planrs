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

### Durées et décalages

## R-006 — Facteurs des unités ouvrées
- **Formulation** : m = 1 ; h = 60 ; j = `MinutesPerDay` ; sem = `MinutesPerWeek` ; mois = `DaysPerMonth` × `MinutesPerDay`. Aucune cohérence imposée entre jour et semaine. Exemples, avec 480/2 400/20 : 5 j = 2 400 ; 2 sem = 4 800 ; 1 mois = 9 600 ; 3 h = 180 ; 1,5 h = 90. Avec `MinutesPerDay` = 420 : 1 j = 420 ; 1 sem = **2 400** (et non 5 × 420) ; 1 mois = 8 400. Avec 450 : 2 j = 900.
- **Source** : support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8 : 8 h, 40 h, 20 jours) ; learn.microsoft.com, schéma MSPDI, élément Project, `MinutesPerDay`, `MinutesPerWeek`, `DaysPerMonth` (S2 : champs entiers). Formule du mois : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r006_working_unit_factors.rs` (`r006_*`).

## R-007 — Unités écoulées
- **Formulation** : em = 1 ; eh = 60 ; ej = 1 440 ; esem = 10 080 ; emois = 43 200 (30 jours). Ces facteurs ne dépendent pas des options. Exemples : 2 ej = 2 880 ; 5 ej = 7 200 ; 1 emois = 43 200 ; avec `MinutesPerDay` = 420, 1 ej vaut toujours 1 440.
- **Source** : learn.microsoft.com, schéma MSPDI, « DurationFormat Element » (S3 : « Elapsed time counts all time, including non-working time ») ; emois = 30 jours : Microsoft Q&A, « Quick switch from mons to emons » (Q1, communautaire).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r007_elapsed_unit_factors.rs` (`r007_*`).

## R-008 — Signe des durées
- **Formulation** : durée et travail d'une tâche sont ≥ 0 (`Duration::new` et `Work::new` refusent un négatif) ; un décalage de lien est signé (avance = négatif). Bornes : |durée| et |décalage| ≤ 87 308 640 min ; travail ≤ 99 999 999 960 min. Exemples : `Duration::new(−480)` → `Negative` ; un `Lag` de −480 min est accepté ; `Work::new(99 999 999 961)` → `OutOfRange` ; `ConversionFactors::to_minutes(−1 j)` = −480 (admis pour un décalage).
- **Source** : support.microsoft.com, « Add lead or lag time to a task » (S15 : avance = décalage négatif) ; « Specifications for Microsoft Project » (S11 : travail maximal 1 666 666 666 h). Refus des durées négatives : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r008_duration_sign.rs` (`r008_*`).

## R-009 — Durée stockée en minutes ; les options n'agissent qu'aux conversions
- **Formulation** : une durée est stockée en minutes avec son unité d'affichage. Changer les facteurs ne modifie aucune minute stockée ; seule la valeur affichée est recalculée, à 2 décimales (R-001), en écriture canonique (sans zéro final). Exemples : « 84 j » à 480 = 40 320 min ; après passage à 540 min/jour, toujours 40 320 min, et `to_unit_value` = 74,67 j ; en jours à 480 : 158 min → 0,33 ; 720 → 1,5 ; en semaines : 2 400 → 1.
- **Source** : Microsoft Q&A, « Microsoft Project task duration days not displaying » (Q2, communautaire : changer les heures par jour change l'affichage, pas les heures stockées) ; hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r009_stored_minutes_options.rs` (`r009_*`).

## R-010 — Décalage en pourcentage (conversion)
- **Formulation** : minutes = `round_div(durée du prédécesseur × pct, 10 000)`, pct en 1/10 000. L'application aux liens arrive en J2. Exemples : 50 % de 1 440 = 720 ; 33 % de 480 = 158,4 → 158 ; −25 % de 480 = −120 ; 0,01 % de 480 = 0,048 → 0.
- **Source** : learn.microsoft.com, « DurationFormat Element » (S3 : formats % et e%) ; support.microsoft.com, « Predecessors (task field) » (S15 : « -50% ») ; hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r010_percent_lag_conversion.rs` (`r010_*`).

### Calendriers : modèle et calendriers fournis

## R-011 — Plages horaires d'un jour
- **Formulation** : plages [début, fin) à la minute, avec 0 ≤ début < fin ≤ 24:00 ; 1 à 5 plages par jour ouvré, triées, sans chevauchement ; deux plages contiguës équivalent à une seule (fusionnées à la compilation). Exemples : [08:00,12:00) + [13:00,17:00) = 480 ; [08:00,12:00) + [12:00,17:00) = 540, et `next_working_moment(12:00)` = 12:00 ; [12:00,08:00) → `EmptyRange` ; [08:00,12:00) + [11:00,13:00) → `OverlappingRanges` ; 6 plages → `TooManyRanges` ; [00:00,24:00) = 1 440.
- **Source** : learn.microsoft.com, schéma MSPDI, « XML Schema for the Calendars Element » (S1 : au plus cinq plages `WorkingTime`) ; 24:00 et contiguïté : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r011_time_ranges.rs` (`r011_*`), cas au niveau du jour ; le cas `next_working_moment` s'ajoute avec les calendriers compilés.

## R-012 — Calendrier Standard
- **Formulation** : lundi à vendredi 08:00-12:00 et 13:00-17:00, samedi et dimanche chômés. Exemples : total de la semaine 2 400 ; `working_between(lun 19/10 08:00, ven 23/10 17:00)` = 2 400 ; samedi → 0.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6) et « Create a new base calendar » (S7).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r012_standard_calendar.rs` (`r012_*`), cas au niveau du jour et de la semaine ; les cas de conversion s'ajoutent avec les calendriers compilés.

## R-013 — Calendrier 24 heures
- **Formulation** : tous les jours 00:00-24:00. Exemples : total de la semaine 10 080 ; `working_between(sam 24/10 10:00, dim 25/10 10:00)` = 1 440 ; `add_working(sam 24/10 10:00, 480)` = sam 18:00 (1 j vaut 480 et non 1 440, R-026).
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « All time … 12:00 A.M. to 12:00 A.M. »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r013_24_hours_calendar.rs` (`r013_*`), cas au niveau du jour et de la semaine ; les cas de conversion s'ajoutent avec les calendriers compilés.

## R-014 — Calendrier Équipe de nuit
- **Formulation** : dimanche chômé ; lundi [23:00,24:00) ; mardi à vendredi [00:00,03:00), [04:00,08:00), [23:00,24:00) ; samedi [00:00,03:00), [04:00,08:00). La plage de 23:00 appartient au jour qui commence. Exemples : total de la semaine 60 + 4 × 480 + 420 = 2 400 ; `add_working(lun 19/10 23:00, 480)` = mar 20/10 08:00 ; `add_working(sam 24/10 08:00, 60)` = mar 27/10 00:00.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « Monday night through Saturday morning, 11:00 P.M. to 8:00 A.M. … break between 3:00 A.M. and 4:00 A.M. »), « Create a new base calendar » (S7) ; découpage par jour : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r014_night_shift_calendar.rs` (`r014_*`), cas au niveau du jour et de la semaine ; les cas de conversion s'ajoutent avec les calendriers compilés.

### Projet

Les règles R-015 à R-031 (héritage, exceptions, récurrences, semaines datées, conversions, placement) sont réservées aux calendriers (ADR-0008) et ajoutées avec eux.

## R-032 — Valeurs par défaut d'un projet neuf
- **Formulation** : 480 min/jour, 2 400 min/semaine, 20 jours/mois ; début 08:00, fin 17:00 ; unité de durée : jour ; unité de travail : heure ; tâche à unités fixes, non pilotée par l'effort, estimée ; planification depuis le début ; respect des contraintes ; semaine commençant le lundi (affichage) ; calendriers Standard (UID 1, calendrier du projet), 24 heures (UID 2), Équipe de nuit (UID 3) ; date de création = date courante injectée. Nouvelle tâche : « 1j? » (une journée de `MinutesPerDay`, soit 480 min, dans l'unité de durée par défaut, estimée), contrainte « le plus tôt possible », priorité 500, mode automatique. Ces valeurs sont celles de l'application ; les valeurs du schéma MSPDI pour un élément absent à l'import (`DefaultTaskType` = durée fixe, `NewTasksEffortDriven` = vrai) relèvent de `planrs-mspdi` (J3).
- **Source** : support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8), « Change default times for entered start or finish dates » (S9), « Change the default task duration unit » (S10), « Project Information dialog box » (S6 : planification depuis le début), « Duration (task field) » (S14 : « 1d? ») ; learn.microsoft.com, schéma MSPDI, élément Project (S2). Type de tâche, pilotage par l'effort, UID des calendriers, premier jour de la semaine et durée d'une nouvelle tâche hors unité « jour » : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r032_new_project_defaults.rs` (`r032_*`).

## R-033 — Date saisie sans heure
- **Formulation** : un début reçoit l'heure de début par défaut, une fin l'heure de fin par défaut ; une heure par défaut de 24:00 donne le lendemain 00:00. L'alignement sur le calendrier se fait ensuite par R-030. Exemples : 2026-10-19 en début → 08:00 ; en fin → 17:00 ; avec un début par défaut à 07:00 → 07:00, puis `forward_span(07:00, 480)` démarre à 08:00 (exemple de S9).
- **Source** : support.microsoft.com, « Change default times for entered start or finish dates » (S9).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r033_date_without_time.rs` (`r033_*`) ; le cas `forward_span` s'ajoute avec le placement.

## R-034 — Date d'état effective
- **Formulation** : sans date d'état (NA), la date d'état est la date courante injectée. Exemples : date d'état NA et date courante 2026-10-17 08:00 → 2026-10-17 08:00 ; date d'état 2026-10-15 17:00 → 2026-10-15 17:00.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « If you set the status date to NA, Project uses the current date »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r034_effective_status_date.rs` (`r034_*`).
