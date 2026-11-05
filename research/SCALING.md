# Frei skalierbare Netzgrößen und dieselben Diagnoseprüfungen

Der Größen-/Umweltadapter akzeptiert jetzt **jede positive ganzzahlige Anzahl
von Verarbeitungsneuronen innerhalb der expliziten Ressourcenbudgets**. Weder
Zweierpotenzen noch die frühere Spanne 16–1.024 sind vorgeschrieben. Beispiele
sind 1, 17, 257, 1.025, 4.096 und 65.536. Gesamtneuronenzahl, Synapsenzahl und
tatsächlicher E/I-Anteil werden weiterhin separat gespeichert.

Dieselbe wissenschaftliche Prüfmethode gilt für jede Größe. Das bedeutet nicht,
dass jede Größe dieselben funktionalen Gates bestehen muss. Ein sehr kleines
Netz kann zu wenig Information tragen oder überhaupt keine hemmende Core-Zelle
enthalten (`floor(N/5)`). Das wird gemessen, nicht als technischer Fehler versteckt.

## Größen konfigurieren

[plasticity_diagnosis.toml](config/plasticity_diagnosis.toml) führt den vorgeschlagenen
Diagnoseversuch mit der ursprünglichen Trainingslänge für mehrere Netzgrößen aus.
[plasticity_scaling.toml](config/plasticity_scaling.toml) ist die kürzere technische
Vorprüfung mit vier statt acht Trainings-/Probekommandos.

```toml
[base]
processing_neurons = 17       # ausdrücklich gewählte Referenzgröße
max_total_neurons = 100000    # Budget einschließlich Sensor-/Motor-/Kopiezellen
max_observed_events = 10000000

[[studies]]
id = "plasticity_by_size"
method = "plasticity_diagnostics"
[studies.options]
sizes = [17, 256, 1025, 4096]  # beliebige eindeutige positive Ganzzahlen
```

Die vollständige Datei enthält außerdem Seeds, Trainingsdauer, Voraussetzungen
und Gesamtbudget. `sizes` bestimmt die untersuchten Größen; `processing_neurons`
unter `[base]` bestimmt die separate Referenz. Liegt diese außerhalb der Liste,
wird sie zusätzlich ausgewertet und budgetiert. Bis zu 256 Größenangaben sind
pro Design möglich, soweit das Jobbudget ausreicht.

Zum Beispiel kann `sizes = [17, 1000, 4097, 65536]` gewählt werden. Neue
Protokolle und geänderte Quellen brauchen einen neuen Ausgabeordner. Vom
Projektverzeichnis `DSVLM`:

```bash
./NERVA_Modell/research/run.sh research-plan NERVA_Modell/research/config/plasticity_diagnosis.toml
./NERVA_Modell/research/run.sh research NERVA_Modell/research/config/plasticity_diagnosis.toml
./NERVA_Modell/research/run.sh research-report NERVA_Modell/research/config/plasticity_diagnosis.toml
```

Der erste neue Lauf und seine Aussagegrenzen stehen in
[SCALING_RESULTS_20261105.md](SCALING_RESULTS_20261105.md).

## Identischer Diagnoseversuch pro Größe

`plasticity_diagnostics` konstruiert je Größe vier frisch aufgebaute Varianten:

| Strukturentwicklung | Homöostase | STDP |
|---|---|---|
| aus | aus | an |
| aus | an | an |
| an | aus | an |
| an | an | an |

Jede Variante erhält die vorhandenen sechs Rückkopplungs-/Plastizitätsbedingungen
einschließlich vollständig eingefrorener Kontrollen. Alle Varianten einer Größe
verwenden dieselben Seeds, Anfangsnetze, Bildverläufe und Zeitpunkte. Diese
Gleichheit wird über die Varianten hinweg geprüft; bei Unterschieden bricht die
Kontrastberechnung ab. Netzwerke unterschiedlicher Größe werden nicht als
identische Anfangsnetze behandelt.

Je Größe werden visuelle Auslesbarkeit, Vorhersagegenauigkeit, Positionsfehler,
Synapsenzahl und native Ereigniszähler getrennt ausgewertet. Der Bericht enthält
seedgepaarte Effekte beider Schalter bei jeweils festem anderem Schalter sowie
ihre Differenz der Differenzen. Die einzelnen Endpunkte bleiben separat.

Strukturentwicklung schaltet **Wachstum und Pruning gemeinsam**. Die Diagnose
isoliert daher noch keinen reinen Pruningeffekt. Sie vergleicht frisch trainierte
Netze mit unveränderten lokalen Regeln und passenden gealterten Kontrollen;
es ist keine akute Ablation eines gemeinsamen trainierten Checkpoints.
Visuelle Genauigkeit wird nach Training und an den Kontrollen gemessen, nicht
an einem unabhängigen Vortrainingscheckpoint.

Die vier Kontraste sind explorativ. Die vorhandene optionale Finalistenprüfung
bleibt auf kontrolladjustierte aktive Vorhersagegenauigkeit begrenzt; sie
bestätigt keine nachträglich ausgewählten Mechanismenkontraste. Dafür sind neue,
vorab festgelegte Einzelvergleiche nötig.

