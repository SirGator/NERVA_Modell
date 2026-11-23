# Struktur und Gewichte beim eingefrorenen B-Abruf

Der native Arm des [Ratenvergleichs](CONTINGENCY_LIVE_RATE.md)
erreichte einen B-Teiltreffer auf Seed 34223, Umkehrzuordnung,
`cue_b0`. Dieses vorregistrierte Folgeprotokoll untersucht retrospektiv,
welche Komponenten des trainierten Zustands diesen Abruf tragen.
[Ausgangsbefund](CONTINGENCY_LIVE_RATE_RESULTS_20261123.md)

## Fester Umfang und Rekonstruktion

Die Kohorte umfasst ausschließlich die vier archivierten Seeds 34211,
34223, 34237 und 34253 unter nativer Lernregel, jeweils mit `forward`,
`reverse` und `balanced`. Keine Auswahl nach neuen Abrufwerten ist zulässig.
Die übrigen Seeds prüfen Reichweite und unspezifische Wirkungen; der bekannte
Einzeltreffer bleibt eine ausgewählte Mechanismusdiagnose.

Jeder Record wird aus dem archivierten Eingang rekonstruiert. Die nativen
Frame-Ledger des Trainings A und B und beide Endtopologien müssen mit den
archivierten JSON-Werten exakt übereinstimmen. Auch die unveränderten
nativen B-Proben `blank`, `cue_b0` und `cue_b1` müssen vollständig reproduziert
werden. Eine Abweichung verhindert die Interpretation des betroffenen Records.
Die aufgezeichneten Referenzen nach A und im entwickelten Ausgangszustand
bleiben unverändert. Zielpools werden aus dem Archiv übernommen und unabhängig
nach dem Ausgangsprotokoll geprüft, ohne erneute Auswahl anhand der Eingriffe.

## Zeitpunkt und Faktoren

Nach B werden Lernen, Bildung, Abbau und Homöostase für die Probe eingefroren.
Jede Bedingung und jede Probe erhält eine unabhängige Rekonstruktion. Zunächst
laufen die elf Sekunden Blank-Ruhe ohne Gewichtsreset und ohne Blockade.
Erst unmittelbar vor dem B-Cue- oder Blank-Puls erfolgt der Eingriff.
Der vollständige Zustand bis zu diesem Zeitpunkt muss zwischen den
Bedingungen identisch sein.

Zwei binäre Faktoren ergeben vier Bedingungen:

| Bedingung | B-geborene Übertragung sperren | Persistente Gewichte zurücksetzen |
| --- | --- | --- |
| Unverändert | nein | nein |
| Nur Strukturblockade | ja | nein |
| Nur Gewichtsreset | nein | ja |
| Beide Eingriffe | ja | ja |

Als B-geboren gelten Kanten, die während B gebildet wurden und am B-Ende
aktiviert sind. Die Blockade unterbindet ihre gesamte Übertragung ab dem
Eingriffszeitpunkt, einschließlich bereits eingeplanter Ankünfte. Sie setzt
keine Zellzustände zurück und entfernt keine bereits integrierte Wirkung.
Der Gewichtsreset betrifft sämtliche an A- und B-Ende aktivierten persistenten
Kanten mit verändertem Gewicht: Jede erhält exakt ihr eigenes A-Endgewicht.
Diese Menge wird nicht auf ursprüngliche Core-Kanten oder einen Plastizitätstyp
beschränkt. B-geborene Kanten gehören nicht zur Resetmenge.

Der Reset lässt bereits eingeplante Amplituden auf älteren Kanten unverändert;
erst spätere Übertragungen verwenden das zurückgesetzte Gewicht. Zellzustände,
Carry-Zustände, bisherige Aktivität und vollständige Bildungs- und
Trainingsgeschichte bleiben beim Eingriff erhalten. Kantenidentitäten,
Mitgliedschaften, A-/B-Gewichte, Eingriffszeit und betroffene ausstehende
Ankünfte werden dokumentiert.

