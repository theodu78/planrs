# Fixtures

Plannings MSPDI de test écrits pour planrs (donc versionnés, sous la licence du dépôt) et leurs résultats attendus figés (« golden files »).

## Convention

- Un planning par fichier : `<sujet>.xml` (MSPDI), accompagné de ses résultats attendus (dates, durées, marges) relus et validés à la main. Le format exact des résultats attendus est fixé avec la première fixture.
- Toute variation d'un résultat attendu fait échouer le test ; la modifier exige une justification dans le message de commit (quelle règle a changé, pourquoi).
- Pas de fichier dont la provenance n'est pas claire : les fichiers publics tiers vont dans `tests/corpus/` (non versionnés), pas ici.

Aucune fixture pour l'instant (jalon J1, socle). Les premières arrivent avec le modèle et les calendriers.
