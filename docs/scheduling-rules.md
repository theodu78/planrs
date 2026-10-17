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

Sources citées : pages Microsoft uniquement ; « Q » = Microsoft Q&A, communautaire, jamais suffisant pour le statut « vérifié ». Aucune source tierce (MPXJ, ProjectLibre, GanttProject, autres clones) n'a été lue. Les codes S1 à S26 et Q1 à Q9 (J1, J2), S27 et suivants (J3) renvoient à la table ci-dessous, les codes O1 à O13 (J2) et O14 et suivants (J3) aux observations du corpus (données de fichiers enregistrés par MS Project, lues comme des données) ; chaque règle reprend aussi le nom de la page. Repère : 1984-01-01 est un dimanche ; `Moment(2026-10-19T08:00)` = 22 510 560.

| Réf. | Page | Utilisée pour |
|---|---|---|
| S1 | learn.microsoft.com, schéma MSPDI, « XML Schema for the Calendars Element » et « Calendar Element » | WeekDay/DayType (1 = dimanche), `WorkingTimes` (au plus 5), exceptions (`Type` 1 à 9, `Period`, `DaysOfWeek`, `MonthItem`, `MonthPosition`, `Month`, `MonthDay`, `Occurrences`), `WorkWeeks`, `BaseCalendarUID` (« only used if this calendar is not a base calendar »), `IsBaseCalendar`, exemple d'exception sur deux jours |
| S2 | learn.microsoft.com, schéma MSPDI, « XML Schema for the Project Element », `MinutesPerDay`, `DaysPerMonth` | `MinutesPerDay`, `MinutesPerWeek`, `DaysPerMonth` (entiers), `DefaultStartTime`, `DefaultFinishTime`, `DefaultTaskType` (`default="1"`), `NewTasksEffortDriven` (`default="true"`), `ScheduleFromStart`, `StatusDate`, `CurrentDate`, `CalendarUID`, `WeekStartDay` ; `CriticalSlackLimit` (entier, jours), `MultipleCriticalPaths` (défaut faux) |
| S3 | learn.microsoft.com, schéma MSPDI, « DurationFormat Element » | Codes m/em/h/eh/d/ed/w/ew/mo/emo/%/e% et « ? » ; « Elapsed time counts all time, including non-working time » ; « 7d … 7ed » |
| S4 | learn.microsoft.com, schéma MSPDI, « Task Element », « PredecessorLink Element », « LinkLag Element » | UID/ID (« UID 4 although the task ID = 3 »), GUID, `IsNull`, exemple Start 2007-11-14 08:00 + PT16H → Finish 2007-11-15 17:00 ; `LinkLag` « in tenths of a minute » ; `Summary`, `Critical`, `Early*`, `Late*`, `TotalSlack`, `FreeSlack`, `PredecessorLink/Type` (0 = FF, 1 = FS, 2 = SF, 3 = SS), `LagFormat` |
| S5 | learn.microsoft.com, schéma MSPDI, « Resources Element », « Assignments Element » | Type de ressource, `MaxUnits` (défaut 1.0), taux, `Units`, `Work`, `Delay`, `WorkContour`, lignes de base |
| S6 | support.microsoft.com, « Project Information dialog box » | Standard (lun-ven 8:00-17:00, pause 12:00-13:00), 24 Hours (« All time … 12:00 A.M. to 12:00 A.M. »), Night Shift (« Monday night through Saturday morning, 11:00 P.M. to 8:00 A.M. … break between 3:00 A.M. and 4:00 A.M. »), date d'état NA → date du jour, planification depuis le début par défaut |
| S7 | support.microsoft.com, « Create a new base calendar » | Trois calendriers de base fournis ; « the base calendar updates the resource calendar » |
| S8 | support.microsoft.com, « How Project schedules tasks: Behind the scenes » | 1 jour = 8 h, 1 semaine = 40 h, 1 mois = 20 jours ouvrés ; 8:00 et 17:00 par défaut ; nouvelle tâche au début du projet ; calendrier de la tâche, sinon du projet ; récapitulative à durée fixe ; tâches manuelles et automatiques |
| S9 | support.microsoft.com, « Change default times for entered start or finish dates » | Date sans heure → heure par défaut ; début par défaut 7:00 sur un calendrier à 8:00 → la tâche commence à 8:00 |
| S10 | support.microsoft.com, « Change the default task duration unit » | Unité de saisie par défaut : jours |
| S11 | support.microsoft.com, « Specifications for Microsoft Project » | Dates du 1er janvier 1984 au 31 décembre 2149 ; 11 lignes de base ; unités ≤ 60 000 000 ; travail ≤ 1 666 666 666 h ; coût ≤ 999 999 999 999,99 ; plus de 15 000 exceptions par calendrier |
| S12 | support.microsoft.com, « Add a holiday to the project calendar » | Fériés de la base valables pour les ressources ; récurrence et « Range of recurrence » |
| S13 | support.microsoft.com, « Set the general working days and times for a project » | Semaines de travail datées (périodes nommées) |
| S14 | support.microsoft.com, « Duration (task field) » | Nouvelle tâche automatique : « 1d? » |
| S15 | support.microsoft.com, « Predecessors (task field) », « Add lead or lag time to a task » | Avance = décalage négatif (nombre ou pourcentage négatif) ; décalage en % (« -50% ») |
| S16 | support.microsoft.com, « Total Slack (task field) » | TF = min(LF − EF, LS − ES) |
| S17 | support.microsoft.com, « Free Slack (task field) » | Sans successeur, marge libre = marge totale |
| S18 | support.microsoft.com, « Start Slack (task field) », « Finish Slack (task field) » | LS − ES, LF − EF |
| S19 | support.microsoft.com, « Critical fields » | Critique si la marge totale ≤ seuil, seuil 0 par défaut |
| S20 | support.microsoft.com, « Manage your project's critical path » | Option de seuil ; « multiple critical paths for each independent network or series of tasks » |
| S21 | support.microsoft.com, « Task Information dialog box, Predecessors tab » | Décalage compté sur le calendrier de tâche du successeur, sinon du projet ; % de la durée du prédécesseur ; « 2ed ». Signe de l'avance inversé sur cette page : erreur de la page, S15 fait foi |
| S22 | support.microsoft.com, « Early Start », « Late Start », « Late Finish (task field) » | Début au plus tôt depuis les prédécesseurs ; fin au plus tard initiale = fin du projet |
| S23 | support.microsoft.com, « Indent or outdent tasks in your project » | Récapitulative : début le plus tôt, fin la plus tardive des sous-tâches |
| S24 | learn.microsoft.com, « Tasks aren't scheduled as expected in Microsoft Project » | Une sous-tâche ne commence pas avant sa récapitulative liée |
| S25 | support.microsoft.com, « Inactivate a task » | Une tâche inactive n'affecte pas le planning |
| S26 | support.microsoft.com, « Percent (%) Complete fields », « Actual Duration (task field) » | Durée réelle = durée × % achevé |
| S27 | learn.microsoft.com, « Definition of Microsoft Project constraints » (KB 74978) | ALAP « ES=(Calculated)LS » ; ASAP « (Scheduled)ES=(Calculated)ES + Delay » ; FNET « EF=CD » ; FNLT « LF=CD » ; MFO « EF,LF,SF=CD » ; MSO « ES,LS,SS=CD » ; SNET « ES=CD » ; SNLT « LS=CD ». Les conditions écrites pour FNET et SNET (« if Date<EF », « if Date<ES ») sont inversées : erreur de la page, consignée comme celle de S21 |
| S28 | support.microsoft.com, « Set a task start or finish date (constraint) for a task » | Catégories flexible / semi-flexible / inflexible ; ASAP par défaut depuis le début ; ALAP « the default constraint when you schedule from the project finish date » |
| S29 | support.microsoft.com, « Constraint Type (task field) » | Défauts ASAP/ALAP selon le sens ; « All the constraints except As Soon As Possible and As Late As Possible require a date » |
| S30 | learn.microsoft.com, blog archivé « Back to Basics: Working with task date constraints » (2008) | Date saisie à la main → SNET ou FNET ; exemple MFO : « Project warns us of the scheduling conflict, and then overlaps the tasks » |
| S31 | support.microsoft.com, « Show slack in your project » | « If you enter a deadline date before the end of the task's total slack, total slack will be recalculated » ; marge négative après une échéance dépassée ; exemple FS vers une tâche MSO plus tôt → marge négative |
| S34 | learn.microsoft.com, schéma MSPDI (élément Project, `HonorConstraints`, `ScheduleFromStart`, `FinishDate`, `ConstraintType`) | `HonorConstraints` `default="true"` ; `ScheduleFromStart` `default="true"` ; `FinishDate` « Required if ScheduleFromStart is false » ; `ConstraintType` 0 à 7 |
| Q1 | Microsoft Q&A (communautaire), « Quick switch from mons to emons » | emo = 30 jours calendaires |
| Q2 | Microsoft Q&A (communautaire), « Microsoft Project task duration days not displaying » | Changer les heures par jour réinterprète l'affichage, pas les heures stockées |
| Q3 | Microsoft Q&A (communautaire), « Project 2016 milestone tasks ignore non-working time » | Une durée nulle ne consomme pas de temps ouvré |
| Q4 | Microsoft Q&A (communautaire), fil 5110117 | Durée d'une récapitulative = écart début-fin sur son calendrier ; % achevé = Σ durées réelles / Σ durées des feuilles |
| Q5 | Microsoft Q&A (communautaire), fil f54ec732 | Marge en minutes de l'échelle effective ; tâche à durée écoulée : marge en temps civil (15 h, 63 h) ; seuil en durée (1 j = 8 h) |
| Q7 | Microsoft Q&A (communautaire), fil 5029993 | Lien entre une récapitulative et sa sous-tâche refusé |
| Q9 | Microsoft Q&A (communautaire), fil 4877826 | Sans contrainte, fin au plus tard d'une tâche finale = fin du projet |
| Q10 | Microsoft Q&A (communautaire), « MSP 2013: no negative value in free slack? » | « Free Slack will always show a 0d value, even though the Total Slack field shows a negative value » |
| Q13 | Microsoft Q&A (communautaire), fil 5182424 | L'option « respecter les contraintes » n'agit que sur les contraintes qui créent une marge négative (SNLT, FNLT, MSO, MFO depuis le début) |