Zwei zusätzliche Bedingungen blockieren jeweils nur B-geborene Kanten mit
Ziel in der Vereinigung beider vorab festgelegten B-Zielpools beziehungsweise
mit Ziel in deren Komplement. Beide behalten die B-Endgewichte. Diese
strukturellen Teilmengenkontraste verwenden die vorgegebenen Zielpools.
Poolgrößen, Kantenzahlen und Kanten mit Ankünften werden ausgewiesen.

## Messwerte und Kontrollvergleiche

Alle sechs Bedingungen erhalten `blank`, `cue_b0` und `cue_b1` unter allen
drei Trainingszuordnungen. Das vollständige native Readout umfasst
Spikezeitreihen, Ruhe- und Probeaktivität, rohe Spikezahlen in beiden
B-Zielpools und die geprüften Gewichte und Schaltzustände der Kanten.
Die Analyse entnimmt die Proben nach A und im Ausgangszustand dem Archiv.

Für jeden Cue und jede gerichtete Zuordnung werden die ursprünglichen Gates
separat berichtet: auswertbare Pools, passender Pool stärker als falscher Pool,
Ausgangszustand, Gegenzuordnung, balancierte Exposition und derselbe Zweig nach
A; dazu keine zusätzliche Ruhe- oder Blank-Aktivität gegenüber dem
Ausgangszustand. Gegenzuordnung und balancierte Kontrolle verwenden jeweils
dieselbe Eingriffsbedingung. Alle Vergleichszahlen und Margen bleiben sichtbar.
Ein Gatewechsel allein durch veränderte Kontrollen gilt nicht als Verlust
oder Zunahme der B-Zielantwort: Deren rohe Spikezahlen werden stets separat
bewertet. Teiltreffer, Nenner und vollständiger B-Erwerb werden ausgewiesen.

Für jeden numerischen Endpunkt sei `Y00` unverändert, `Y10` blockiert,
`Y01` zurückgesetzt und `Y11` kombiniert. Positive Differenzen entsprechen
einem Verlust beim Eingriff. Berichtet werden `Y00 − Y10`, `Y00 − Y01`,
`Y01 − Y11`, `Y10 − Y11` und die Interaktion
`Y11 − Y10 − Y01 + Y00`, insbesondere für beide Zielpoolzahlen.
Die beiden Teilblockaden werden mit `Y00` und der vollständigen Blockade
verglichen. Gatewechsel werden zusätzlich mit ihrem auslösenden Vergleich
erläutert; es gibt keine nachträgliche Auswahl günstiger Endpunkte.

Ein Verlust bei Einzelblockade stützt die Notwendigkeit der betroffenen
Übertragung für diesen eingefrorenen Abruf. Ein Verlust erst beim kombinierten
Eingriff ist mit Redundanz vereinbar. Ausbleibende Effekte begrenzen die
nachgewiesene Notwendigkeit auf diesen Zustand und diese Probe. Der Versuch
prüft weder prospektive Lernfähigkeit noch eine Vermittlung des Trainings
durch Strukturbildung: Die gesamte Lernhistorie liegt vor dem Eingriff.

## Versiegelung und Reproduktion

Vor dem Hauptlauf werden Protokoll, Quellen beider Projekte, Quellarchive,
archivierte Eingangsrecords, Runner/Binary und unabhängiger Prüfer mit SHA-256
versiegelt. Der native Befehl wiederholt jeden vollständigen Ausgabe-Record
intern und verlangt deterministische Gleichheit. Der unabhängige Prüfer prüft
Archivgleichheit, Mengen, Zeitpunkt, Kontrollgates und sämtliche Kontraste.
Fehlende Records und technische Abweichungen werden als solche ausgewiesen.

Studienwurzel ist `research/results/visual_contingency_b_factor_20261123`.
Die vier archivierten Records liegen vor dem Lauf unverändert unter
`input/<job>.json`; pro Record entsteht `raw/<job>.json`.
Die Befehle gelten ab `NERVA_Modell/`:

```bash
root=research/results/visual_contingency_b_factor_20261123
for input in "$root"/input/*.json; do
  job=${input##*/}
  research/target/release/nerva-research contingency-b-factor "$input" \
    > "$root/raw/$job.partial" || exit
  mv "$root/raw/$job.partial" "$root/raw/$job"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_b_factor_analysis.py "$root"
```
