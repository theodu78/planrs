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
- **Tests** : `tests/rules/r005_local_time_no_dst.rs` (`r005_*`), en temps civil et sur le calendrier 24 heures ; propriété P7.

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
- **Tests** : `tests/rules/r011_time_ranges.rs` (`r011_*`) ; fusion des plages contiguës vérifiée par P8 (plages compilées = `resolve_day` après fusion).

## R-012 — Calendrier Standard
- **Formulation** : lundi à vendredi 08:00-12:00 et 13:00-17:00, samedi et dimanche chômés. Exemples : total de la semaine 2 400 ; `working_between(lun 19/10 08:00, ven 23/10 17:00)` = 2 400 ; samedi → 0.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6) et « Create a new base calendar » (S7).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r012_standard_calendar.rs` (`r012_*`), cas au niveau du jour, de la semaine et des conversions.

## R-013 — Calendrier 24 heures
- **Formulation** : tous les jours 00:00-24:00. Exemples : total de la semaine 10 080 ; `working_between(sam 24/10 10:00, dim 25/10 10:00)` = 1 440 ; `add_working(sam 24/10 10:00, 480)` = sam 18:00 (1 j vaut 480 et non 1 440, R-026).
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « All time … 12:00 A.M. to 12:00 A.M. »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r013_24_hours_calendar.rs` (`r013_*`), cas au niveau du jour, de la semaine et des conversions.

## R-014 — Calendrier Équipe de nuit
- **Formulation** : dimanche chômé ; lundi [23:00,24:00) ; mardi à vendredi [00:00,03:00), [04:00,08:00), [23:00,24:00) ; samedi [00:00,03:00), [04:00,08:00). La plage de 23:00 appartient au jour qui commence. Exemples : total de la semaine 60 + 4 × 480 + 420 = 2 400 ; `add_working(lun 19/10 23:00, 480)` = mar 20/10 08:00 (60 + 180 + 240) ; `add_working(lun 19/10 00:00, 480)` = mar 08:00 ; `next_working_moment(mar 03:00)` = 04:00 et `add_working(mar 03:00, 60)` = 05:00 ; `add_working(ven 23/10 23:00, 480)` = sam 24/10 08:00, avec 481 → lun 26/10 23:01 ; `add_working(sam 24/10 08:00, 60)` = mar 27/10 00:00.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « Monday night through Saturday morning, 11:00 P.M. to 8:00 A.M. … break between 3:00 A.M. and 4:00 A.M. »), « Create a new base calendar » (S7) ; découpage par jour : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r014_night_shift_calendar.rs` (`r014_*`), cas au niveau du jour, de la semaine et des conversions.

### Calendriers : héritage, exceptions, récurrences, semaines datées

## R-015 — Héritage à un niveau
- **Formulation** : un calendrier de base n'a pas de parent ; un calendrier dérivé a une base, qui doit exister et être un calendrier de base. Un jour `Inherit` du dérivé prend la valeur de la base, et une modification de la base se répercute. Erreurs : base absente → `BaseMissing` ; base dérivée → `BaseNotBase` ; boucle → `InheritanceCycle` (chemin complet, premier UID répété en fin ; la chaîne des bases est suivie au plus 8 pas). Exemples : dérivé de Standard dont seul le vendredi vaut [08:00,12:00) : vendredi 240, lundi 480 ; base passée au lundi 09:00-12:00 et 13:00-17:00 : le lundi du dérivé vaut 420 ; dérivé fondé sur un dérivé → `BaseNotBase` ; 5 → 6 → 5 → `InheritanceCycle { chain: [5, 6, 5] }` ; base 99 inexistante → `BaseMissing`.
- **Source** : learn.microsoft.com, schéma MSPDI, « XML Schema for the Calendars Element » (S1 : `BaseCalendarUID` « only used if this calendar is not a base calendar ») ; support.microsoft.com, « Create a new base calendar » (S7 : « the base calendar updates the resource calendar »). Traitement des erreurs : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r015_calendar_inheritance.rs` (`r015_*`) ; tests unitaires de `calendar::resolve` ; propriété P9 (dérivé compilé = même calendrier écrit en entier comme base).