## Grenzen durch Ressourcen statt feste Größenklassen

- `max_total_neurons` ist eine native Vorabgrenze und enthält die I/O-Zellen.
  Für die aktuelle Sensorik gilt `Gesamtgröße = Coregröße + 11`. Der Standard
  beträgt 100.000. Ein höheres Budget kann ausdrücklich konfiguriert werden.
- `max_observed_events` begrenzt Ereignisse je Assay; maximal 10.000.000 sind
  derzeit konfigurierbar. Ein überschrittenes Budget liefert einen Fehler und
  keinen erfundenen Messwert.
- `[requirements].max_synapses` und `.max_observed_events` sind zusätzlich
  externe Zulassungskriterien. Für größere Netze müssen sie zum gewünschten
  Ressourcenrahmen passen. Sie ändern keine neuronale Lernregel.
- `max_trials` zählt Kontrollgruppen und reservierte Bestätigung. Bei `K`
  Größen und `S` Explorationsseeds kostet das vollständige Diagnoseraster
  `K × 4 × S × 6` Jobs, sofern die Baseline bereits im Raster enthalten ist.
  Hinzu kommen konservativ `2 × V × 6` reservierte Prüfjobs bei `V` Prüfseeds.
  Jeder Job umfasst zwei vollständige Ausführungen einschließlich Wiederholung.

Das Beispiel reserviert für vier Größen und jeweils zwei Explorations-/Prüfseeds
192 Explorationsjobs plus 24 Prüfjobs, insgesamt höchstens 216 Jobs. Die
einzelnen gültigen Größen werden vor dem ersten Modellaufbau geprüft. Ein
zurückgestelltes Design wird nicht teilweise ausgewertet.

Hostspeicher und Laufzeit bleiben praktische Grenzen. Die Software behauptet
keine unbegrenzte physische Skalierbarkeit und keine bestätigten Fähigkeiten
für nur geplante Größen.

## Technische Umsetzung

Die bisherige vollständige Suche durch alle Core-Zellen je Quellneuron und
Wachstumsziel wurde durch einen unveränderlichen räumlichen Index ersetzt.
Er filtert geometrisch lokale Kandidaten und gibt sie wieder in derselben
ID-Reihenfolge zurück. Die Auswahl, Distanzprüfung, lokale Bewertung,
Tie-Breaks und Zufallsziehungen bleiben erhalten. Die sensorische Auswahl
ermittelt die nächsten 24 Zellen ohne vollständige Sortierung aller Zellen.

Die neue Bibliotheksfunktion `local_candidates_in` nutzt denselben lokalen
Scorer wie `local_candidates`. Tests vergleichen beide Verfahren und den
räumlichen Index mit vollständigen Suchen, einschließlich negativer Koordinaten
und f32-Rundungen an Bucketgrenzen. Gespeicherte Referenzdigests belegen außerdem
identische Anfangs-, Trainings- und Endzustände für die früheren 17er-/256er-Läufe.
Große Debug-Snapshots werden für ihre Digests direkt gestreamt, statt erst eine
große Zwischenzeichenkette anzulegen. Netzwerkzustände selbst bleiben im Speicher.

## Dieselbe Testsuite für eigene Größen

`NERVA_TEST_SIZES` parametrisiert die eigentlichen Prüfungen. Es gibt keine
separaten Implementierungen pro Größenklasse:

```bash
# Gleiche neuronale Prüfungen, sechs Bedingungen und vier Diagnoseschalterkombinationen:
NERVA_TEST_SIZES=37,1025,4096 cargo test --offline --release \
  --manifest-path NERVA_Modell/research/Cargo.toml --test sensorimotor

# Gleiche Topologieinvarianten für die Modellbibliothek:
NERVA_TEST_SIZES=37,65536 cargo test --offline --release \
  --manifest-path NERVA_Modell/Modell/Cargo.toml --test scaling

# Gleiche Harness-, Budget-, Diagnosekontrast- und Archivprüfungen:
NERVA_TEST_SIZES=1,37,1025 python3 -W error::ResourceWarning -m unittest discover \
  -s NERVA_Modell/research/tests -p 'test_sensorimotor.py' -v
```

Die regulären Tests enthalten sehr kleine, ungerade, nicht potenzförmige Größen
und Größen über der alten Grenze. Sie prüfen generische Invarianten statt
vorgegebener Leistungswerte: Größe und Gesamtzahl, dünne Verdrahtung,
E/I-Rundung, gleiche Anfangszustände/Erfahrungen, Schalterwirkung, eingefrorene
Kontrollen, Vorhersagezeitpunkt, Wiederholung, Ressourcenfehler und unveränderte
Rohdaten bei erneuter Auswertung.

Zusätzlich wurden Topologie und alle sechs Rückkopplungsbedingungen mit
65.536 Core-Neuronen tatsächlich geprüft. Das ist ein technischer
Skalierbarkeitsnachweis für diesen Assay, kein Kognitionsnachweis.