Q6 et Q8 ne sont pas cités (Q6, « une récapitulative est critique si un enfant l'est », est contredite par le corpus A1 ; Q8, sur les marges négatives, est remplacé par Q10 et Q13 en J3).

Observations du corpus (fichiers du manifeste `tests/corpus/manifest.toml`, ADR-0009 ; données seulement). Elles fondent des hypothèses ; une règle ne passe à `vérifié contre MS Project` que quand le harnais reproduit le cas à l'identique. Codes des fichiers : A1 = `cogito-advanced-sample-msp2010.xml`, A2 = `practicemap.xml`, A4 = `shopmetrics.xml`, A6 = `takeout-system-zh.xml`, A7 = `projinteg.xml` (A3 et A5 écartés). Parité mesurée par le harnais (`tests/corpus`, 2026-10-17) : 91 cas identiques sur 159 (A1 51/51, A2 18/18, A4 22/42, A6 0/37, A7 0/11) ; détail dans `docs/scorecard.md`.

| Réf. | Fichier, UID | Observation |
|---|---|---|
| O1 | A2 UID 1 | Jalon sans prédécesseur au début du projet (mar 08:00) ; successeur UID 3, LS mar 08:00 → LateStart = LateFinish = mar 08:00, TF 0 : R-031 en passe arrière |
| O2 | A2 UID 2, 6, 14 | `TotalSlack` en dixièmes (345 600 = 72 j) ; marge libre 0 quand le successeur commence au matin ouvré suivant ; tâche finale : LF = fin du projet |
| O3 | A1 UID 108 | Récapitulative : LS = min LS des enfants, LF = max LF, TF = min(LS − ES, LF − EF) = 105 600 ; `Critical` 0 alors qu'un enfant (UID 110) est critique |
| O4 | A1 UID 17, 18, 53, 54, 105 | Récapitulative sans lien sortant : `FreeSlack` = `TotalSlack` |
| O5 | A6 UID 10, 12, 14 | Récapitulative avec lien sortant : marge libre = écart entre sa fin agrégée et le début du successeur |
| O6 | A6 UID 9, 11, 35 | Lien FS vers une récapitulative : ses sous-tâches ne commencent pas avant la borne ; lien FS depuis une récapitulative : borne = sa fin agrégée |
| O7 | A6 UID 37, 38, 44 | FF : début = `backward_span(borne, d).start` |
| O8 | A7 UID 1, 5, 6 | Récapitulative manuelle : dates saisies, différentes du min/max des enfants |
| O9 | A7 UID 7, 35 | Feuille manuelle : `EarlyStart`/`EarlyFinish` = dates saisies |
| O10 | A6 UID 20, 36 | `LevelingDelay` ≠ 0 : `Start` = `EarlyStart` + retard (divergence connue, J5) |
| O11 | A4 | Calendrier du projet dérivé ; marge libre de 2 h entre une fin à 17:00 et un début le matin ouvré suivant : cause non établie, divergence attendue |
| O12 | A6 | Calendrier du projet ouvré le samedi, tâches affectées jamais placées le samedi : calendrier de ressource probable (J5), divergence attendue |
| O13 | A4 UID 0, 1, 5, 8, 12… (récapitulatives) ; UID 14, 20, 21, 24… (feuilles affectées) | Mesuré par le harnais : `Duration` enregistrée = durée du moteur − 120 min sur les 11 récapitulatives et l'UID 0 ; `FreeSlack` 1 200 (2 h) sur 8 feuilles affectées là où le moteur donne 0 (O11). Les deux écarts sont ceux d'une mesure sur le calendrier de base UID 2 (09:00-13:00, 15:00-19:00) : dernier jour terminé à 17:00 = 6 h au lieu de 8 h ; de 17:00 au matin ouvré suivant = 2 h. Hypothèse, non retenue sans second cas : un calendrier de projet dérivé (que MS Project n'accepte pas comme calendrier de projet) fait mesurer durées des récapitulatives et marges libres sur sa base, les feuilles étant placées par le calendrier UID 1 de leurs ressources (J5) |
| O14 | A7 UID 15 à 21, 32 | Feuilles automatiques FNET (`ConstraintType` 6), sans lien, avec une affectation à la ressource −65535 ; calendrier du projet 5 (ven 18:00-20:00 et 21:00-23:00, sam 09:00-13:00 et 14:00-18:00). `EarlyFinish` = `Finish` = `ConstraintDate` brute (ven 18:00, début d'une plage) ; `EarlyStart` = `add(date, −d)`. UID 15 : FNET ven 18/10/2024 18:00, 8 h → sam 12/10 09:00 – ven 18/10 18:00 ; UID 21 : 12 h, FNET ven 29/11 18:00 → ven 22/11 18:00 – ven 29/11 18:00. Dates au plus tard tirées de la fin du projet ven 20/12 18:00 par R-030 (LF sam 14/12 18:00) : le FNET n'agit pas en passe arrière. TF = FS (64 800 dixièmes pour UID 15). Les dates au plus tard étaient déjà identiques en J2 ; seules les dates au plus tôt et les marges divergeaient |
| O15 | A1, A2, A4, A6, A7 | `HonorConstraints` = 0 dans les 5 fichiers, alors que le schéma donne `default="true"` et que l'option est cochée par défaut (S8, S24). Aucun conflit contrainte/lien dans le corpus : la lecture littérale et le codage inversé donnent les mêmes dates |
| O17 | A6 UID 7 | FNET jeu 14/11/2024 17:00 avec un prédécesseur FF et une porte qui donnent une fin plus tardive : la contrainte ne lie pas |

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
- **Tests** : `tests/rules/r016_resolution_precedence.rs` (`r016_*`) ; tests unitaires de `calendar::resolve` ; propriété P8 (forme compilée = `resolve_day` = oracle indépendant écrit dans le fichier de test, qui ne réutilise ni le dépliage, ni les tris, ni le choix de la semaine datée de `planrs-core`).

## R-017 — Exceptions datées
- **Formulation** : la période est en jours entiers, bornes incluses ; l'exception remplace la journée entière, soit chômée, soit avec ses propres plages. Chevauchement dans un même calendrier (jugé sur les jours dépliés, R-018) : refusé par une commande (`OverlappingException`) ; à l'import, la première (par date de début, puis ordre de stockage) l'emporte sur les jours communs, avec l'avertissement `CalendarExceptionsOverlap { kept, ignored }`. Exemples : samedi 24/10/2026 ouvré [08:00,12:00) : 240, `working_between(ven 23/10 17:00, lun 26/10 08:00)` = 240 et `add_working(ven 23/10 13:00, 480)` = sam 24/10 12:00 ; jeu 24/12 ouvré [08:00,12:00) : `add_working(jeu 24/12 08:00, 480)` = ven 25/12 12:00, et si le 25/12 est aussi chômé → lun 28/12 12:00 ; exception chômée du 24/12/2026 au 31/12/2026 : `add_working(mer 23/12 16:00, 480)` = ven 01/01/2027 16:00, et si le 01/01/2027 est aussi chômé → lun 04/01/2027 16:00 ; deux exceptions sur le 11/11 (chômée en premier, ouvrée 8-12 ensuite) : jour chômé et diagnostic.
- **Source** : learn.microsoft.com, schéma MSPDI, « Calendar Element » (S1 : exemple d'exception sur deux jours) ; support.microsoft.com, « Add a holiday to the project calendar » (S12). Chevauchement : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r017_calendar_exceptions.rs` (`r017_*`), dont un cas où l'ordre de stockage diffère de l'ordre des dates de début (`r017_storage_order_differs_from_start_order`) ; propriété P8 (oracle indépendant).

## R-018 — Exceptions récurrentes
- **Formulation** : les récurrences sont dépliées en jours explicites, toujours restreintes à la période et limitées à `occurrences` occurrences si ce champ est renseigné (ordre chronologique ; `Some(0)` : aucun jour). Elles ne sont jamais lues comme une période continue. `Period` : chaque jour de la période ; `Daily { every }` : first + k × every ; `Weekly { every, days }` : semaines (commençant le dimanche) comptées depuis celle qui contient `period.first`, une sur `every`, jours cochés ; `MonthlyByDay` et `MonthlyByPosition` : un mois sur `every`, compté depuis le mois de `period.first`, un jour inexistant (31 avril) est sauté ; `YearlyByDay` et `YearlyByPosition` : chaque année, le 29 février d'une année non bissextile est sauté ; position : 1er à 4e ou dernier ; `MonthItem` : jour calendaire, jour ouvré lun-ven, jour de week-end, ou jour de semaine donné ; `ByDayCount` et `ByWeekdayCount` : ignorées, avec l'avertissement `CalendarRecurrenceUnsupported`. Exemples : Daily every 2 du lun 02/11/2026 au ven 06/11, chômé : lun, mer, ven chômés et `working_between(02/11 00:00, 07/11 00:00)` = 960 ; Daily every 1, 2 occurrences, à partir du jeu 26/11/2026 : 26 et 27/11 ; YearlyByDay 25 décembre du 25/12/2026 au 25/12/2028 : 3 jours, et le 15/06/2027 reste à 480 ; Weekly every 1, vendredi, janvier 2027 : 1, 8, 15, 22, 29 ; Weekly every 2, lundi, du mer 04/11/2026 au 06/12/2026 : 16/11 et 30/11 ; MonthlyByPosition dernier vendredi, janvier à mars 2027 : 29/01, 26/02, 26/03 ; MonthlyByDay 31, janvier à avril 2027 : 31/01 et 31/03 ; YearlyByPosition 4e jeudi de novembre, 2026-2027 : 26/11/2026 et 25/11/2027 ; ByDayCount : aucun jour et un avertissement.
- **Source** : learn.microsoft.com, schéma MSPDI, « XML Schema for the Calendars Element » (S1 : champs `Type`, `Period`, `DaysOfWeek`, `MonthItem`, `MonthPosition`, `Month`, `MonthDay`, `Occurrences`) ; support.microsoft.com, « Add a holiday to the project calendar » (S12 : récurrence et « Range of recurrence »). Sémantique fine (ancrage des semaines, jour inexistant sauté, `Occurrences` nul, types 7 et 8) : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r018_recurring_exceptions.rs` (`r018_*`), dont une récurrence hebdomadaire qui chevauche une exception sur une période (`r018_recurring_exception_overlapping_a_period` : du 02/11 au 07/11/2026, 480 + 120 + 0 + 120 + 120 = 840) ; tests unitaires de `calendar::recurrence` ; propriété P8 (oracle indépendant, récurrences dépliées jour par jour avec chrono).

## R-019 — Semaines de travail datées
- **Formulation** : sur leur période (jours inclus), elles remplacent la semaine par défaut du même calendrier pour les jours qu'elles définissent ; `Inherit` renvoie à la semaine par défaut du même calendrier. Chevauchement (fichier importé seulement : une commande le refuse, `OverlappingWorkWeek`) : comme R-017, la première par date de début, puis ordre de stockage, possède les jours communs, y compris ses jours `Inherit`, qui renvoient à la semaine par défaut et jamais à une autre semaine datée ; avertissement `CalendarWorkWeeksOverlap { kept, ignored }`, `kept` étant la semaine réellement appliquée. Écart à la spécification J1 (qui retenait la première semaine datée *définissant* ce jour de semaine), consigné dans l'ADR-0008. Exemples : semaine datée du 01/07/2026 au 31/08/2026, lun-ven [08:00,15:00) : `add_working(lun 06/07/2026 08:00, 480)` = mar 07/07 09:00 (420 + 60) ; semaine datée du 01/07/2027 au 31/08/2027, lun-jeu hérités et ven [08:00,12:00) : `working_between(dim 04/07/2027, dim 11/07/2027)` = 2 160 ; chevauchement, stockées à rebours : n° 0 du 09/11 au 30/11/2026 avec lundi [08:00,10:00), n° 1 du 01/11 au 15/11/2026 avec mardi chômé, autres jours hérités : lun 09/11 → 480 (n° 1, lundi hérité), mar 10/11 → 0, lun 16/11 → 120, et `working_between(lun 09/11 00:00, mer 18/11 00:00)` = 2 520.
- **Source** : support.microsoft.com, « Set the general working days and times for a project » (S13 : périodes nommées) ; learn.microsoft.com, schéma MSPDI (S1 : `WorkWeeks`). Hypothèse : le schéma publié ne donne pas de `WorkingTimes` au `WeekDay` d'une `WorkWeek`. Traitement du chevauchement : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r019_work_weeks.rs` (`r019_*`), dont `r019_overlap_storage_order_differs_from_start_order` ; test unitaire `calendar_nested_work_weeks_report_the_applied_week` (`model::validate`) ; propriété P8 (oracle indépendant).

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
- **Formulation** : temps civil pur, 24 h sur 24 et 7 j sur 7, sans calendrier, sans exception, sans recalage du début. Résultat hors de 1984-2149 : `CalendarError::ElapsedOutOfDomain { direction }` (`After` ou `Before`) ; cette variante, propre au temps écoulé, n'a pas d'UID de calendrier, contrairement à `OutOfDomain` (R-029). Exemples : ven 23/10 17:00 + 2 ej = dim 25/10 17:00 ; + 1 esem = ven 30/10 17:00 ; + 3 ej = lun 26/10 17:00 ; 2026-02-01 00:00 + 1 emois = 2026-03-03 00:00 ; `forward_span(Elapsed, sam 24/10 10:00, 1 440)` = sam 10:00 → dim 10:00 ; `add(Elapsed, mer 2149-12-31 23:00, 61)` → `ElapsedOutOfDomain { After }`.
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
- **Formulation** : passe avant : début = instant ouvré suivant la date au plus tôt (R-024) ; fin = début + durée (R-021). Passe arrière : fin = instant ouvré précédant la date au plus tard (R-025) ; début = fin − durée (R-022). En temps écoulé : aucun recalage (R-028), et une fin (passe avant) ou un début (passe arrière) hors du domaine donne `ElapsedOutOfDomain` avec sa direction. Exemples : `forward_span(lun 12:00, 240)` = 13:00 → 17:00 ; `forward_span(ven 23/10 17:00, 480)` = lun 26/10 08:00 → 17:00 ; `forward_span(sam 24/10 10:00, 480)` = lun 08:00 → 17:00 ; `backward_span(lun 26/10 08:00, 480)` = ven 23/10 08:00 → 17:00 ; `backward_span(lun 13:00, 240)` = lun 08:00 → 12:00.
- **Source** : support.microsoft.com, « Change default times for entered start or finish dates » (S9) ; hypothèse.
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 4, 6, 7 ; A2 UID 2, 3, 4 ; A4 UID 2, 3, 4 : 69 cas identiques, calendrier du projet Standard ou dérivé).
- **Tests** : `tests/rules/r030_task_span.rs` (`r030_*`) ; propriété P10.

## R-031 — Jalon (durée nulle)
- **Formulation** : passe avant : t si t termine une minute ouvrée ou se trouve dans une plage (`previous_working_moment(t) == t`), sinon l'instant ouvré suivant. Passe arrière, symétrique : t si `next_working_moment(t) == t`, sinon l'instant ouvré précédent. Début = fin. Exemples, passe avant : ven 23/10 17:00 → ven 17:00 (et non lun 08:00) ; lun 08:00 → lun 08:00 ; lun 12:00 → lun 12:00 ; lun 12:30 → 13:00 ; sam 10:00 → lun 26/10 08:00. Passe arrière : sam 10:00 → ven 17:00 ; lun 08:00 → lun 08:00 ; lun 12:30 → 12:00.
- **Source** : hypothèse. Un jalon lié en FS s'affiche à la fin de son prédécesseur. Microsoft Q&A, « Project 2016 milestone tasks ignore non-working time » (Q3, communautaire) suggère au contraire l'absence de recalage : hypothèse concurrente, que le corpus tranchera.
- **Statut** : `vérifié contre MS Project` (corpus A2 UID 1, 17 ; A4 UID 41 : jalons, dates au plus tôt et au plus tard).
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

### Moteur : réseau, liens, passes, marges (R-035 à R-069)

Le moteur (`planrs_core::schedule`, ADR-0009) calcule un `Schedule` hors du modèle. Notations : `P(t)` = échelle de placement d'une feuille (temps écoulé pour une durée écoulée, sinon calendrier de la tâche, sinon du projet) ; `M(t)` = échelle de marge (= `P(t)` ; calendrier pour une récapitulative) ; `σ` = échelle du décalage d'un lien (R-041) ; `L` = minutes du décalage ; `g(x) = x` si `L = 0`, sinon `σ.add(x, L)` ; `h(y) = y` si `L = 0`, sinon `y − L` en temps écoulé et `σ.next_working_moment(σ.add(y, −L))` en temps ouvré (adjoint, R-047 ; adjoint en brut pour une avance vers une cible qui lit sa borne en `Moment` brut). Les dates « brutes » sont comparées en `Moment`. Sauf mention contraire : calendrier Standard, options 480 / 2 400 / 20, début du projet lun 19/10/2026 08:00, `j` = 480 min. Propriétés sur réseaux aléatoires (`crates/planrs-core/tests/schedule_properties.rs`, nombre de cas réglé par `PROPTEST_CASES`, 10 000 configurés en CI) : P13 (liens respectés, TF ≥ FS ≥ 0, enveloppe des récapitulatives, déterminisme, permutation des liens, point fixe, ordre, dates au plus tard réalisables, retards de TF ou de FS sans effet), P14 (chaîne de marge nulle), P15 (oracle indépendant en temps ouvré, réseau sans jalon), P16 (monotonie de la fin du projet), P17 (chaos, cycles signalés) ; profondeur : `crates/planrs-core/tests/schedule_depth.rs`.

## R-035 — Ordre topologique stable
- **Formulation** : nœuds numérotés en pré-ordre (porte d'une récapitulative, feuille, agrégat à la fermeture du sous-arbre) ; ordre de Kahn départagé par le plus petit indice ; identité si tout lien va vers l'avant de la liste. L'ordre ne dépend ni de l'ordre de stockage des liens ni d'une table de hachage. `Schedule::order` liste les tâches planifiées (une récapitulative à la place de son agrégat). Exemples : T1 (UID 10, ID 1), T2 (UID 5, ID 2), T3 (UID 7, ID 3), lien T3 → T1 : ordre `[5, 7, 10]` ; sans lien : `[10, 5, 7]`.
- **Source** : hypothèse (règle 5).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r035_stable_topological_order.rs` (`r035_*`).

## R-036 — Début du projet, plancher de toute tâche automatique
- **Formulation** : toute borne de début est au moins le début du projet (brut) ; une tâche sans prédécesseur commence au début du projet recalé sur son échelle (R-030, R-031) ; une avance qui ferait commencer avant est tronquée. Le `Schedule` garde le début du projet brut. Exception (ADR-0010, R-105) : option R-099 active, une contrainte dure (serrage, épingle) peut placer une tâche avant le début du projet, avec `constraint.override { ProjectStart }`. Exemples : début sam 17/10 00:00, A 2 j sans lien → lun 19/10 08:00 – mar 20/10 17:00 ; jalon sans lien → lun 08:00 ; A 1 j, B 1 j SS −1 j de A : `add(lun 08:00, −480)` = ven 16/10 08:00 → B lun 08:00 – 17:00, pilote « début du projet ».
- **Source** : support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8), « Early Start » (S22) ; learn.microsoft.com, « Tasks aren't scheduled as expected » (S24) ; troncature : hypothèse.
- **Statut** : `vérifié contre MS Project` pour le début sans avance (corpus A1 UID 4 ; A2 UID 1, 2, 3, 4 ; A4 UID 2) ; troncature d'une avance : `hypothèse` (aucun décalage dans le corpus).
- **Tests** : `tests/rules/r036_project_start_floor.rs` (`r036_*`) ; exception des contraintes dures : `tests/rules/r105_constraint_outside_project.rs`.

## R-037 — Fin à début (FS)
- **Formulation** : borne de début du successeur `g(EF_p)`, puis placement (R-030). Exemples : A 2 j (lun 08:00 – mar 17:00) ; B FS 1 j → mer 21/10 08:00 – 17:00 ; B FS+1 j : `add(mar 17:00, 480)` = mer 17:00 → jeu 22/10 08:00 – 17:00.
- **Source** : S8 ; support.microsoft.com, « Add lead or lag time to a task » (S15).
- **Statut** : `vérifié contre MS Project` pour un décalage nul (corpus A1 UID 6, 7, 8 ; A2 UID 5, 6, 7 ; A4 UID 3, 4, 6 : 66 cas) ; avec décalage : `hypothèse` (aucun décalage dans le corpus).
- **Tests** : `tests/rules/r037_finish_to_start.rs` (`r037_*`).

## R-038 — Début à début (SS)
- **Formulation** : borne de début `g(ES_p)`. Exemple : A 2 j lun 08:00 ; B SS+4 h 1 j : `add(lun 08:00, 240)` = lun 12:00 → B lun 13:00 – mar 12:00.
- **Source** : S8.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r038_start_to_start.rs` (`r038_*`).

## R-039 — Fin à fin (FF)
- **Formulation** : borne de fin `b = g(EF_p)`, convertie en borne de début `P(s).backward_span(b, d).start`. Exemples : A 2 j (fin mar 17:00) ; B FF 1 j → mar 08:00 – 17:00 ; B FF 3 j : la borne convertie (ven 16/10 08:00) précède le début du projet → B lun 08:00 – mer 21/10 17:00.
- **Source** : S8 ; conversion : observée (O7, A6 UID 38 : fin du prédécesseur mar 26/11/2024 17:00, 1 j → début 08:00).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r039_finish_to_finish.rs` (`r039_*`).

## R-040 — Début à fin (SF)
- **Formulation** : borne de fin `b = g(ES_p)`, convertie comme R-039. Exemples : A 2 j (début lun 08:00) ; B SF+3 j 1 j : `add(lun 08:00, 1 440)` = mer 17:00 → B mer 08:00 – 17:00 ; B SF+0 1 j : borne lun 08:00 → `backward_span` = ven 16/10 08:00 – 17:00 → plancher → B lun 08:00 – 17:00.
- **Source** : S8.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r040_start_to_finish.rs` (`r040_*`).

## R-041 — Calendrier du décalage
- **Formulation** : un décalage ouvré se compte sur le calendrier de tâche du successeur d'origine s'il en a un (utilisable, R-066), sinon sur le calendrier du projet ; jamais sur celui du prédécesseur, jamais en temps écoulé parce que le successeur a une durée écoulée. Un décalage écoulé (`ed`, `e%`) se compte en temps civil. Exemples : A (Standard) lun 08:00 – 17:00 ; B (24 heures) FS+1 j : `add₂₄(lun 17:00, 480)` = mar 01:00 → B mar 01:00 – 09:00. A (24 heures) 1 j = lun 08:00 – 16:00 ; B (sans calendrier) FS+1 j : `add_std(lun 16:00, 480)` = mar 16:00 → B mar 16:00 – mer 16:00. Début ven 23/10 08:00, A 1 j, B FS+2 ej : dim 25/10 17:00 → B lun 26/10 08:00 – 17:00. B à durée écoulée de 8 h, FS+1 j de A (lun) : `add_std(lun 17:00, 480)` = mar 17:00 → B mar 17:00 – mer 01:00.
- **Source** : support.microsoft.com, « Task Information dialog box, Predecessors tab » (S21).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r041_lag_calendar.rs` (`r041_*`).

## R-042 — Avance (décalage négatif)
- **Formulation** : un décalage négatif recule la borne (convention de début, R-022). Exemples : A 2 j (fin mar 17:00) ; B FS −1 j 1 j → mar 08:00 – 17:00 ; B FS −2 h 1 j → mar 15:00 – mer 15:00.
- **Source** : S15. La page S21 inverse le signe de l'avance : erreur de la page, consignée ; S15 fait foi.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r042_negative_lag.rs` (`r042_*`).

## R-043 — Décalage en pourcentage
- **Formulation** : `L = percent_of(durée du prédécesseur, pct)` (R-010), consommé sur `σ` (`%` ouvré, `e%` temps civil). Durée du prédécesseur : celle de la feuille ; d'une feuille manuelle, sa durée utilisée (R-063) ; d'une récapitulative, sa durée R-054. Exemples : A 2 j ; B SS+50 % : 480 → lun 17:00 → B mar 08:00 – 17:00 ; B FS −50 % → mar 08:00 – 17:00 ; B SS+50e % : lun 16:00 → B lun 16:00 – mar 16:00 ; A 1 j, B FS+33 % (158) → mar 10:38 – mer 10:38.
- **Source** : S21, S15 ; base pour une récapitulative : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r043_percent_lag.rs` (`r043_*`).

## R-044 — Plusieurs prédécesseurs et pilote
- **Formulation** : ES0 = plus grande des bornes en `Moment` brut ; le pilote (`driver`) est la première borne maximale dans l'ordre : début du projet, porte du parent, liens dans l'ordre de stockage (bornes FF/SF converties comprises). Exemples : A 1 j lun, B 2 j lun – mar, C FS de A et de B, 1 j → mer 08:00 – 17:00, pilote `Link { B, FS }` ; A et B de 1 j : pilote = premier lien stocké. Amendement J3 (ADR-0010) : une contrainte de date n'est pas une borne brute ; elle se compare au placement des autres bornes (R-093, R-094) et ne pilote (`Constraint { kind }`) que si elle déplace le début placé : une date de contrainte plus grande en brut mais de même temps ouvré ne change ni les dates ni le pilote. Exemple : A 1 j FS → B 1 j SNET mar 20/10 07:00 : la date dépasse en brut la borne du lien (lun 17:00), les deux se placent mar 08:00 → B mar 08:00, pilote `Link { A, FS }` ; SNET mar 09:00 → B mar 09:00 – mer 09:00, pilote `Constraint { StartNoEarlierThan }`.
- **Source** : S22 ; départage : hypothèse.
- **Statut** : `vérifié contre MS Project` pour la borne retenue (max des prédécesseurs, corpus A1 UID 59, 109, 110, 113 ; A2 UID 17 ; A4 UID 22, 37, 38) ; choix du pilote en cas d'égalité : `hypothèse` (MSPDI n'enregistre pas le pilote).
- **Tests** : `tests/rules/r044_multiple_predecessors.rs` (`r044_*`, dont `r044_constraint_driver`).

## R-045 — Fin du projet
- **Formulation** : max des fins au plus tôt des feuilles actives planifiées (manuelles comprises, inactives exclues) et des récapitulatives manuelles datées (R-064) ; sans tâche planifiée : début du projet brut. Précision J3 (ADR-0010) : c'est le max des fins seulement, même s'il précède le début du projet (tâche manuelle, contrainte dure respectée avant lui, R-105) ; en J2 le calcul partait du début du projet. Exemple : M manuelle ven 16/10 08:00 – 17:00 seule, début lun 19/10 → fin du projet ven 16/10 17:00. Exemples : réseau N1 (A 1 j ; B 2 j ; C 1 j FS de A et de B ; D 1 j seul) → mer 21/10 17:00 ; projet vide commençant sam 17/10 10:00 → sam 17/10 10:00.
- **Source** : S22, S16 ; projet vide : hypothèse.
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 0, A2 UID 0 : fin de la récapitulative du projet) ; projet vide : `hypothèse`.
- **Tests** : `tests/rules/r045_project_finish.rs` (`r045_*`, dont `r045_finish_before_the_project_start`).

## R-046 — Fin au plus tard sans successeur
- **Formulation** : plafond commun de toute feuille = fin du projet ; une tâche sans successeur effectif (hérités compris) a donc LF = fin du projet. Exemple : N1, D → LF mer 17:00, LS mer 08:00.
- **Source** : support.microsoft.com, « Late Finish (task field) » (S22) ; Microsoft Q&A 4877826 (Q9) ; observé O2 (A2 UID 2).
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 10, 16, 22, 113 ; A2 UID 17 ; A4 UID 41).
- **Tests** : `tests/rules/r046_late_finish_without_successor.rs` (`r046_*`).

## R-047 — Passe arrière : transfert adjoint
- **Formulation** : plafond de fin d'une feuille : FS `h(LS_s)`, FF `h(LF'_s)` ; plafonds de début : SS `h(LS_s)`, SF `h(LF'_s)`, convertis en plafond de fin par `P(t).forward_span(x, d).finish`, où `x = P(t).latest_start(y, d)` est le dernier début plaçable au plus tard en `y` (en brut) : `y` en temps écoulé ; en temps ouvré, `y` s'il commence une minute ouvrée de `P(t)`, sinon `previous_working_moment(y) − 1 min` (`d > 0`) ou `previous_working_moment(y)` (jalon, R-031) ; `late = P(t).backward_span(min des plafonds, d)`. `h` est l'adjoint de `g` : le plus grand `x` tel que `W_σ(g(x)) ≤ W_σ(y)`. Quand la cible lit sa borne en `Moment` brut et non en temps ouvré de `σ` — porte d'une récapitulative, jalon (R-031), feuille placée sur une autre échelle que `σ` (calendrier propre, durée écoulée) —, `h` doit être l'adjoint en brut, le plus grand `x` tel que `g(x) ≤ y` : pour une avance (`L < 0`) en temps ouvré, `g` tombe toujours au début d'une minute ouvrée de `σ`, donc `y` est d'abord ramené à `σ.previous_working_moment(y) − 1 min` s'il n'en commence pas une. Pour FF et SF, l'ancre du successeur est d'abord portée au dernier instant qui ne le fait pas finir plus tard sur son échelle de placement : `LF'_s = P(s).next_working_moment(LF_s)` (identité en temps écoulé ; fin du domaine s'il ne reste aucun temps ouvré après `LF_s`), car la contrainte FF/SF se lit en temps ouvré de `P(s)` (R-039) ; pour un jalon, qu'une borne de fin `b` place en `b` si `b` commence une minute ouvrée, `LF'_s = LF_s` si `LF_s` commence une minute ouvrée, sinon `P(s).next_working_moment(LF_s) − 1 min`. Ces trois conversions garantissent qu'une tâche placée à ses dates au plus tard ne décale ni la fin du projet ni un successeur (S16, S17). Cause de la fin au plus tard (`TaskSchedule::late_driver`, type `LateDriver`, base d'`explain_date` pour les dates au plus tard ; ADR-0010) : le premier plafond strictement le plus petit, dans l'ordre fin du projet (`ProjectFinish`), plusieurs chemins critiques (`MultipleCriticalPaths`, R-051), plafond hérité `C(S)` puis `K(S)` (`SummaryCap`, avec la récapitulative qui a posé le plafond, éventuellement un ancêtre), liens sortants FS/FF puis SS/SF dans l'ordre des arcs (`Successor`, avec le successeur d'origine et le type) ; à égalité, le premier garde sa cause ; récapitulative automatique : `Rollup` (R-060) ; repli de R-069 : `Fallback` (N1 → C et D `ProjectFinish`, A et B `Successor { C, FS }` ; S {A, B} FS → Y → A et B `SummaryCap { S }`). Exemples : N1 : C mer 08:00 – 17:00 ; B plafond mer 08:00 → lun 08:00 – mar 17:00 ; A mar 08:00 – 17:00. Calendriers mixtes avec avance : début ven 23/10 08:00 ; P (24 heures) 960 min : ven 08:00 → sam 24/10 00:00 ; Q (Standard) 1 j FS −60 min : `add_std(sam 00:00, −60)` = ven 16:00 → Q ven 16:00 – lun 26/10 16:00 = fin du projet ; retour : `h(ven 16:00)` = `next_std(ven 17:00)` = lun 08:00 → LF(P) = lun 08:00, TF(P) = 3 360 (56 h) ; le miroir naïf `add(y, −L)` donnerait LF ven 17:00 et TF −420. FF : A 2 j, B FF 1 j, D 3 j seul → TF(A) = 480. SS : A 1 j, B SS+4 h 1 j tâche finale → plafond de début lun 08:00, A lun 08:00 – 17:00, TF 0 ; avec D 3 j seul → A mar 13:00 – mer 12:00, TF(A) = TF(B) = 720. SF : A 2 j, B SF+3 j 1 j, D 4 j seul → A mar 08:00 – mer 17:00, TF 480. Décalage écoulé : début ven 23/10, A 1 j, B FS+2 ej (lun 26/10) → `h(lun 08:00)` = sam 24/10 08:00 → A ven 08:00 – 17:00, TF 0. FF écoulé vers une feuille ouvrée : A 1 j, B FF+6 em 1 j : borne lun 17:06, B lun 08:00 – 17:00 ; `LF'(B)` = mar 08:00, `h` = mar 07:54 → TF(A) = FS(A) = 0 (sans le report : −6). Plafond de début hors temps ouvré : A 1 j, B 1 j SS+600 em de A (mar 08:00 – 17:00), C 4 j seule (fin du projet jeu 22/10 17:00) : `h(LS_B = jeu 08:00)` = mer 22:00 → dernier début plaçable mer 16:59 → LS(A) = mer 16:59, TF 1 439, FS 479 (avec `forward_span(mer 22:00)`, A commencerait jeu 08:00 et B passerait au ven). Même défaut entre calendriers : A (Standard) 1 j, B (24 heures) 2 400 min SS+0, C 5 j : LS(B) = jeu 01:00 → LS(A) = mer 16:59 (et non jeu 08:00, qui pousserait la fin du projet au sam 24/10 00:00). Avance vers une durée écoulée : A 2 j, B 2 000 em FS −1 j de A (mar 08:00 – mer 17:20), C 1 j FS de B (jeu) : LS(B) = mar 22:40, ramené à mar 16:59 → `h` = mer 16:59 → LF(A) = mer 16:59, TF 479 (l'adjoint en temps ouvré donnait mer 17:00 : B finirait jeu 17:20 et C passerait au ven). Avance vers un jalon : A 1 j, M jalon FS −60 min (lun 16:00), E 1 440 em FS de M, C 3 j seule : LS(M) = mar 17:00 → `h` = mer 08:59, TF(A) = 539 (et non mer 09:00, qui placerait M à mer 08:00). FF vers un jalon : A 480 em, M jalon FF de A, E 1 440 em FS de M, C 3 j : LF(M) = mar 17:00 → `LF'(M)` = mer 07:59 → LF(A) = mer 07:59, TF 2 399.
- **Source** : S22 ; définitions des marges (S16, S17 : retard possible sans décaler la fin du projet ni un successeur) ; adjoint, report de l'ancre FF/SF, dernier début plaçable, adjoint en brut : hypothèse (aucun décalage dans le corpus). Écarts à la spécification J2 : le report `LF'_s` (trouvé par les propriétés aléatoires, marge −6 sans contrainte) ; dernier début plaçable, adjoint en brut et ancre d'un jalon (relecture J2 : dates au plus tard infaisables d'une minute à un jour).
- **Statut** : `vérifié contre MS Project` pour un décalage nul, où `h` est l'identité (corpus A1 UID 4, 6, 7 ; A2 UID 1, 2, 3 ; A4 UID 2, 3, 4 : 63 cas, FS seulement) ; transfert adjoint avec décalage, liens SS, FF et SF, report de l'ancre FF/SF, conversions en brut : `hypothèse`.
- **Tests** : `tests/rules/r047_backward_transfer.rs` (`r047_*`, un cas par type de lien, cas de la relecture J2 : `r047_start_cap_*`, `r047_lead_*`, `r047_finish_link_into_milestone_anchor` ; cause de la fin au plus tard : `r047_late_driver_*`) ; `crates/planrs-core/tests/schedule_properties.rs` (P13 : TF ≥ FS ≥ 0, (g) feuilles placées à leurs dates au plus tard sans lien violé ni fin du projet décalée, (h) retard de TF ou de FS sans effet ; P15 : oracle en temps ouvré, réseau sans jalon).

## R-048 — Marge totale
- **Formulation** : `TF = min(M.between(ES, LS), M.between(EF, LF))`, `M` = échelle de marge : calendrier effectif de la tâche, temps civil pour une durée écoulée (R-067). Exemples : N1 : A 480, B 0, C 0, D 960. A (24 heures) 1 j = lun 08:00 – 16:00 et B 2 j (Standard) : LF(A) = mar 17:00, LS(A) = mar 09:00, TF(A) = 1 500 en temps ouvré de 24 heures.
- **Source** : support.microsoft.com, « Total Slack (task field) » (S16), « Start Slack », « Finish Slack » (S18) ; Q5 ; dixièmes de minute dans MSPDI : O2.
- **Statut** : `vérifié contre MS Project` (corpus A1, A2, A4 : 91 cas identiques en dixièmes de minute, par exemple A1 UID 0 à 5 ; A2 UID 0 à 5).
- **Tests** : `tests/rules/r048_total_slack.rs` (`r048_*`).

## R-049 — Marge libre
- **Formulation** : `FS = max(0, min(TF, écarts))` (amendée en J3, R-101 : bornée à 0) ; pour chaque lien sortant effectif (propre, ou hérité d'une récapitulative ancêtre), écart = `M.between(ancre propre, h(ancre au plus tôt du successeur))`, ancre propre EF pour FS/FF, ES pour SS/SF ; ancre du successeur ES pour FS/SS, `EF'` pour FF/SF (report de R-047, jalon compris) ; `h` en brut sous les mêmes conditions qu'en R-047 ; pour SS/SF, la borne d'une feuille est ramenée à son dernier début plaçable (`latest_start`, R-047) ; cible récapitulative : son début agrégé. Un lien SS/SF hérité ne compte que pour une tâche de même début que la récapitulative (comme son plafond, R-057). Sans lien : FS = max(0, TF). Sans contrainte ni échéance, la borne à 0 est sans effet : FS ≥ 0 est une propriété vérifiée sans troncature (P13) ; avec une marge négative, FS = 0 (Q10, R-101). Exemples : N1 : A = `between(lun 17:00, mer 08:00)` = 480 ; D = 960. N2 (A 1 j → E 2 h → C ; B 2 j → C ; C 1 j) : E mar 08:00 – 10:00, C mer 08:00 – 17:00 ; E TF 360, FS 360 ; A TF 360, FS 0. A 1 j, B FS+1 j : `h(mer 08:00)` = mar 08:00 → FS(A) = 0. A 1 j, B 1 j SS+600 em de A (mar 08:00) : `h(mar 08:00)` = lun 22:00 → dernier début plaçable lun 16:59 → FS(A) = 479 (480 retarderait B au mer).
- **Source** : support.microsoft.com, « Free Slack (task field) » (S17) ; formule : hypothèse ; observé O2 (A2 UID 6, 14 : FS 0, TF > 0).
- **Statut** : `vérifié contre MS Project` pour des liens FS sans décalage sur un seul calendrier (corpus A1, A2, A4 : 91 cas, dont O2, A2 UID 6 et 14) ; divergence connue entre calendriers (O11, A4).
- **Tests** : `tests/rules/r049_free_slack.rs` (`r049_*`) ; borne à 0 avec une marge négative : `tests/rules/r101_negative_slack.rs` ; `tests/rules/r047_backward_transfer.rs` (`r047_start_cap_off_working_time_elapsed_lag` : FS 479) ; propriétés P13 ((h) : retard de FS sans effet) et P15 (`crates/planrs-core/tests/schedule_properties.rs`).

## R-050 — Critique
- **Formulation** : une tâche est critique si `TF ≤ critical_slack_limit_days × minutes_per_day` (option `CriticalSlackLimit`, 0 par défaut) ; une marge négative (contrainte, échéance, R-101) est critique. Exemples : N2, seuil 0 : B et C critiques ; seuil 1 j (480) : A, E, B, C critiques.
- **Source** : support.microsoft.com, « Critical fields » (S19), « Manage your project's critical path » (S20) ; schéma (S2) ; Q5 (seuil en durée : 1 j = 8 h).
- **Statut** : `vérifié contre MS Project` au seuil 0 (corpus A1, A2, A4 : 91 cas, `CriticalSlackLimit` 0 partout) ; seuil > 0 : `hypothèse`.
- **Tests** : `tests/rules/r050_critical_threshold.rs` (`r050_*`) ; marge négative critique : `tests/rules/r101_negative_slack.rs`, `tests/rules/r095_start_no_later_than.rs`.

## R-051 — Plusieurs chemins critiques
- **Formulation** : si l'option est active, une feuille sans successeur effectif (lien non coupé vers une tâche active et planifiée, propre ou hérité d'une récapitulative ancêtre) a aussi le plafond `LF = EF`. Un lien ignoré parce que sa cible n'a pas de dates (récapitulative vide, tâche non planifiable, `link.excluded_endpoint` ; lien vers une récapitulative non appliqué, R-069) ne compte pas. Exemples : A 1 j → B 1 j (FS), C 3 j seul. Par défaut : TF(A) = TF(B) = 480. Option active : TF(A) = TF(B) = 0 ; A, B, C critiques. X 1 j FS → S {I inactive} (vide), Y 3 j seule, option active : TF(X) = 0, X critique.
- **Source** : S20 ; « par tâche finale » : hypothèse. Hypothèse concurrente consignée : « par réseau indépendant » (LF = max EF de la composante faiblement connexe). Aucun fichier du corpus n'active l'option.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r051_multiple_critical_paths.rs` (`r051_*`).

## R-052 — Hiérarchie
- **Formulation** : le parent d'une tâche est la dernière tâche non vide qui la précède avec un niveau strictement inférieur (sauts de niveau tolérés) ; une tâche est récapitulative si la tâche non vide suivante a un niveau supérieur ; lignes vides ignorées ; le drapeau MSPDI `Summary` n'est pas lu. Exemple : ID 1 (niv. 1), 2 (niv. 2), 3 (vide), 4 (niv. 2), 5 (niv. 3), 6 (niv. 1) : 2 et 4 enfants de 1, 5 enfant de 4, 1 et 4 récapitulatives, 6 de premier niveau ; niveau 1 puis niveau 3 : la seconde est enfant de la première.
- **Source** : learn.microsoft.com, schéma MSPDI (S4, `OutlineLevel`) ; hypothèse.
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 1 à 6 ; A4 UID 2, 3, 4 : 71 cas sous une récapitulative).
- **Tests** : `tests/rules/r052_outline_hierarchy.rs` (`r052_*`).

## R-053 — Dates d'une récapitulative
- **Formulation** : début = min des ES, fin = max des EF de ses enfants actifs planifiés (manuels compris, inactifs exclus) ; aucun enfant planifié → `Excluded { EmptySummary }`, et les liens qui la touchent sont ignorés (`link.excluded_endpoint`). En J2, dates planifiées = dates au plus tôt. Exemples : S {A 2 j ; B FS de A 1 j} → lun 19/10 08:00 – mer 21/10 17:00 ; S {A 1 j ; M manuelle jeu 22/10 – ven 23/10} → lun – ven ; S {A 1 j, I inactive 3 j} → lun 08:00 – 17:00 ; S {I inactive} → `EmptySummary`.
- **Source** : support.microsoft.com, « Indent or outdent tasks… » (S23), « How Project schedules tasks: Behind the scenes » (S8) ; observé O3 (A1 UID 108) ; inactifs : S25.
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 1, 2, 3, 5, 17, 18 : 17 récapitulatives identiques) ; récapitulative vide et sous-tâches manuelles : `hypothèse`.
- **Tests** : `tests/rules/r053_summary_dates.rs` (`r053_*`).

## R-054 — Durée d'une récapitulative
- **Formulation** : temps ouvré entre son début et sa fin sur le calendrier de la récapitulative, sinon du projet ; jamais la somme des durées des enfants. Exemples : S {A 1 j ; B FS+2 j de A 1 j} : B jeu 22/10 → S lun 08:00 – jeu 17:00 = 1 920 ; début ven 23/10 08:00, S {A 1 j ; B FS 1 j} → ven 08:00 – lun 26/10 17:00 = 960 ; S (24 heures) {A 1 j Standard, lun 08:00 – 17:00} → 540.
- **Source** : Microsoft Q&A 5110117 (Q4) ; S8.
- **Statut** : `vérifié contre MS Project` sur un calendrier de base (corpus A1 UID 1, 2, 3, 5, 17, 18 : 17 récapitulatives) ; divergence sur les 11 récapitulatives de A4 et son UID 0 : durée trop longue de 120 min, planrs − MS Project = +120 (calendrier du projet dérivé, O13).
- **Tests** : `tests/rules/r054_summary_duration.rs` (`r054_*`).

## R-055 — % achevé d'une récapitulative
- **Formulation** : `round_div(Σ réel × 10 000, Σ durée)` (centièmes de %, R-001) sur les feuilles actives planifiées descendantes, récapitulatives intermédiaires exclues ; réel = `percent_of(durée, % achevé)` (R-010) ; Σ durée nulle : moyenne arrondie des % des feuilles ; aucune feuille : 0. Une feuille garde son % saisi. Exemples : A 960 à 50 %, B 480 à 0 %, C 240 à 100 % → 720 / 1 680 → 4 286 ; S1 {S2 {A ; B FS+2 j de A}, C} → S2 3 333, S1 4 286 (une pondération par la durée de S2, 2 400, donnerait 3 939) ; deux jalons à 100 % et 0 % → 5 000.
- **Source** : Microsoft Q&A 5110117 (Q4) ; support.microsoft.com, « Percent (%) Complete fields », « Actual Duration » (S26) ; cas limites et arrondi : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r055_summary_percent_complete.rs` (`r055_*`).

## R-056 — Lien FS ou SS vers une récapitulative (porte)
- **Formulation** : la borne du lien s'applique comme plancher de début à toutes les sous-tâches, à tout niveau (porte de la récapitulative, instant non recalé, héritée par les portes des sous-récapitulatives) ; une sous-tâche garde ses propres liens, la plus grande borne gagne (R-044) ; pilote `SummaryGate` de la récapitulative la plus proche. Exemples : X 2 j (lun – mar) FS → S {A 1 j, B 1 j} → A, B mer 08:00 – 17:00, pilote `SummaryGate(S)` ; X SS+1 j → S : borne lun 17:00 → A, B mar 08:00 – 17:00 ; X FS → S1 {S2 {A}} → A mer 08:00.
- **Source** : learn.microsoft.com, « Tasks aren't scheduled as expected in Microsoft Project » (S24) ; observé O6 (A6 UID 9 : sous-tâches à partir du 27/11/2024 08:00).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r056_summary_predecessor_gates_subtasks.rs` (`r056_*`).

## R-057 — Lien sortant d'une récapitulative
- **Formulation** : FS et FF lisent la fin agrégée ; au retour, leur plafond de fin (`h`, R-047) s'applique à toutes les feuilles de la récapitulative, à tout niveau. SS et SF lisent le début agrégé ; au retour, leur plafond de début s'applique aux seules feuilles dont l'ES égale celui de la récapitulative (ADR-0009 A11), à travers les sous-récapitulatives de même début. Le décalage en % se calcule sur la durée R-054. La marge libre d'une feuille compte les liens hérités sous la même restriction (R-049). Exemples : S {A 1 j, B 2 j} FS → Y 1 j : Y mer 08:00 – 17:00 ; A LF mar 17:00, TF 480, FS 480 ; B TF 0. S {A 1 j ; B FS de A 1 j} SS+1 j → Y 1 j : Y mar 08:00 – 17:00 ; plafond lun 08:00 sur A seule → TF(A) = TF(B) = TF(Y) = 0 (sur toutes les feuilles : TF(B) = −480 sans contrainte). S {A 1 j, B 2 j} FS+50 % → Y : 50 % de 960 → Y jeu 22/10 08:00. S {A 1 j, B 2 j} FF → Y 1 j : Y mar 08:00 – 17:00, TF(A) = 480.
- **Source** : S24 ; observé O6 (A6 UID 11 et 25) ; restriction SS/SF : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r057_summary_successor.rs` (`r057_*`).

## R-058 — Lien FF ou SF vers une récapitulative
- **Formulation** : déplié en une borne de fin sur chaque feuille active descendante, à tout niveau (conversion R-039, R-040), avec l'avertissement `link.finish_to_summary`. Exemples : X 3 j (lun – mer) FF → S {A 1 j, B 1 j} → A, B mer 08:00 – 17:00 ; X 1 j SF+2 j → S1 {S2 {A 1 j}, B 1 j} → A, B mar 08:00 – 17:00.
- **Source** : hypothèse (MS Project refuse peut-être ce lien à la saisie).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r058_finish_link_into_summary.rs` (`r058_*`).

## R-059 — Lien entre une récapitulative et sa descendante
- **Formulation** : ignoré, dans les deux sens, avec `link.summary_descendant` (erreur). Exemple : S {A 1 j}, lien A FS → S : ignoré, A lun 08:00 – 17:00 ; lien S FS → A : ignoré aussi.
- **Source** : Microsoft Q&A 5029993 (Q7).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r059_summary_descendant_link.rs` (`r059_*`).

## R-060 — Dates au plus tard, marges et critique d'une récapitulative
- **Formulation** : LS = min des LS, LF = max des LF de ses enfants actifs ; TF = `min(between(ES, LS), between(EF, LF))` sur ces dates agrégées et le calendrier de la récapitulative, sinon du projet ; marge libre par R-049 (sans lien sortant propre ni hérité : FS = TF) ; critique par R-050 seulement, jamais « parce qu'un enfant l'est ». Exemples : P 1 j (lun) ; S {A 1 j sans lien, B 1 j FS de P} ; Q 1 j FS de B → B mar, Q mer, fin du projet mer 17:00 ; A TF 960 ; B TF 0, critique ; S : ES lun 08:00, EF mar 17:00, LS mar 08:00, LF mer 17:00 → TF(S) = FS(S) = 480, non critique malgré B (critique au seuil 1 j). S {A 1 j, B 2 j} FS → Y ; Z 4 j seule : TF(S) = 480, FS(S) = 0 (écart à Y).
- **Source** : observé O3 (A1 UID 108 : TF 105 600 dixièmes, `Critical` 0 alors que l'enfant UID 110 est critique), O4 (A1 UID 17, 18, 53, 54, 105 : FS = TF sans lien sortant), O5 (A6 UID 10, 12, 14 : FS avec lien sortant) ; reste : hypothèse.
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 1, 2, 3, 5, 17, 18 : 17 récapitulatives, dont O3, UID 108, et O4, UID 17, 18, 53, 54, 105) ; récapitulative avec lien sortant (O5) : `hypothèse` (A6 diverge pour d'autres causes).
- **Tests** : `tests/rules/r060_summary_slack_and_critical.rs` (`r060_*`).

## R-061 — Jalon et durée nulle dans le réseau
- **Formulation** : une tâche de durée nulle est placée par R-031 dans les deux passes ; l'indicateur `milestone` n'influe pas, seule la durée compte. Exemples : A 1 j ; M FS 0 → lun 17:00 ; N FS de M 1 j → mar 08:00 – 17:00 ; au plus tard, M = `backward_span(mar 08:00, 0)` = mar 08:00, TF 0 ; A LF lun 17:00. Projet commençant mar 20/10 08:00, jalon sans lien, successeur LS mar 08:00 → LF du jalon mar 08:00 (O1). Tâche d'un jour marquée jalon : planifiée comme un jour.
- **Source** : observé O1 (A2 UID 1) ; A6 UID 4 et 38 (drapeau sur une tâche d'un jour).
- **Statut** : `vérifié contre MS Project` (corpus A2 UID 1 et 17, A4 UID 41 ; O1).
- **Tests** : `tests/rules/r061_milestone_in_network.rs` (`r061_*`).

## R-062 — Tâche inactive
- **Formulation** : `Excluded { Inactive }`, ses descendants `Excluded { InactiveAncestor }` ; elle est placée pour l'affichage (`display`) par ses prédécesseurs actifs, ses prédécesseurs inactifs (lien entre deux tâches inactives, par exemple dans une récapitulative inactive) et la porte de son parent, comme une tâche automatique ; ses liens vers une tâche active sont ignorés (`link.inactive`), sans pont entre ses prédécesseurs et ses successeurs ; elle reste hors des agrégats actifs, de la fin du projet, de la passe arrière et de la marge libre de ses prédécesseurs. Une récapitulative inactive a pour `display` l'agrégat de ses descendants. Exemples : A 2 j → I (inactive, 1 j) → B 1 j : I `display` mer 08:00 – 17:00 ; B lun 08:00 – 17:00 ; fin du projet mar 17:00 ; TF(A) = 0 ; TF(B) = 480 ; `link.inactive { I → B }`. S inactive {A 1 j} : S et A affichées lun 08:00 – 17:00. S inactive {A 1 j, B 1 j FS de A} : B affichée mar 08:00 – 17:00, S lun 08:00 – mar 17:00, sans `link.inactive`.
- **Source** : support.microsoft.com, « Inactivate a task » (S25) ; affichage et absence de pont : hypothèse. Relecture J2 : les liens entre deux tâches inactives étaient ignorés, si bien que B ci-dessus s'affichait lun comme A (ADR-0009 A9 : « placée par ses prédécesseurs ») ; ils servent désormais à l'affichage, en vue de l'écriture MSPDI (J3).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r062_inactive_task.rs` (`r062_*`).

## R-063 — Tâche manuelle (feuille)
- **Formulation** : les dates saisies sont conservées sans recalage ; une seule saisie : l'autre par `P.add` avec la durée saisie, sinon la durée de la tâche (`fin = add(début, d)`, `début = add(fin, −d)`) ; deux saisies : durée utilisée = `P.between(début, fin)` ; fin < début : `task.manual_dates_inverted` (erreur) et fin ramenée au début ; ni liens entrants ni porte ne la déplacent (`link.manual_not_honored` pour chaque lien dont la borne est violée : début en brut, fin en temps ouvré) ; elle pilote ses successeurs ; ses dates au plus tard, marges et criticité se calculent comme pour une feuille (informatives) ; elle compte dans les agrégats et la fin du projet ; sans aucune date : `Excluded { ManualUndated }` (`task.manual_undated`) et ses liens ignorés (`link.excluded_endpoint`). Exemples : M manuelle mer 21/10 08:00 – jeu 22/10 17:00 (durée utilisée 960) ; B FS de M 1 j → ven 23/10 08:00 – 17:00. A 2 j FS → M manuelle lun 08:00 – 17:00 : M inchangée, avertissement, TF(M) = 480. M manuelle, début mer 08:00 seul, 2 j → fin jeu 17:00 ; fin jeu 17:00 seule → début mer 08:00.
- **Source** : S8 ; observé O9 (A7) ; reste : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r063_manual_task.rs` (`r063_*`) ; propriété P13 (e) (point fixe : feuilles passées en manuelles à leurs dates au plus tôt).

## R-064 — Récapitulative manuelle
- **Formulation** : une récapitulative manuelle active avec au moins une date saisie garde ses dates comme une feuille manuelle (R-063, sur le calendrier de la récapitulative, sinon du projet ; durée utilisée = durée saisie, sinon temps ouvré entre les deux dates) ; ses successeurs lisent ces dates, y compris pour un décalage en % ; elle entre dans l'agrégat de son parent et dans la fin du projet (R-045). Ses enfants sont planifiés normalement : la porte de ses liens FS/SS entrants s'applique à eux (S24), mais ils n'héritent ni de ses liens sortants ni de ceux de ses ancêtres (ses dates ne dépendent pas d'eux), en passe arrière comme pour la marge libre et R-051. Ses dates au plus tard, sa marge et sa criticité se calculent comme pour une feuille manuelle. Sans date saisie : récapitulative automatique. Inactive : traitée comme une récapitulative inactive automatique (R-062). Exemples : S manuelle mar 20/10 08:00 – ven 23/10 17:00 {A 1 j} : A lun 08:00 – 17:00, S reste mar – ven (durée 1 920) ; Y FS de S 1 j → lun 26/10 08:00 – 17:00 ; S TF 0 ; A TF 2 400 (n'hérite pas de S → Y). X 2 j FS → S manuelle lun {A 1 j} → A mer 08:00, S inchangée. P {S manuelle mar – ven {A}} → P mar – ven.
- **Source** : observé O8 (A7 UID 1, 5, 6 : dates saisies différentes du min/max des enfants) ; dates au plus tard, héritage des liens : hypothèse (A7 UID 6 : LS ≠ min LS des enfants, non comparé).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r064_manual_summary.rs` (`r064_*`).

## R-065 — Cycles
- **Formulation** : après Kahn, les nœuds restants sont découpés en composantes fortement connexes (Tarjan itératif) ; dans chaque composante d'au moins deux nœuds, les liens internes qui remontent la liste (`from ≥ to` dans la numérotation canonique) sont coupés, puis l'ordre est recalculé. Un `link.cycle` (erreur) par composante, avec les UID triés ; un `link.ignored_in_cycle` (avertissement) par lien coupé. Jamais de panique. Exemple : UID = ID, A (1), B (2), C (3), X (4) ; liens A → B, B → A, B → C, X → A (FS, 1 j chacune) : B → A coupé ; X lun, A mar, B mer, C jeu ; `link.cycle { [1, 2] }`, `link.ignored_in_cycle { 2 → 1 }`.
- **Source** : hypothèse (MS Project refuse les relations circulaires à la saisie).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r065_link_cycle.rs` (`r065_*`) ; propriété P17 (chaos : sur un réseau plat, les composantes fortement connexes sont exactement les cycles signalés).

## R-066 — Calendrier d'une tâche et replis
- **Formulation** : calendrier propre de la tâche, sinon celui du projet ; calendrier propre inutilisable (inconnu, en échec, sans temps ouvré) → calendrier du projet et `task.calendar_fallback` (avertissement) ; calendrier du projet inutilisable → calendrier Standard intégré, compilé sous l'UID du calendrier du projet, et `project.calendar_fallback` (erreur). Exemples : A, calendrier 99 inexistant, 1 j → lun 08:00 – 17:00, avertissement ; calendrier du projet sans temps ouvré → Standard, erreur, A lun 08:00 – 17:00.
- **Source** : S8 ; replis : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r066_task_calendar_fallback.rs` (`r066_*`).

## R-067 — Tâche à durée écoulée
- **Formulation** : placement et marges en temps civil (R-028) ; ses décalages restent régis par R-041. Exemples : A 57 eh (3 420) lun 08:00 → mer 21/10 17:00 ; B FS 1 j → jeu 08:00 – 17:00 ; TF(A) = 900 (15 h) : non critique au seuil 1 j, critique au seuil 2 j. A de 105 eh finissant ven 23/10 17:00, B lun 26/10 08:00 → TF(A) = 3 780 (63 h).
- **Source** : Microsoft Q&A f54ec732 (Q5).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r067_elapsed_duration_task.rs` (`r067_*`).

## R-068 — Récapitulative du projet (UID 0)
- **Formulation** : `Schedule::project_summary` (aussi lue par `scheduled(UID 0)`) agrège les tâches actives planifiées de premier niveau : début = min ES, fin = max EF (R-053), dates au plus tard min LS / max LF (R-060), durée et marges sur le calendrier du projet (R-054, R-060), marge libre = marge totale (aucun lien), critique par le seuil (R-050), % achevé sur toutes les feuilles actives planifiées (R-055), pilote `Rollup`. Absente si aucune tâche n'est planifiée. Elle n'apparaît pas dans `Schedule::tasks` ni dans `critical()`. Exemples : N1 → lun 19/10 08:00 – mer 21/10 17:00, durée 1 440, TF 0, critique ; S {A 960 à 50 %, B 480}, C 240 à 100 %, I inactive → fin mar 20/10 17:00, % 4 286.
- **Source** : S8, S23 ; observé A1 à A7 (UID 0, `OutlineLevel` 0).
- **Statut** : `vérifié contre MS Project` (corpus A1 UID 0, A2 UID 0). Divergences connues : A4 UID 0 (durée trop longue de 120 min, calendrier du projet dérivé, O13, comme R-054) ; A6 et A7 UID 0 (calendriers de ressource, contraintes, avancement : causes J3 à J5).
- **Tests** : `tests/rules/r068_project_summary.rs` (`r068_*`).

## R-069 — Conversion impossible
- **Formulation** : une conversion qui échoue en passe avant (hors domaine, calendrier sans temps ouvré) exclut la tâche (`Excluded { Unschedulable }`, `task.unschedulable { pass: Forward, cause }`, `cause` = code de l'erreur) ; ses liens sortants sont ignorés (`link.excluded_endpoint`) et ses successeurs, planifiés sans eux, ont `degraded = true` (transitif). Un lien vers une récapitulative dont la borne ne se calcule pas (décalage hors domaine) est ignoré avec `link.unschedulable { cause }` (erreur), sans `task.unschedulable` : la porte garde ses autres bornes (début du projet, porte du parent, autres liens), la récapitulative et ses sous-tâches restent planifiées et `degraded` ; le lien ne plafonne pas son prédécesseur et ne compte ni pour sa marge libre ni pour R-051. En passe arrière : `late = early`, `degraded`, `task.unschedulable { pass: Backward }`. Une tâche dont un successeur (ou la porte d'une récapitulative successeur, ou ses sous-tâches) est dégradé ou non planifiable a des dates au plus tard et des marges de repli : `degraded = true`, transmis vers l'amont (prédécesseurs, sous-tâches qui héritent des liens d'une récapitulative, récapitulative qui agrège ces dates). Un invariant interne violé donne `schedule.internal`. Exemples : début mer 31/12/2149 08:00 ; A 2 j → `calendar.out_of_domain` → non planifiable ; B FS de A 1 j → mer 08:00 – 17:00, dégradée ; C FS de B, dégradée. Début lun 29/12/2149 08:00 ; X 1 j FS+4 800 → S {A 1 j}, Y 1 j FS → S : `link.unschedulable { X → S }`, A mar 30/12 08:00 (porte de Y), S et A planifiées, dégradées.
- **Source** : hypothèse (règle 10). Écart à la spécification J2 (§ 2.3), relecture J2 : une porte en échec émettait `task.unschedulable` alors que la récapitulative restait planifiée ; c'est désormais le lien qui est signalé et ignoré.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r069_unschedulable_task.rs` (`r069_*`) ; propriété P17 (chaos : dates près de 1984 et 2149, décalages géants ; régressions rejouées par `p17_regressions`).

Planification à rebours (`FromFinish`) : toutes les tâches non vides sont exclues (`UnsupportedAnchor`) avec `schedule.from_finish_unsupported` (J3, ADR-0009 A13) ; aucune règle de calcul n'est encore posée.

### Moteur : contraintes, échéances, planification à rebours (R-090 à R-112)

Spécification J3 et ADR-0010. Notations de R-035 à R-069 ; en plus : `c` = date saisie d'une contrainte (brute) ; `A` = son ancrage normalisé sur `P(t)` (R-090), `s* = A.start` pour une contrainte de début (MSO, SNET, SNLT), `f* = A.finish` pour une contrainte de fin (MFO, FNET, FNLT). Une table dérive, une fois par feuille automatique active, l'effet de sa contrainte selon le type, le sens de planification et l'option « les tâches respectent toujours leurs dates de contrainte » (R-099) : aucun, opposé (ALAP depuis le début), plancher, plafond, fenêtre, serrage, épingle ; les passes ne lisent que cette règle. Depuis le début : SNET et FNET sont des planchers (R-093, R-094) ; SNLT et FNLT des plafonds de la passe arrière, serrés dans la passe avant si l'option est active (R-095, R-096, R-099) ; MSO et MFO des épingles, ou des fenêtres si l'option est inactive (R-097, R-098) ; le conflit avec un lien est signalé (R-100) et la marge négative conservée (R-101). Sauf mention contraire : option active, depuis le début. Sources ajoutées : S27 à S31, S34, Q10, Q13 ; observations O14, O15, O17.

## R-090 — Normalisation des dates de contrainte
- **Formulation** : sur l'échelle de placement `P(t)` de la feuille (calendrier effectif, temps civil pour une durée écoulée), durée `d`. Contrainte de début (MSO, SNET, SNLT) : `A = P.forward_span(c, d)`, `s* = A.start` (S9 : un début saisi à 07:00 sur un calendrier à 08:00 commence à 08:00 ; jalon : R-031 en passe avant). Contrainte de fin (MFO, FNET, FNLT) : `f* = P.backward_span(c, 0).finish` (R-031 en passe arrière : `c` gardé s'il commence une minute ouvrée ou tombe dans une plage, sinon l'instant ouvré précédent), puis `A = (P.add(f*, −d), f*)` (R-022). Temps écoulé : `A = (c, c + d)` ou `(c − d, c)`, aucun recalage. Comparaisons aux autres bornes : un début en brut (`s*` commence une minute ouvrée), une fin en temps ouvré (`P.between(f*, x) > 0`), car `f*` peut être le début d'une plage (O14 : ven 18:00 vaut sam 18:00 en temps ouvré). Une conversion impossible rend la feuille non planifiable dans la passe qui utilise l'ancrage (R-069). Aucune nouvelle primitive de calendrier (ADR-0010). Exemples : SNET sam 24/10 10:00, 1 j → lun 26/10 08:00 – 17:00 ; SNET mer 21/10 07:00 → mer 08:00 ; FNET ven 23/10 17:00, 1 j → ven 08:00 – 17:00 ; FNET lun 26/10 08:00, 1 j → `f*` = lun 08:00, A = ven 23/10 08:00 – lun 26/10 08:00 (R-030 donnerait ven 17:00 : cas qui discrimine) ; FNET sam 24/10 10:00 → ven 08:00 – 17:00 ; calendrier « A7 » (ven 18:00-20:00, 21:00-23:00 ; sam 09:00-13:00, 14:00-18:00) : FNET ven 18/10/2024 18:00, 8 h → sam 12/10 09:00 – ven 18/10 18:00 ; 12 h, ven 29/11 18:00 → ven 22/11 18:00 – ven 29/11 18:00 ; durée écoulée 1 ej, FNET mer 21/10 10:00 → mar 20/10 10:00 – mer 10:00.
- **Source** : learn.microsoft.com, « Definition of Microsoft Project constraints » (S27 : « EF=CD ») ; support.microsoft.com, « Change default times for entered start or finish dates » (S9) ; observé O14 (A7 UID 15 à 21, 32 : fin = date de contrainte au début d'une plage). Hypothèse concurrente : fin = date brute même en temps chômé.
- **Statut** : `hypothèse`. Côté fin reproduit par le harnais (A7 UID 15 à 21, 32 : 8 cas identiques, mesuré le 2026-10-17, parité 99/159) : passage à `vérifié contre MS Project` au relevé des planchers.
- **Tests** : `tests/rules/r090_constraint_date_normalization.rs` (`r090_*`) ; tests unitaires de `crates/planrs-core/src/schedule/constraint.rs` (table de dérivation, ancrage).

## R-091 — Le plus tôt possible (ASAP)
- **Formulation** : depuis le début, aucune borne : la tâche est placée par ses liens, la porte de son parent et le début du projet (R-036 à R-044), comme en J2. Exemple : réseau N1 de R-045, toutes ASAP : A lun, B lun – mar, C mer, D lun ; TF A 480, B 0, C 0, D 960.
- **Source** : S27 (« (Scheduled)ES=(Calculated)ES + Delay »), support.microsoft.com, « Set a task start or finish date (constraint) for a task » (S28 : défaut depuis le début).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r091_as_soon_as_possible.rs` (`r091_*`).

## R-093 — Ne pas commencer avant le (SNET)
- **Formulation** : plancher `s*` (R-090) en passe avant, comparé au début placé par les autres bornes (R-044 amendée) : si `s* >` ce début, `early = A`, pilote `Constraint { StartNoEarlierThan }` ; sinon rien ne change, pas même le pilote. Aucun effet en passe arrière depuis le début (l'option R-099 n'y change rien : un plancher ne crée pas de marge négative). Exemples : A 2 j FS → B 1 j SNET mer 21/10 13:00 → B mer 13:00 – jeu 22/10 12:00, pilote la contrainte, TF(B) 0 ; A au plus tard lun 13:00 – mer 12:00, TF(A) = FS(A) = 240. P 3 j FS → A SNET mar 20/10 08:00 → A jeu 08:00, pilote `Link { P, FS }`. Sans effet : A 1 j FS → B SNET mar 20/10 08:00 → borne du lien lun 17:00 (brute), placée mar 08:00 = `s*` → B mar 08:00, pilote `Link { A, FS }`. A 1 j SNET mar 20/10 08:00, C 4 j seule → A au plus tard jeu 08:00, TF 960.
- **Source** : S27 (« ES=CD »), S28, learn.microsoft.com, « Back to Basics: Working with task date constraints » (S30 : une date saisie à la main devient SNET).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r093_start_no_earlier_than.rs` (`r093_*`).

## R-094 — Ne pas finir avant le (FNET)
- **Formulation** : plancher de fin `f*` (R-090) en passe avant : si `P.between(fin placée, f*) > 0`, `early = A` (fin = `f*`, O14 ; et non `forward_span(A.start, d)`), pilote `Constraint { FinishNoEarlierThan }` ; sinon rien ne change. Aucun effet en passe arrière depuis le début. Exemples : A 1 j FS → B 1 j FNET jeu 22/10 17:00 → B jeu 08:00 – 17:00, TF(B) 0, TF(A) = FS(A) = 960. A 2 j FNET mar 20/10 12:00 → début candidat ven 16/10 13:00, la fin placée depuis le début du projet (mar 17:00) dépasse déjà `f*` → A lun 08:00 – mar 17:00, pilote `ProjectStart`. A 1 j FNET lun 26/10 08:00 → ven 23/10 08:00 – lun 26/10 08:00 ; B FS → lun 26/10 08:00 – 17:00. Calendrier « A7 » : FNET ven 18/10/2024 18:00, 8 h, fin du projet ven 20/12 18:00 → ES sam 12/10 09:00, EF ven 18/10 18:00, LS sam 14/12 09:00, LF sam 14/12 18:00, TF = FS = 6 480.
- **Source** : S27 (« EF=CD »), S28, S30 ; observé O14 (A7 UID 15 à 21, 32), O17 (A6 UID 7 : FNET qui ne lie pas).
- **Statut** : `hypothèse`. Reproduit par le harnais (A7 UID 15 à 21, 32 : 8 cas identiques, mesuré le 2026-10-17) : passage à `vérifié contre MS Project` au relevé des planchers.
- **Tests** : `tests/rules/r094_finish_no_earlier_than.rs` (`r094_*`).

## R-095 — Ne pas commencer après le (SNLT)
- **Formulation** : plafond `A.finish` (R-090 : `A = P.forward_span(s*, d)`, donc le plafond de fin d'un début au plus tard en `s*`) en passe arrière, cause `LateDriver::Constraint { StartNoLaterThan }` (R-047), appliqué après les liens sortants. Option R-099 active : serrage en passe avant (principale) : si le début placé par les autres bornes dépasse `s*` (brut), `early = A`, pilote `Constraint { StartNoLaterThan }`, et chaque borne violée donne `constraint.override` (R-100) ; le lien violé reste et transmet une marge négative. Option inactive : les liens l'emportent ; si `ES > s*`, `constraint.not_honored` (R-099) et la marge négative vient du plafond. Exemples, option active : P 2 j FS → A 1 j SNLT mar 20/10 08:00 → A mar 08:00 – 17:00, `constraint.override { A, SNLT, Predecessor { P, FS } }` ; fin du projet mar 17:00 ; TF(A) 0 ; P au plus tard ven 16/10 08:00 – lun 19/10 17:00, TF(P) −480, FS(P) 0 ; P et A critiques. Option inactive : A mer 08:00 – 17:00, `constraint.not_honored { A, SNLT, mar 08:00 }` ; fin mer 17:00 ; A au plus tard mar 08:00 – 17:00, TF(A) −480 ; TF(P) −480. Sans conflit : A 1 j SNLT mer 21/10 08:00, C 4 j seule → A lun, au plus tard mer 08:00, TF 960.
- **Source** : S27 (« LS=CD ») ; support.microsoft.com, « How Project schedules tasks: Behind the scenes » (S8 : « Inflexible constraints usually override any task dependencies », option « Tasks will always honor their constraint dates ») ; « Show slack in your project » (S31 : marge négative) ; Microsoft Q&A 5182424 (Q13 : l'option n'agit que sur les contraintes qui créent une marge négative).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r095_start_no_later_than.rs` (`r095_*`).

## R-096 — Ne pas finir après le (FNLT)
- **Formulation** : plafond `f*` (R-090) en passe arrière, cause `LateDriver::Constraint { FinishNoLaterThan }`. Option active : serrage en passe avant : si `P.between(f*, fin placée) > 0`, `early = A = (P.add(f*, −d), f*)`, pilote la contrainte, `constraint.override` par borne violée. Option inactive : `constraint.not_honored` si la fin au plus tôt dépasse `f*` en temps ouvré. Exemples, option active : P 1 j FS → A 2 j FNLT mar 20/10 17:00 → A lun 08:00 – mar 17:00, override `Predecessor { P, FS }` ; P au plus tard ven 16/10 08:00 – 17:00, TF(P) −480 ; TF(A) 0. Option inactive : A mar 08:00 – mer 17:00, `not_honored` ; A au plus tard lun 08:00 – mar 17:00, TF(A) −480, TF(P) −480. Sans conflit : A 2 j FNLT mer 21/10 17:00, C 4 j → A au plus tard mar 08:00 – mer 17:00, TF(A) 480 (1 440 sans la contrainte).
- **Source** : S27 (« LF=CD »), learn.microsoft.com, « Back to Basics: Working with task date constraints » (S30, exemple « Prepare lot »), Q13.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r096_finish_no_later_than.rs` (`r096_*`).

## R-097 — Doit commencer le (MSO)
- **Formulation** : option R-099 active : épingle, `early = late = A` (R-090 : `A = P.forward_span(s*, d)`), pilote et cause `Constraint { MustStartOn }`, quelles que soient les bornes ; chaque borne violée donne `constraint.override` en passe avant (R-100) ; ses dates au plus tard ne sont pas plafonnées (la violation s'exprime par la marge négative de l'amont). Option inactive : fenêtre, plancher `s*` en passe avant (comme R-093) et plafond `A.finish` en passe arrière (comme R-095) ; `constraint.not_honored` si le début au plus tôt dépasse `s*`. Exemples : A 1 j MSO mer 21/10 08:00, C 4 j seule → A mer 08:00 – 17:00 au plus tôt et au plus tard, TF 0, critique malgré une fin du projet jeu 17:00. Conflit : P 3 j FS → A MSO mar 20/10 08:00 → A mar 08:00 – 17:00, override `Predecessor { P, FS }` ; fin du projet mer 17:00 ; P au plus tard jeu 15/10 08:00 – lun 19/10 17:00, TF(P) −960, FS(P) 0. Option inactive : A jeu 22/10, `not_honored`, TF(A) = TF(P) = −960. Fenêtre sans conflit : A MSO jeu 22/10 08:00, C 5 j, option inactive → A jeu, TF 0.
- **Source** : S27 (« ES,LS,SS=CD »), S8 (« Inflexible constraints usually override any task dependencies »), S31 (exemple FS vers une tâche MSO plus tôt → marge négative).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r097_must_start_on.rs` (`r097_*`).

## R-098 — Doit finir le (MFO)
- **Formulation** : comme R-097 avec l'ancrage de fin `A = (P.add(f*, −d), f*)` (R-090) : épingle si l'option est active, fenêtre sinon (plancher de fin comme R-094, plafond `f*` comme R-096). Exemples : A 1 j MFO mer 21/10 17:00 → mer 08:00 – 17:00, TF 0. MFO jeu 22/10 08:00 → `f*` = jeu 08:00 (début d'une plage), A mer 08:00 – jeu 08:00 ; B FS → jeu 08:00 – 17:00. P 3 j FS → A 1 j MFO mar 20/10 17:00 → A mar, TF(P) −960, un override ; option inactive : A jeu, `not_honored`, TF(A) −960.
- **Source** : S27 (« EF,LF,SF=CD »), S8, S30 (exemple MFO : « Project warns us of the scheduling conflict, and then overlaps the tasks »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r098_must_finish_on.rs` (`r098_*`).

## R-099 — Option « les tâches respectent toujours leurs dates de contrainte »
- **Formulation** : `ProjectOptions::honor_constraints`, vraie par défaut (R-032). Une table dérive la règle de chaque feuille automatique active (type × sens × option, spécification J3 § 3.5) ; depuis le début : ASAP aucune, ALAP opposée, SNET et FNET plancher (indépendant de l'option), SNLT et FNLT serrage si l'option est active, plafond sinon ; MSO et MFO épingle si active, fenêtre sinon (R-097, R-098) ; à rebours : ALAP aucune, ASAP opposée, SNET et FNET serrage si active, plancher sinon, SNLT et FNLT plafond. Option active : la contrainte dure l'emporte sur les liens (`constraint.override`, R-100) ; option inactive : les liens l'emportent et `constraint.not_honored { tâche, type, date saisie }` (avertissement) signale une contrainte non tenue — c'est le seul cas où ce diagnostic apparaît ; depuis le début, il est émis après les marges pour un plafond (ou une fenêtre) dont la date au plus tôt dépasse l'ancrage (début en brut, fin en temps ouvré). Exemples : ceux de R-095 à R-098 dans les deux modes (P 2 j FS → A 1 j SNLT ou MSO mar 20/10 08:00, FNLT ou MFO mar 17:00 : active, A mar et override ; inactive, A mer et `not_honored` ; TF(P) −480 partout) ; un plancher (SNET, FNET) donne les mêmes dates et aucun diagnostic dans les deux modes.
- **Source** : S8 (option « Tasks will always honor their constraint dates »), learn.microsoft.com, « Tasks aren't scheduled as expected in Microsoft Project » (S24 : l'option « overrides the task relationship links »), schéma MSPDI (S34 : `HonorConstraints` `default="true"`), Q13 ; sens de la valeur MSPDI à l'import : O15 (R-089).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r099_honor_constraints_option.rs` (`r099_*`) ; table de dérivation : test unitaire `derive_table` de `crates/planrs-core/src/schedule/constraint.rs`.

## R-100 — Conflit contrainte/lien : signalement
- **Formulation** : quand une contrainte dure place la feuille de force (serrage, épingle), un `constraint.override { tâche, type, borne }` (avertissement, sujet la tâche) par borne violée (`OverriddenBound`), en passe principale seulement : début du projet si `A.start <` début (brut, R-105) ; porte de la récapitulative parente si `A.start < G` (`SummaryGate`) ; lien FS ou SS si `A.start <` sa borne (brut) ; lien FF ou SF si `P.between(A.finish, borne) > 0` (`Predecessor { prédécesseur d'origine, type }`) — même lecture des bornes que pour une tâche manuelle (R-063). Jamais d'échec ni de panique ; le lien violé reste dans le réseau et transmet la marge négative à l'amont (R-101). Jamais d'override en passe secondaire : les violations des dates au plus tard s'expriment par la marge négative. Les diagnostics restent triés (par tâche, type, puis borne) et dédoublonnés. Exemples : P 2 j FS → A ; Q 3 j FS → A ; A 1 j SNLT mar 20/10 08:00 → deux overrides (P puis Q), TF(P) −480, TF(Q) −960 ; S {A 1 j SNLT lun 19/10 08:00}, X 2 j FS → S → A lun 08:00 – 17:00, `override { A, SNLT, SummaryGate { S } }`, X au plus tard jeu 15/10 08:00 – ven 16/10 17:00, TF(X) −960 ; P 3 j FF → A 1 j SNLT mar 08:00 → override `Predecessor { P, FF }`. Avec une épingle (R-097) : S {A 1 j MSO lun 19/10 08:00} ; X 2 j FS → S → A lun 08:00 – 17:00, `override { A, MSO, SummaryGate { S } }` ; X au plus tard jeu 15/10 08:00 – ven 16/10 17:00, TF(X) −960.
- **Source** : S30 (« Project warns us of the scheduling conflict, and then overlaps the tasks »), S8 (« Inflexible constraints usually override any task dependencies »).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r100_constraint_conflict_diagnostics.rs` (`r100_*`) ; test unitaire `constraint_diagnostics` de `crates/planrs-core/src/diagnostic.rs` (code, sujet, JSON).

## R-101 — Marge négative, marge libre bornée
- **Formulation** : la marge totale (R-048) est signée et conservée ; une tâche est critique si `TF ≤ seuil` (R-050), marge négative comprise ; la marge libre est `FS = max(0, min(TF, écarts))` (R-049 amendée), y compris pour une récapitulative et la récapitulative du projet (`max(0, TF)`). Sans contrainte ni échéance, la borne est sans effet : `FS ≥ 0` de R-049 reste une propriété vérifiée sans troncature. Exemple : Q 1 j FS → P 3 j FS → A 1 j SNLT (ou MSO) mar 20/10 08:00 (option active) → TF(P) = TF(Q) = −1 440, FS(P) = FS(Q) = 0 ; P et Q critiques.
- **Source** : support.microsoft.com, « Total Slack (task field) » (S16 : « Negative slack … is usually caused by constraint dates or task dependencies »), S31 ; Microsoft Q&A, « MSP 2013: no negative value in free slack? » (Q10 : « Free Slack will always show a 0d value, even though the Total Slack field shows a negative value »). Rouvre l'option « troncature de la marge libre » écartée par l'ADR-0009 (ADR-0010).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r101_negative_slack.rs` (`r101_*`).

## R-105 — Contrainte dure hors du projet
- **Formulation** : option active, un serrage (SNLT, FNLT, R-095, R-096) ou une épingle (MSO, MFO, R-097, R-098) peut placer une tâche avant le début du projet : `constraint.override { ProjectStart }` (R-100) ; l'UID 0 s'étend jusqu'à elle (R-068) ; `project_start` reste l'ancre saisie ; `project_finish` est le max des fins au plus tôt (R-045), même antérieur au début. Option inactive : le plancher R-036 l'emporte, `constraint.not_honored`. Exemples : A 1 j SNLT ven 16/10 08:00, début lun 19/10 → A ven 16/10 08:00 – 17:00, override `ProjectStart`, UID 0 depuis ven 16/10 08:00 ; option inactive : A lun 19/10, au plus tard ven 16/10, TF −480, `not_honored`. Même chose avec A 1 j MSO ven 16/10 08:00.
- **Source** : S24 ; hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r105_constraint_outside_project.rs` (`r105_*`).

### Import MSPDI (R-070 à R-089)

Bloc réservé à la lecture MSPDI (`planrs-mspdi`) ; l'écriture a son propre bloc (R-120 à R-139). Les éléments du schéma sont décrits par une table unique, `planrs_mspdi::schema` (séquences pj15, classe de chaque élément, ordre observé), comparée au XSD par la propriété P27 (`tests/schema`, `cargo xtask schema fetch`). Sources : schéma `mspdi_pj15.xsd` (Project 2013 SDK, https://www.microsoft.com/en-us/download/details.aspx?id=30435, révision 2012-07-18, sha256 `1c208a9732d6ba16f677a9f2fb9d5a629f0c200346b1851998a44765c502eb85`, jamais commité) et pages learn.microsoft.com « XML Schema for the Project / Tasks / Calendars Element » (S1 à S4). Chaque valeur illisible devient un diagnostic d'import (`mspdi.*`) et reçoit la valeur par défaut : la lecture n'échoue jamais pour un champ.

## R-070 — Durée ISO 8601 → minutes
- **Formulation** : `[-]PnDTnHnMnS` ; un jour vaut 24 h ; décimales admises dans la dernière composante seulement ; secondes arrondies à la minute par R-001 (diagnostic `mspdi.rounded`) ; années et mois non nuls refusés (longueur variable), semaines refusées (hors `xsd:duration`) ; une durée négative se lit signée, et la tâche la refuse (durée 0, diagnostic). Exemples : `PT80H0M0S` → 4 800 ; `PT8H30M0S` → 510 ; `PT0H0M30S` → 1 ; `PT1.5H` → 90 ; `P2D` → 2 880 ; `P1M` → refus ; `-PT8H` → −480 (durée de tâche 0 et diagnostic).
- **Source** : learn.microsoft.com, « XML Schema for the Tasks Element » (`Duration` de type `xsd:duration`, S4) ; ADR-0005 ; arrondi : hypothèse.
- **Statut** : `vérifié contre MS Project` pour les durées en heures entières de tâches en jours (corpus A1, A2, A4 : 72 feuilles automatiques identiques) ; décimales, secondes et jours ISO : `hypothèse`.
- **Tests** : `tests/rules/r070_iso_duration.rs` (`r070_*`) ; tests unitaires de `crates/planrs-mspdi/src/scalar.rs`.

## R-071 — `DurationFormat` d'une tâche
- **Formulation** : unité, base et estimation par la table de `DurationFormat` (3/35 m, 4/36 em, 5/37 h, 6/38 eh, 7/39 j, 8/40 ej, 9/41 sem, 10/42 esem, 11/43 mois, 12/44 emois ; + 32 = « ? ») ; 21 et 53 (nul) → unité par défaut du projet (`DurationFormat` du projet, sinon jour), ouvrée ; 19, 20, 51, 52 (%) sur une durée de tâche → jour ouvré, `mspdi.invalid_value` ; code inconnu → `mspdi.unknown_code` ; `Estimated` présent l'emporte sur les codes « ? ». Les minutes viennent de `Duration` (R-070) quelle que soit l'unité. La durée d'une récapitulative automatique (déduite de la hiérarchie, R-052) est une valeur calculée : gardée dans `RecordedSchedule`, mise à 0 dans le modèle (une récapitulative manuelle garde la sienne, R-064). Exemples : `PT40H0M0S` + 39 → 2 400, jour, ouvrée, estimée ; `PT120H0M0S` + 8 → 7 200, jour, écoulée ; `PT8H0M0S` + 7 avec `Estimated 1` → estimée ; récapitulative `PT80H0M0S` + 21 → 4 800 enregistrées, jour.
- **Source** : learn.microsoft.com, « DurationFormat Element » (S3), « XML Schema for the Tasks Element » (S4) ; durée des récapitulatives hors du modèle : ADR-0004, ADR-0009.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r071_duration_format.rs` (`r071_*`), fixture `tests/fixtures/mspdi/tasks.xml`.

## R-072 — `LinkLag` en dixièmes de minute
- **Formulation** : minutes = `round_div(LinkLag, 10)` (R-001), diagnostic `mspdi.rounded` s'il y a un reste ; base et unité par `LagFormat` (absent : jour ouvré). Exemples : 4 800 + `LagFormat` 7 → 480 ouvré ; −4 800 → −480 ; 28 800 + 8 → 2 880 écoulé ; 25 → 3 et diagnostic.
- **Source** : learn.microsoft.com, « LinkLag Element » (« in tenths of a minute », S4), « LagFormat Element ».
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r072_link_lag_tenths.rs` (`r072_*`).

## R-073 — Décalage en pourcentage
- **Formulation** : pour `LagFormat` 19, 20, 51, 52, `LinkLag` vaut le pourcentage × 10, donc `SignedPercent` = `LinkLag` × 10. Diagnostic `mspdi.unsupported` (`percent_lag_encoding`, avertissement) à chaque occurrence, retiré quand un fichier réel l'aura confirmé. Exemples : 500 + 19 → 5 000 (50 %) ; −250 + 19 → −2 500.
- **Source** : hypothèse pure (aucun fichier du corpus n'a de décalage en pourcentage).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r073_percent_lag_encoding.rs` (`r073_*`).

## R-074 — Type de lien et liens ignorés
- **Formulation** : `Type` 0 FF, 1 FS, 2 SF, 3 SS ; absent → FS ; autre → lien ignoré (`mspdi.link_ignored`, `unknown_type`) ; `CrossProject 1` → ignoré (`cross_project`) ; lien sur soi → ignoré (`self_link`) ; prédécesseur inconnu (UID 0 compris) → ignoré, `mspdi.dangling_reference` ; paire en double → le premier gardé, `mspdi.duplicate` ; `PredecessorUID` absent → `mspdi.missing_required`. L'ordre MSPDI des liens est conservé ; un cycle est accepté tel quel (le moteur le traite, R-065). Exemples : un fichier du corpus (A6) a 6 liens `Type 0` → FF ; fixture : FF, SS, SF lus, cinq liens ignorés.
- **Source** : learn.microsoft.com, « PredecessorLink Element » (S4).
- **Statut** : `vérifié contre MS Project` pour `Type` 1 (FS) (corpus A1 UID 6, 7, 8 ; A2 UID 5, 6, 7 ; A4 UID 3, 4, 6) ; types 0, 2, 3 et liens ignorés : `hypothèse`.
- **Tests** : `tests/rules/r074_link_type.rs` (`r074_*`).

## R-075 — Dates sans fuseau
- **Formulation** : `AAAA-MM-JJThh:mm[:ss[.fff]]` en heure locale du projet ; un suffixe `Z` ou `±hh:mm` est ignoré (diagnostic `mspdi.inconsistent`) ; secondes tronquées (R-004, diagnostic `mspdi.rounded` si non nulles) ; `24:00:00` = lendemain 00:00 ; hors du domaine 1984-2149 → aucune date et diagnostic. Exemples : `2009-11-09T08:00:00` → lun 09/11/2009 08:00 ; `2026-08-07T18:18:20` → 18:18, `mspdi.rounded` ; `1983-12-31T08:00:00` → aucune date, diagnostic.
- **Source** : learn.microsoft.com, schéma MSPDI (`xsd:dateTime`) ; R-003, R-004.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r075_datetime_without_timezone.rs` (`r075_*`).

## R-076 — Booléens, éléments absents, défauts du schéma
- **Formulation** : booléen `1`/`true` ou `0`/`false` (espaces ignorés) ; toute autre écriture → valeur par défaut et diagnostic `mspdi.invalid_value`. Élément absent (ou vide) : valeur `default=` du schéma s'il en a une — `ScheduleFromStart`, `HonorConstraints`, `NewTasksEffortDriven`, `NewTasksEstimated` vrais ; `DefaultTaskType` = durée fixe ; `CriticalSlackLimit` = 0 ; `MultipleCriticalPaths` faux ; `Active` vrai ; `Manual`, `IsNull` faux —, sinon celle du modèle (R-032 : 480 / 2 400 / 20, jour, lundi). `CurrentDate` absent : « maintenant » injecté ; `StartDate` absent : « maintenant » et `mspdi.missing_required` ; `ScheduleFromStart` faux : ancre sur `FinishDate` et `mspdi.unsupported` (`schedule_from_finish`, J3). Facteurs hors bornes (1 à 1 440, 1 à 10 080, 1 à 31) : défaut et diagnostic. Exemples : `SaveVersion` 12 sans `Active` → tâche active ; `<Milestone>2</Milestone>` → faux, diagnostic ; `<HonorConstraints>2</HonorConstraints>` → vrai, diagnostic ; `<MinutesPerDay>0</MinutesPerDay>` → 480, diagnostic.
- **Source** : learn.microsoft.com, « XML Schema for the Project Element » et « … Tasks Element » (attributs `default=`, S2, S4) ; ADR-0008 ; défauts sans attribut `default=` : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r076_booleans_and_schema_defaults.rs` (`r076_*`), fixtures `tests/fixtures/mspdi/project_defaults.xml` et `project_options.xml`.

## R-077 — Plages horaires
- **Formulation** : `[FromTime, ToTime)` ; `ToTime` 00:00:00 = 24:00 ; secondes tronquées (`mspdi.rounded`) ; plage vide, inversée ou illisible → ignorée, `mspdi.invalid_value` ; plages triées ; plages qui se chevauchent fusionnées (`mspdi.inconsistent`) ; au-delà de 5 plages (S1), les plages contiguës sont fusionnées, puis seules les 5 premières sont gardées (`mspdi.invalid_value`). Exemples : « 24 Hours » 00:00 → 00:00 = 1 440 ; 18:00-22:00 puis 10:00-18:00 → 720 (contiguës, fusionnées par R-011) ; 08:00-12:00 et 12:00 → 08:00 → 240 et diagnostic ; 08:00-12:00 et 10:00-14:00 → 08:00-14:00 = 360 et diagnostic.
- **Source** : support.microsoft.com, « Project Information dialog box » (S6 : « 24 Hours … 12:00 A.M. to 12:00 A.M. ») ; schéma MSPDI (S1) ; fusion des chevauchements et troncature à 5 : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r077_working_times.rs` (`r077_*`), fixture `tests/fixtures/mspdi/calendars.xml`.

## R-078 — Jours de la semaine
- **Formulation** : `DayType` 1 = dimanche … 7 = samedi ; `DayWorking 0` → chômé ; `DayWorking 1` avec plages → ouvré ; `DayWorking 1` sans plage → 08:00-12:00 et 13:00-17:00 et `mspdi.invalid_value` ; `DayWorking` absent → ouvré si des plages sont données, sinon chômé ; jour absent : `Inherit` dans un calendrier dérivé, chômé et `mspdi.missing_required` dans un calendrier de base ; `DayType` inconnu → ignoré (`mspdi.unknown_code`) ; jour en double → le premier gagne (`mspdi.duplicate`). Exemples : Standard → 2 400 par semaine ; Standard plus samedi 08:00-12:00 → 2 640 ; dérivé ne définissant que le samedi → lundi hérité (480).
- **Source** : schéma MSPDI, « Calendars Element » (S1) ; jours manquants et plages par défaut : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r078_week_days.rs` (`r078_*`).

## R-079 — Calendrier de base ou dérivé
- **Formulation** : `IsBaseCalendar 1` → base (`BaseCalendarUID` ignoré) ; `0` avec `BaseCalendarUID ≥ 0` → dérivé de ce calendrier (base inexistante : `mspdi.dangling_reference`, et `calendar.base_missing` du cœur) ; `0` avec `BaseCalendarUID < 0` (−1, −65 533, −65 535) ou absent → `mspdi.dangling_reference`, puis dérivé du calendrier du projet s'il est de base et différent, sinon calendrier de base (jours manquants chômés). `IsBaseCalendar` absent : base si `BaseCalendarUID` est absent ou négatif, `mspdi.missing_required`. UID en double : le premier gagne (`mspdi.duplicate`). Exemple : calendrier « IDEA » d'un fichier du corpus (A6), `BaseCalendarUID −65533` → dérivé du calendrier du projet (UID 9) ; fixture : calendrier 5 dérivé du 1, il hérite de Noël.
- **Source** : schéma MSPDI (`BaseCalendarUID`, S1) ; traitement des valeurs négatives : hypothèse (observé A6).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r079_base_or_derived_calendar.rs` (`r079_*`).

## R-080 — Exceptions
- **Formulation** : période = [jour(`FromDate`), jour(`ToDate`)], un `ToDate` à 00:00 strictement après `FromDate` comptant pour la veille (`ToDate` absent : un seul jour, `mspdi.missing_required`) ; `Type` 1 `Daily { Period ou 1 }`, 2 `YearlyByDay { Month, MonthDay }`, 3 `YearlyByPosition { Month, MonthPosition, MonthItem }`, 4 `MonthlyByDay { Period, MonthDay }`, 5 `MonthlyByPosition { Period, MonthPosition, MonthItem }`, 6 `Weekly { Period, DaysOfWeek }`, 7 `ByDayCount`, 8 `ByWeekdayCount`, 9 `Period` (`Month` 0 = janvier ; `MonthItem` 0 jour, 1 jour ouvré, 2 week-end, 3 dimanche … 9 samedi ; `MonthPosition` 0 premier … 4 dernier ; `DaysOfWeek` masque, 1 = dimanche … 64 = samedi) ; `occurrences = Some(Occurrences)` seulement si `EnteredByOccurrences 1` ; `DayWorking` et `WorkingTimes` → réglage du jour (comme R-078 ; `DayWorking` absent sans plage → chômé) ; `Type` inconnu ou champ obligatoire manquant → exception ignorée et signalée ; chevauchement avec une exception déjà lue → ignorée, `mspdi.inconsistent`. Exemples : `Type 2`, `Month 3`, `MonthDay 4`, du 04/04/2025 au 06/04/2025 → seul le 04/04/2025 chômé ; `Type 1` du 25/12 00:00 au 25/12 23:59 → un jour ; `Type 9` du 09/11/2026 00:00 au 11/11/2026 00:00 → 9 et 10 chômés ; `Type 6`, `DaysOfWeek` 64, juin 2026 → samedis ouvrés ; `Type 1`, `Period 2`, `Occurrences 3` par occurrences → 02, 04 et 06/03/2026.
- **Source** : schéma MSPDI, « Exceptions » (S1) ; observé A6 (deux formes d'un même fériés) ; règle du `ToDate` à 00:00 : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r080_calendar_exceptions_import.rs` (`r080_*`).

## R-081 — Ancienne forme des exceptions (`WeekDay` de `DayType 0`)
- **Formulation** : ignorée si le calendrier a un élément `Exceptions` ; sinon chaque entrée devient une exception `Period` sur sa `TimePeriod` (convention de R-080), avec son `DayWorking` et ses `WorkingTimes`. La comparaison des deux formes (oracle des récurrences) est reportée à J3. Exemples : fixture sans `Exceptions`, `DayType 0` du 01/02/2004 ouvré 08-12 et 13-17 → exception ouvrée de 480 min ; fichiers A6, A7 (deux formes) → seule la forme `Exceptions` est lue.
- **Source** : schéma MSPDI (S1) ; précédence : hypothèse.
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r081_legacy_exceptions.rs` (`r081_*`).

## R-082 — Semaines de travail datées
- **Formulation** : `WorkWeeks/WorkWeek` : `TimePeriod` → période (convention de R-080), `Name`, `WeekDays/WeekDay` (`DayType` 1 à 7, réglage comme R-078 ; `WorkingTimes` accepté malgré le schéma) ; jour absent → `Inherit` (semaine par défaut du calendrier) ; chevauchement avec une semaine déjà lue → ignorée, `mspdi.inconsistent`. Exemple (fixture écrite à la main) : « Winter Break » du 22/12 au 26/12/2026 23:59, dimanche à vendredi chômés, samedi absent → 22 au 25 chômés, samedi 26 hérité (08:00-12:00).
- **Source** : schéma MSPDI (S1) ; support.microsoft.com, « Set the general working days and times for a project » (S13).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r082_work_weeks_import.rs` (`r082_*`).

## R-083 — Valeurs enregistrées, marges en dixièmes
- **Formulation** : `Start`, `Finish`, `EarlyStart`, `EarlyFinish`, `LateStart`, `LateFinish`, `TotalSlack`, `FreeSlack`, `Critical`, `Summary`, `Duration`, `LevelingDelay` de chaque tâche (UID 0 compris) vont dans `RecordedSchedule`, jamais dans le `Project` ; marges et `LevelingDelay` lues en `Tenths` sans arrondi (la comparaison du corpus se fait en dixièmes, moteur × 10) ; `LevelingDelay` ≠ 0 → `mspdi.unsupported` (`leveling_delay`, J5), jamais appliqué ; affectations comptées par `TaskUID` (inconnu : `mspdi.dangling_reference`). Exemples : 9 600 = 960 min ; 43 200 = 4 320 min ; 768 = 76,8 min, gardé tel quel.
- **Source** : learn.microsoft.com, « TotalSlack Element », « FreeSlack Element » (dixièmes de minute, S4) ; observé O2 (345 600 = 72 j).
- **Statut** : `vérifié contre MS Project` (corpus A1, A2, A4 : 91 cas dont `TotalSlack` et `FreeSlack` égaux au moteur × 10, par exemple A2 UID 2 : 345 600).
- **Tests** : `tests/rules/r083_recorded_slack_tenths.rs` (`r083_*`) ; propriété P19 « pas de fuite » dans `crates/planrs-mspdi/tests/no_leak.rs`.

## R-084 — Récapitulative du projet, ordre et identité des tâches
- **Formulation** : UID 0 (`OutlineLevel 0`) → `summary_task`, au premier niveau du modèle ; ordre des tâches = ordre du document (l'ID se déduit de la position) ; `ID` ≠ position → `mspdi.inconsistent` ; `OutlineLevel` absent → 1 ; `OutlineLevel 0` hors UID 0 → 1 et `mspdi.inconsistent` ; drapeau `Summary` contraire à la hiérarchie (R-052) → `mspdi.inconsistent` ; `IsNull 1` → ligne vide ; UID absent, négatif ou en double → tâche ignorée et signalée. Exemple : fixture de 11 lignes dont 5 `IsNull` → ID 1 à 11, 6 tâches calculées.
- **Source** : learn.microsoft.com, « Task Element » (S4) ; ADR-0008.
- **Statut** : `vérifié contre MS Project` pour la récapitulative du projet (corpus A1 UID 0, A2 UID 0) ; ordre, `ID` et lignes vides : `hypothèse`.
- **Tests** : `tests/rules/r084_project_summary_and_task_identity.rs` (`r084_*`), fixture `tests/fixtures/mspdi/null_rows.xml`.

## R-085 — Tâches manuelles
- **Formulation** : `Manual 1` → `SchedulingMode::Manual { start, finish, duration }` depuis `ManualStart`, `ManualFinish`, `ManualDuration` (lus comme `dateTime` et durée ISO malgré leur type `xsd:string`), à défaut `Start`, `Finish`, `Duration` (unité et base de la tâche) ; ce sont des entrées, gardées même pour une récapitulative. Exemples : un fichier du corpus (A7) UID 7 → manuelle 12/10/2024 09:00 – 18/10/2024 18:00 ; fixture : `ManualDuration` 23 h l'emporte sur `Duration` 16 h ; sans `Manual*`, `Start`/`Finish`/`Duration`.
- **Source** : schéma `mspdi_pj15.xsd` ; observé O9 (A7).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r085_manual_tasks_import.rs` (`r085_*`), fixture `tests/fixtures/mspdi/manual.xml`.

## R-086 — Options du chemin critique
- **Formulation** : `CriticalSlackLimit` (jours, entier ≥ 0 ; négatif, au-delà de `u32::MAX` ou illisible → 0 et `mspdi.invalid_value`) → `critical_slack_limit_days` ; `MultipleCriticalPaths` → `multiple_critical_paths` ; absents → 0 et faux (R-076). Exemple : 10 → seuil de 4 800 min avec 480 min/jour (R-050).
- **Source** : learn.microsoft.com, « XML Schema for the Project Element » (S2).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r086_cpm_options_import.rs` (`r086_*`).

## R-087 — Lecture : éléments non modélisés en `Extras`
- **Formulation** : chaque enfant d'une entité (projet, calendrier, tâche ; ressource, affectation et lignes de base avec R-088) a une classe dans la table du schéma : lu (M modélisé, M? modélisé optionnel, D déduit, C calculé par planrs et gardé dans `RecordedSchedule`, F format) ; X calculé non recalculé par planrs (coûts, valeur acquise, écarts, surutilisation, travail des ressources) : ni lu ni conservé, compté par nom (`SourceInfo::dropped_computed`), MS Project le recalcule ; P conservé : **capturé verbatim** (tranche exacte de l'entrée, de `<` à la fin de la balise fermante) dans un `ExtraEntry { origin: "mspdi", key, payload }`. Un élément absent de la table (hors schéma : champs agiles, `StartText`, `BuildNumber`, sections d'interface `Views`…) est conservé comme un P. Clé : `[Baseline#n/][ancre>]nom` ; un nom du schéma n'a pas d'ancre (sa place vient de la table) ; un nom hors schéma est ancré sur le plus proche frère précédent du schéma **toujours réécrit** (M, D, F, ou P capturé), `^` s'il n'y en a aucun ; C, X et M? ne servent jamais d'ancre (leur présence dépend des valeurs : la clé changerait à la relecture) ; les conteneurs `Calendars`, `Resources`, `Assignments` (omis s'ils sont vides) non plus, `Tasks` oui. Déclarations `xmlns:p` d'une racine préfixée : `Extras` du projet `@xmlns:p` (charge = adresse). Ordre canonique : `Extras` triés (tri stable) par emplacement d'écriture dans le profil observé (nom du schéma : son rang ; ancré : rang de l'ancre + ½ ; `^` : −½). Un jour de calendrier, une exception, une semaine datée, une plage et un lien n'ont pas d'`Extras` : leurs enfants non lus sont perdus, comptés (`SourceInfo::not_preserved`) et signalés (`mspdi.not_preserved`, info). Une capture compte pour un élément retenu (`Limits::max_retained`) ; total des octets capturés borné par `Limits::max_extras_bytes` (défaut : `max_bytes`), dépassement → `mspdi.error.too_many_retained`. `ReadOptions::keep_extras` faux : rien n'est capturé (mémoire minimale). `LevelingDelay` d'une tâche est conservé (P) et lu aussi dans `RecordedSchedule` (R-083). Exemples : tâche `… DurationFormat, FreeformDurationFormat, Work …` → `DurationFormat>FreeformDurationFormat` ; `CommitmentType, ExtendedAttribute ×2` → `CommitmentType`, `ExtendedAttribute` ×2, ordre gardé ; `Recurring, OverAllocated (X), Zzz` → `Recurring>Zzz`, `dropped_computed["OverAllocated"] = 1` ; `Hyperlink` puis `Contact` → remis dans l'ordre du profil (`Contact`, `Hyperlink`) ; projet `CurrencyCode` (P) → `CurrencyCode`, `Views` après `CalendarUID` → `CalendarUID>Views` ; `CrossProjectName` d'un lien, enfant inconnu d'un `WeekDay` → perdus, `mspdi.not_preserved`.
- **Source** : ADR-0008 (`Extras` opaques, calculés exclus), ADR-0010 (D13, D14) ; schéma `mspdi_pj15.xsd` (séquences) ; ancre et ordre canonique : hypothèse (conçus pour l'identité lecture → écriture → lecture, R-133).
- **Statut** : `hypothèse`.
- **Tests** : `tests/rules/r087_extras_capture.rs` (`r087_*`) ; tests unitaires de `crates/planrs-mspdi/src/extras.rs` (`r087_*`) et de `crates/planrs-mspdi/src/xml.rs` (`r087_*`) ; P19 étendue (`crates/planrs-mspdi/tests/no_leak.rs` : aucune clé C ou X dans les `Extras`) ; robustesse (`r087_many_unknown_task_children_are_kept_in_linear_time`).
