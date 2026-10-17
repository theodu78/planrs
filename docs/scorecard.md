# Scorecard — parité avec MS Project

Mise à jour à la fin de chaque jalon. Trois niveaux, jamais confondus : **implémenté** (le code existe), **testé** (tests verts avec valeurs calculées à la main), **vérifié contre MS Project** (cas de corpus ou d'oracle reproduit à l'identique). Le câblage n'est pas le comportement.

| Domaine | Implémenté | Testé | Vérifié | Notes |
|---|---|---|---|---|
| Calendriers ouvrés | oui | oui | – | J1 : semaine type, exceptions (récurrences dépliées), semaines datées, héritage à un niveau (boucle refusée), Standard / 24 heures / Équipe de nuit, forme compilée (fonction cumulée W), conversions ouvrées et écoulées, placement des durées et des jalons. Règles R-011 à R-031 (statut `hypothèse`), une par fichier de test `tests/rules/r011_*` à `r031_*`, propriétés P1 à P10 (oracle naïf fondé sur `resolve_day`). Aucune règle vérifiée contre MS Project (pas de corpus ni d'oracle avant J2). |
| Chemin critique, liens, décalages | – | – | – | J2 |
| Récapitulatives | – | – | – | J2 |
| Contraintes, planification à rebours | – | – | – | J3 |
| MSPDI aller-retour | – | – | – | J3 |
| CSV | – | – | – | J3 |
| Registre de commandes, CLI, MCP, undo | – | – | – | J4 |
| Corpus public, plancher de parité | – | – | – | J2-J4 |
| Ressources, affectations, types de tâches | – | – | – | J5 |
| Avancement, ligne de base, explain_date, what_if | – | – | – | J6 |
| Interface egui | – | – | – | J7 |
| IA, PDF, traductions, web, releases | – | – | – | J8 |

Parité corpus : 0 / 0 cas (plancher `FLOOR` = 0).