## R-016 — Précédence de résolution d'un jour
- **Formulation** : du plus prioritaire au moins prioritaire, jour de semaine par jour de semaine (`Inherit` passe au niveau suivant) : 1. exception du dérivé ; 2. exception de la base ; 3. semaine datée du dérivé ; 4. semaine par défaut du dérivé ; 5. semaine datée de la base ; 6. semaine par défaut de la base. Exemples, base Standard avec le 25/12/2026 chômé et dérivé « Paul » dont le vendredi vaut [08:00,12:00) : ven 18/12 → 240 ; ven 25/12 → 0 (l'exception de la base l'emporte sur le jour du dérivé) ; lun 21/12 → 480 ; si Paul a sa propre exception le 25/12, ouvrée [09:00,11:00) → 120. Base avec une semaine datée du 01/07/2027 au 31/08/2027 à lun-ven [08:00,15:00) : ven 02/07/2027 → 240 (la semaine par défaut du dérivé l'emporte sur la semaine datée de la base) ; lun 05/07/2027 → 420.
- **Source** : support.microsoft.com, « Add a holiday to the project calendar » (S12 : les fériés de la base valent pour les ressources), « Create a new base calendar » (S7). Ordre exact : hypothèse, la plus lourde pour la parité (à confronter en priorité à un cas de corpus avec calendrier de ressource et fériés).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r016_resolution_precedence.rs` (`r016_*`) ; tests unitaires de `calendar::resolve` ; propriété P8 (forme compilée = `resolve_day`).

## R-017 — Exceptions datées
- **Formulation** : la période est en jours entiers, bornes incluses ; l'exception remplace la journée entière, soit chômée, soit avec ses propres plages. Chevauchement dans un même calendrier (jugé sur les jours dépliés, R-018) : refusé par une commande (`OverlappingException`) ; à l'import, la première (par date de début, puis ordre de stockage) l'emporte sur les jours communs, avec l'avertissement `CalendarExceptionsOverlap { kept, ignored }`. Exemples : samedi 24/10/2026 ouvré [08:00,12:00) : 240, `working_between(ven 23/10 17:00, lun 26/10 08:00)` = 240 et `add_working(ven 23/10 13:00, 480)` = sam 24/10 12:00 ; jeu 24/12 ouvré [08:00,12:00) : `add_working(jeu 24/12 08:00, 480)` = ven 25/12 12:00, et si le 25/12 est aussi chômé → lun 28/12 12:00 ; exception chômée du 24/12/2026 au 31/12/2026 : `add_working(mer 23/12 16:00, 480)` = ven 01/01/2027 16:00, et si le 01/01/2027 est aussi chômé → lun 04/01/2027 16:00 ; deux exceptions sur le 11/11 (chômée en premier, ouvrée 8-12 ensuite) : jour chômé et diagnostic.
- **Source** : learn.microsoft.com, schéma MSPDI, « Calendar Element » (S1 : exemple d'exception sur deux jours) ; support.microsoft.com, « Add a holiday to the project calendar » (S12). Chevauchement : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r017_calendar_exceptions.rs` (`r017_*`).

## R-018 — Exceptions récurrentes
- **Formulation** : les récurrences sont dépliées en jours explicites, toujours restreintes à la période et limitées à `occurrences` occurrences si ce champ est renseigné (ordre chronologique ; `Some(0)` : aucun jour). Elles ne sont jamais lues comme une période continue. `Period` : chaque jour de la période ; `Daily { every }` : first + k × every ; `Weekly { every, days }` : semaines (commençant le dimanche) comptées depuis celle qui contient `period.first`, une sur `every`, jours cochés ; `MonthlyByDay` et `MonthlyByPosition` : un mois sur `every`, compté depuis le mois de `period.first`, un jour inexistant (31 avril) est sauté ; `YearlyByDay` et `YearlyByPosition` : chaque année, le 29 février d'une année non bissextile est sauté ; position : 1er à 4e ou dernier ; `MonthItem` : jour calendaire, jour ouvré lun-ven, jour de week-end, ou jour de semaine donné ; `ByDayCount` et `ByWeekdayCount` : ignorées, avec l'avertissement `CalendarRecurrenceUnsupported`. Exemples : Daily every 2 du lun 02/11/2026 au ven 06/11, chômé : lun, mer, ven chômés et `working_between(02/11 00:00, 07/11 00:00)` = 960 ; Daily every 1, 2 occurrences, à partir du jeu 26/11/2026 : 26 et 27/11 ; YearlyByDay 25 décembre du 25/12/2026 au 25/12/2028 : 3 jours, et le 15/06/2027 reste à 480 ; Weekly every 1, vendredi, janvier 2027 : 1, 8, 15, 22, 29 ; Weekly every 2, lundi, du mer 04/11/2026 au 06/12/2026 : 16/11 et 30/11 ; MonthlyByPosition dernier vendredi, janvier à mars 2027 : 29/01, 26/02, 26/03 ; MonthlyByDay 31, janvier à avril 2027 : 31/01 et 31/03 ; YearlyByPosition 4e jeudi de novembre, 2026-2027 : 26/11/2026 et 25/11/2027 ; ByDayCount : aucun jour et un avertissement.
- **Source** : learn.microsoft.com, schéma MSPDI, « XML Schema for the Calendars Element » (S1 : champs `Type`, `Period`, `DaysOfWeek`, `MonthItem`, `MonthPosition`, `Month`, `MonthDay`, `Occurrences`) ; support.microsoft.com, « Add a holiday to the project calendar » (S12 : récurrence et « Range of recurrence »). Sémantique fine (ancrage des semaines, jour inexistant sauté, `Occurrences` nul, types 7 et 8) : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r018_recurring_exceptions.rs` (`r018_*`) ; tests unitaires de `calendar::recurrence`.

## R-019 — Semaines de travail datées
- **Formulation** : sur leur période (jours inclus), elles remplacent la semaine par défaut du même calendrier pour les jours qu'elles définissent ; `Inherit` renvoie à la semaine par défaut du même calendrier. Chevauchement : comme R-017 (la première par date de début, puis ordre de stockage, possède les jours communs, y compris ses jours `Inherit`). Exemples : semaine datée du 01/07/2026 au 31/08/2026, lun-ven [08:00,15:00) : `add_working(lun 06/07/2026 08:00, 480)` = mar 07/07 09:00 (420 + 60) ; semaine datée du 01/07/2027 au 31/08/2027, lun-jeu hérités et ven [08:00,12:00) : `working_between(dim 04/07/2027, dim 11/07/2027)` = 2 160.
- **Source** : support.microsoft.com, « Set the general working days and times for a project » (S13 : périodes nommées) ; learn.microsoft.com, schéma MSPDI (S1 : `WorkWeeks`). Hypothèse : le schéma publié ne donne pas de `WorkingTimes` au `WeekDay` d'une `WorkWeek`.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r019_work_weeks.rs` (`r019_*`).

### Conversions

Toutes les conversions dérivent de la fonction cumulée W (R-020), calculée sur la forme compilée du calendrier (segments hebdomadaires ou journaliers avec cumul, ADR-0008). Sauf mention contraire, les exemples utilisent le calendrier Standard et les options par défaut (480/2 400/20) ; ils ont été recalculés par un simulateur naïf qui avance minute par minute. Propriétés P1 à P9 : `crates/planrs-core/tests/calendar_properties.rs`.

## R-020 — Fonction cumulée et temps ouvré entre deux instants
- **Formulation** : W(t) = minutes ouvrées dans [1984-01-01 00:00, t). `working_between(a, b) = W(b) − W(a)`, signé, antisymétrique et additif. Exemples : `working_between(lun 19/10 08:00, ven 23/10 17:00)` = 2 400 ; `(lun 12:00, lun 13:00)` = 0 ; `(ven 23/10 15:00, lun 26/10 10:00)` = 240, et −240 dans l'autre sens ; `(ven 23/10 17:00, lun 26/10 08:00)` = 0.
- **Source** : hypothèse (définition).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r020_working_between.rs` (`r020_*`) ; propriétés P6 (Chasles, antisymétrie, bornes) et P8.

## R-021 — Fin d'une durée ouvrée positive
- **Formulation** : `add_working(t, d > 0)` = plus petit instant u tel que d minutes ouvrées séparent t de u (W(u) = W(t) + d). La fin tombe en fin de plage, jamais au début de la plage suivante. Exemples : lun 19/10 08:00 + 480 = lun 17:00 (et non mar 08:00) ; lun 08:00 + 240 = lun 12:00 (et non 13:00) ; + 300 = lun 14:00 ; + 720 = mar 12:00 ; lun 16:00 + 120 = mar 09:00 ; ven 23/10 13:00 + 960 = mar 27/10 12:00 ; sam 24/10 10:00 + 480 = lun 26/10 17:00.
- **Source** : learn.microsoft.com, schéma MSPDI, « Task Element » (S4 : Start 2007-11-14 08:00, Duration PT16H, Finish 2007-11-15 17:00) ; support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r021_add_working_forward.rs` (`r021_*`) ; propriétés P1, P2 et P8.

## R-022 — Décalage négatif : compte à rebours en convention de début
- **Formulation** : `add_working(t, −d)` = plus grand instant s tel que d minutes ouvrées séparent s de t (W(s) = W(t) − d). Le résultat tombe au début d'une minute ouvrée. Exemples : mer 21/10 17:00 − 480 = mer 08:00 ; mer 21/10 10:00 − 480 = mar 20/10 10:00 ; lun 26/10 08:00 − 60 = ven 23/10 16:00 ; lun 26/10 09:00 − 60 = lun 08:00 (et non ven 17:00) ; lun 26/10 09:00 − 120 = ven 16:00 ; mar 20/10 12:00 − 240 = mar 08:00 (et non lun 17:00) ; lun 13:00 − 240 = lun 08:00.
- **Source** : support.microsoft.com, « Add lead or lag time to a task » (S15 : avance) ; convention : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r022_add_working_backward.rs` (`r022_*`) ; propriétés P1, P2, P5 et P8.

## R-023 — Durée nulle
- **Formulation** : `add_working(t, 0) = t`, sans recalage, quel que soit t (même sur un calendrier sans temps ouvré). Le placement d'un jalon relève de R-031. Exemples : lun 17:00 + 0 = lun 17:00 ; sam 10:00 + 0 = sam 10:00.
- **Source** : Microsoft Q&A, « Project 2016 milestone tasks ignore non-working time » (Q3, communautaire : une durée nulle ne consomme pas de temps ouvré).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r023_add_working_zero.rs` (`r023_*`).

## R-024 — Instant ouvré suivant
- **Formulation** : `next_working_moment(t)` = plus petit u ≥ t dont la minute [u, u+1) est ouvrée. La fonction est idempotente. Exemples : sam 24/10 10:00 → lun 26/10 08:00 ; lun 12:00 → 13:00 ; lun 17:00 → mar 08:00 ; lun 07:00 → 08:00 ; lun 09:30 → 09:30 ; mer 11/11 10:00, jour férié → jeu 12/11 08:00.
- **Source** : support.microsoft.com, « Change default times for entered start or finish dates » (S9 : début à 7:00 sur un calendrier à 8:00 → la tâche commence à 8:00).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r024_next_working_moment.rs` (`r024_*`) ; propriétés P3, P4 et P8.

## R-025 — Instant ouvré précédent
- **Formulation** : `previous_working_moment(t)` = plus grand u ≤ t tel que la minute [u − 1, u) est ouvrée. S'il n'en existe aucun : erreur `OutOfDomain { Before }`. Exemples : lun 26/10 08:00 → ven 23/10 17:00 ; lun 13:00 → lun 12:00 ; dim 25/10 10:00 → ven 17:00 ; lun 09:30 → 09:30 ; lun 1984-01-02 08:00 → erreur (première minute ouvrée du domaine).
- **Source** : hypothèse (symétrique de R-024, sert à la passe arrière).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r025_previous_working_moment.rs` (`r025_*`) ; propriétés P3, P4 et P8.

## R-026 — Un jour vaut `MinutesPerDay`, quelles que soient les plages du calendrier
- **Formulation** : une durée en jours est convertie en minutes par R-006, puis consommée sur le calendrier. Exemples : calendrier lun-ven [08:00,12:00) et [13:00,16:00) (420 min/jour) avec 480 min/jour : lun 19/10 08:00 + 1 j = mar 20/10 09:00 ; calendrier [08:00,12:00) et [13:00,18:00) (540 min/jour) : lun 08:00 + 1 j = lun 17:00 ; Standard avec 420 min/jour : lun 08:00 + 2 j (840) = mar 15:00.
- **Source** : brief § 5.1 ; support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r026_day_uses_minutes_per_day.rs` (`r026_*`).

## R-027 — Week-ends et jours fériés sautés
- **Formulation** : le temps chômé ne consomme pas de durée (conséquence de R-020 et R-021, testée à part). Exemples, avec un jour férié le mer 11/11/2026 : lun 09/11 08:00 + 5 j (2 400) = lun 16/11 17:00, et `working_between` entre ces deux instants = 2 400 ; `working_between(lun 09/11 08:00, ven 13/11 17:00)` = 1 920 ; ven 06/11 13:00 + 3 j (1 440) = jeu 12/11 12:00. Avec un jour férié le ven 25/12 : jeu 24/12 08:00 + 2 j = lun 28/12 17:00.
- **Source** : learn.microsoft.com, schéma MSPDI, « DurationFormat Element » (S3 : « 7d » du lundi au vendredi, puis lundi et mardi, contre « 7ed »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r027_weekend_and_holiday_skipped.rs` (`r027_*`).

## R-028 — Temps écoulé
- **Formulation** : temps civil pur, 24 h sur 24 et 7 j sur 7, sans calendrier, sans exception, sans recalage du début. Exemples : ven 23/10 17:00 + 2 ej = dim 25/10 17:00 ; + 1 esem = ven 30/10 17:00 ; + 3 ej = lun 26/10 17:00 ; 2026-02-01 00:00 + 1 emois = 2026-03-03 00:00 ; `forward_span(Elapsed, sam 24/10 10:00, 1 440)` = sam 10:00 → dim 10:00.
- **Source** : learn.microsoft.com, « DurationFormat Element » (S3 : « Elapsed time counts all time, including non-working time ») ; absence de recalage : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r028_elapsed_time.rs` (`r028_*`) ; propriété P10.

## R-029 — Calendrier sans temps ouvré, sortie du domaine
- **Formulation** : calendrier sans aucune minute ouvrée : `add_working(d ≠ 0)`, `next_working_moment` et `previous_working_moment` → `NoWorkingTime` ; `working_between` = 0 ; `add_working(t, 0)` = t ; diagnostic `CalendarNoWorkingTime` (émis par `CalendarSet::compile`). Cible hors de 1984-2149 : `OutOfDomain`, avec sa direction (`After` si W(t) + d dépasse le total, `Before` si W(t) < |d|). Jamais de boucle, jamais de panique (le coût d'une conversion ne dépend pas de la distance parcourue). Exemples : semaine entièrement chômée, sans exception : lun 08:00 + 60 → `NoWorkingTime` ; Standard : mer 2149-12-31 16:00 + 60 = 17:00, + 61 → `OutOfDomain { After }` ; Standard : lun 1984-01-02 09:00 − 60 = 08:00, − 61 → `OutOfDomain { Before }` ; semaine chômée avec seulement le 01/12/2026 ouvré [08:00,12:00) : 01/12 08:00 + 600 → `OutOfDomain { After }`.
- **Source** : support.microsoft.com, « Specifications for Microsoft Project » (S11 : bornes) ; hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r029_no_working_time_and_domain.rs` (`r029_*`) ; diagnostic dans les tests unitaires de `calendar::compiled` (`CalendarSet`).

### Placement

Le placement vit dans `TimeScale::forward_span` et `TimeScale::backward_span` (`crates/planrs-core/src/calendar/scale.rs`), seuls points d'entrée du CPM (J2) avec `add` et `between` : les conventions de début, de fin et de jalon ne vivent qu'ici. Une durée négative est refusée (`NegativeSpan`).

## R-030 — Placement d'une durée > 0
- **Formulation** : passe avant : début = instant ouvré suivant la date au plus tôt (R-024) ; fin = début + durée (R-021). Passe arrière : fin = instant ouvré précédant la date au plus tard (R-025) ; début = fin − durée (R-022). En temps écoulé : aucun recalage (R-028). Exemples : `forward_span(lun 12:00, 240)` = 13:00 → 17:00 ; `forward_span(ven 23/10 17:00, 480)` = lun 26/10 08:00 → 17:00 ; `forward_span(sam 24/10 10:00, 480)` = lun 08:00 → 17:00 ; `backward_span(lun 26/10 08:00, 480)` = ven 23/10 08:00 → 17:00 ; `backward_span(lun 13:00, 240)` = lun 08:00 → 12:00.
- **Source** : support.microsoft.com, « Change default times for entered start or finish dates » (S9) ; hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r030_task_span.rs` (`r030_*`) ; propriété P10.

## R-031 — Jalon (durée nulle)
- **Formulation** : passe avant : t si t termine une minute ouvrée ou se trouve dans une plage (`previous_working_moment(t) == t`), sinon l'instant ouvré suivant. Passe arrière, symétrique : t si `next_working_moment(t) == t`, sinon l'instant ouvré précédent. Début = fin. Exemples, passe avant : ven 23/10 17:00 → ven 17:00 (et non lun 08:00) ; lun 08:00 → lun 08:00 ; lun 12:00 → lun 12:00 ; lun 12:30 → 13:00 ; sam 10:00 → lun 26/10 08:00. Passe arrière : sam 10:00 → ven 17:00 ; lun 08:00 → lun 08:00 ; lun 12:30 → 12:00.
- **Source** : hypothèse. Un jalon lié en FS s'affiche à la fin de son prédécesseur. Microsoft Q&A, « Project 2016 milestone tasks ignore non-working time » (Q3, communautaire) suggère au contraire l'absence de recalage : hypothèse concurrente, que le corpus tranchera.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r031_milestone_placement.rs` (`r031_*`).

### Projet

## R-032 — Valeurs par défaut d'un projet neuf
- **Formulation** : 480 min/jour, 2 400 min/semaine, 20 jours/mois ; début 08:00, fin 17:00 ; unité de durée : jour ; unité de travail : heure ; tâche à unités fixes, non pilotée par l'effort, estimée ; planification depuis le début ; respect des contraintes ; semaine commençant le lundi (affichage) ; calendriers Standard (UID 1, calendrier du projet), 24 heures (UID 2), Équipe de nuit (UID 3) ; date de création = date courante injectée. Nouvelle tâche : « 1j? » (une journée de `MinutesPerDay`, soit 480 min, dans l'unité de durée par défaut, estimée), contrainte « le plus tôt possible », priorité 500, mode automatique. Ces valeurs sont celles de l'application ; les valeurs du schéma MSPDI pour un élément absent à l'import (`DefaultTaskType` = durée fixe, `NewTasksEffortDriven` = vrai) relèvent de `planrs-mspdi` (J3).
- **Source** : support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8), « Change default times for entered start or finish dates » (S9), « Change the default task duration unit » (S10), « Project Information dialog box » (S6 : planification depuis le début), « Duration (task field) » (S14 : « 1d? ») ; learn.microsoft.com, schéma MSPDI, élément Project (S2). Type de tâche, pilotage par l'effort, UID des calendriers, premier jour de la semaine et durée d'une nouvelle tâche hors unité « jour » : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r032_new_project_defaults.rs` (`r032_*`).

## R-033 — Date saisie sans heure
- **Formulation** : un début reçoit l'heure de début par défaut, une fin l'heure de fin par défaut ; une heure par défaut de 24:00 donne le lendemain 00:00. L'alignement sur le calendrier se fait ensuite par R-030. Exemples : 2026-10-19 en début → 08:00 ; en fin → 17:00 ; avec un début par défaut à 07:00 → 07:00, puis `forward_span(07:00, 480)` démarre à 08:00 (exemple de S9).
- **Source** : support.microsoft.com, « Change default times for entered start or finish dates » (S9).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r033_date_without_time.rs` (`r033_*`), dont le recalage par `forward_span`.

## R-034 — Date d'état effective
- **Formulation** : sans date d'état (NA), la date d'état est la date courante injectée. Exemples : date d'état NA et date courante 2026-10-17 08:00 → 2026-10-17 08:00 ; date d'état 2026-10-15 17:00 → 2026-10-15 17:00.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « If you set the status date to NA, Project uses the current date »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r034_effective_status_date.rs` (`r034_*`).
