# Fortlaufendes Lernen mit kontrollierter Strukturbildung

Die Blockade neuer B-Zieleingänge hob den bisherigen einzelnen
B-Erwerbstreffer auf. Das zeigt ihre Notwendigkeit beim Abruf,
belegt aber noch keine Speicherung der Zuordnung allein durch
die Struktur. Der folgende prospektive Vergleich prüft auf neuen
Seeds, ob aktivitätsabhängiges Wachstum den selektiven Erwerb
von B und den Erhalt von A begünstigt.
[Ausgangsbefund](CONTINGENCY_B_FACTOR_RESULTS_20261123.md)

## Festgelegter Versuchsraum

Die sechs Seeds 34301, 34307, 34313, 34319, 34327 und 34333
verwenden jeweils 256 Core-Zellen. Bilder, Reizzeiten und
Phasendauer bleiben wie in
[CONTINGENCY_LIVE_RATE.md](CONTINGENCY_LIVE_RATE.md).
A und B enthalten je acht Wiederholungen und disjunkte Bilder.
Die direkte Reaktion im entwickelten Ausgangszustand setzt
pro Aufgabe zwei feste Zielpools. Die Reizhäufigkeiten sind je
Phase zwischen allen Bedingungen angeglichen.

Jeder Seed lernt zunächst drei A-Historien mit Vorwärtszuordnung,
Umkehrzuordnung und balancierter Exposition. Während A gilt
native Bildung mit `RecentExcitatory` und vollständiger Retention.
Jeder exakte native Live-Zustand nach A wird anschließend in neun
frische B-Zweige aufgeteilt. Sie kreuzen die drei B-Zuordnungen
vorwärts, umgekehrt und balanciert mit den drei Bildungspolitiken
`recent`, `nearest` und `off`. Damit entstehen je Seed 27 Zweige.
Die B-Zuordnung wird unabhängig von der A-Historie gewählt.

`recent` und `nearest` verwenden dieselbe native Bildung und
dieselbe zusätzliche explorative Gelegenheit je Ziel über A und B.
Während B unterscheidet sich nur die Kandidatenauswahl nach
jüngster Aktivität beziehungsweise räumlicher Nähe. Die Zahl
tatsächlich gebildeter Kanten darf sich unterscheiden und wird
gemessen. `off` setzt während B sämtliche Bildung aus.
Pruning bleibt in allen Phasen und Armen ausgeschaltet.

Native lokale Lernregeln und Core-Homöostase bleiben durchgehend
aktiv, auch beim Fork von A nach B. Der Trainingszweig erhält
keine Aufgabenlabels, externen Rewards oder Auswertungsrückmeldungen.
Ein Freeze erfolgt ausschließlich in unabhängigen Probenkopien.

## Proben und vorab festgelegte Kriterien

Direkte Folgebildantworten vor dem Lernen bestimmen die Zielpools.
Die Ausschlüsse und die vier Fenster lange Cue-Probe folgen dem
Ratenprotokoll. Jede eingefrorene Blank- oder Cue-Probe beginnt
mit elf Sekunden Ruhe. Getrennte Kopien prüfen den Ausgangszustand,
den Zustand nach A und den Zustand nach B. Die B-Cue-Proben am
Checkpoint nach A sind die gemeinsame Referenz für alle neun
Forks derselben A-Historie.

Ein Erwerbstreffer verlangt zwei auswertbare Zielpools und im
passenden Pool mehr Spikes als im anderen Pool, im entwickelten
Ausgangszustand, bei entgegengesetzter Zuordnung und bei balancierter
Exposition. Ruheaktivität und Blank-Antwort dürfen gegenüber dem
Ausgangszustand nicht zunehmen. A-Erwerb wird nach A zwischen den
drei A-Historien geprüft. B-Erwerb wird nach B ausschließlich zwischen
B-Zuordnungen aus demselben exakten A-Zustand bei gleicher
Bildungspolitik geprüft. Die passende B-Antwort muss außerdem die
gemeinsame Probe unmittelbar vor B übertreffen.

A-Erhalt nach B ist nur für nach A bereits erworbene Fälle zulässig.
Er verlangt erneut die A-Kriterien im Vergleich der A-Historien bei
festgehaltener B-Zuordnung und Bildungspolitik. Fehlender A-Erwerb
ist kein auswertbarer Erhaltsfall.

Ein gemeinsamer Pfad verlangt den Erwerb und Erhalt beider A-Cues
sowie den Erwerb beider B-Cues. Jeder Pfad verbindet eine gerichtete
A-Zuordnung mit einer gerichteten B-Zuordnung. Es gibt höchstens
vier solche Pfade je Seed und Bildungspolitik.

## Auswertung und Reichweite

Der Hauptvergleich ist der gepaarte B-Erwerbsgewinn von `recent`
gegenüber `nearest` und `off` bei gleicher A-Historie und
B-Zuordnung. Berichtet werden Treffer und auswertbare Fallzahlen,
die Zahl vollständiger A-Strata mit vier bestandenen B-Vergleichen,
A-Erwerb, bedingter A-Erhalt und gemeinsame Pfade.
Nicht auswertbare Fälle werden getrennt ausgewiesen.
Tatsächlicher Kantenzuwachs und Zielpool-Spikes je Cue werden für
alle Bedingungen berichtet, einschließlich Teiltreffern.

Ein Vorteil von `recent` wäre Evidenz für wachstumsabhängige
gelernte Selektivität unter fortlaufender nativer Plastizität.
Er würde weder gleiche realisierte Kantenzahlen voraussetzen noch
belegen, dass die Struktur allein die Zuordnung speichert.
Gewichte, Dynamik und Homöostase können gemeinsam zum Abruf beitragen.

## Reproduktion

Die Konfiguration wird vor dem Hauptlauf versiegelt.
Das Szenario heißt `hippocampal_contingency_live_growth`.
Der native Harness prüft die Kopien am Fork und wiederholt
jeden vollständigen Lauf. Ein unabhängiger Prüfer rekonstruiert
Vorlagen, Reizhäufigkeiten, Kantenlebenszyklen, Zielspikes sowie
Erwerbs-, Erhalts- und Pfadkriterien aus den wiederholten Messdaten.
Das Archiv enthält auch die Ergebnisse der Repository-Validierung.
[Feste Konfiguration](config/visual_contingency_live_growth_20261123.toml)

```bash
config=research/config/visual_contingency_live_growth_20261123.toml
root=research/results/visual_contingency_live_growth_20261123
research/run.sh run "$config" --output "$root"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_live_growth_analysis.py "$root"
```
