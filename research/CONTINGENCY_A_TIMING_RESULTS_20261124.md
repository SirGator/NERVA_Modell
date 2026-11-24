# Ankunftszeiten entscheiden bei gleicher Dosis über den A-Zielspike

Auf dem zuvor stummen Vor-A-Träger von Seed 34327 erzeugte das
Verschieben einer einzigen erregenden Ankunft einen Zielspike.
Ein Versatz um 1 ms zu beiden Seiten ließ eine Antwort bestehen.
Auf dem rotierten Träger löschte sowohl das Trennen der beiden
gebündelten Ankünfte als auch ihre gemeinsame Verschiebung den
Zielspike. Über das vollständige 200-ms-Fenster blieben je Eingang
Anzahl, Vorzeichen und Amplituden exakt erhalten.
[Versuchsplan](CONTINGENCY_A_TIMING.md)

## Erregung und Hemmung in denselben Zuständen

Jede Eingangsfolge ersetzt die an einer ausgewählten Zielzelle
ankommenden Synapsensignale. Zellzustand und übriges Netz starten
aus derselben eingefrorenen Kopie nach elf Sekunden Ruhe.
Alle 30 unveränderten Proben reproduzierten Core-Spikes, Root-Ereignisse,
Ruhe-Spikezahl und Zielzell-Batches exakt. Die neun Eingriffe änderten
nur die im Plan ausgewiesenen Zeitpunkte von `cue1`. Auf jeder
Probe blieb der vollständige vorzeichengetrennte Amplitudensatz
pro Eingang gleich. Die Zielzellen blieben bei Blank und `cue0` stumm.

| Seed und Träger | Zeitänderung | Zielspike nach Cue-Beginn |
| --- | --- | ---: |
| 34313, nach A | Unverändert | 35,335 ms |
| 34313, nach A | Drei erregende Ankünfte auf −4, 0 und +4 ms verteilen | 34,335 ms |
| 34313, nach A | Dieselben Ankünfte gemeinsam um +4 ms verschieben | 39,335 ms |
| 34327, nach A | Unverändert | 52,001 ms |
| 34327, nach A | Erste Hemmung von 18,668 auf 1,001 ms vorziehen | 35,335 ms |
| 34327, nach A | Erste Hemmung von 18,668 auf 36,335 ms verzögern | 35,335 ms |
| 34327, Vor-A-Träger mit gewachsenen Kanten | Unverändert | kein Spike |
| 34327, Vor-A-Träger mit gewachsenen Kanten | Erregende Ankunft auf 43,668 ms legen | 43,668 ms |
| 34327, Vor-A-Träger mit gewachsenen Kanten | Dieselbe Ankunft auf 42,668 oder 44,668 ms legen | 43,668 bzw. 44,668 ms |
| 34327, Vor-A-Träger mit rotierten Quellen | Unverändert | 43,668 ms |
| 34327, Vor-A-Träger mit rotierten Quellen | Eine oder beide gebündelten Ankünfte auf 35,335 ms vorziehen | kein Spike |

Auf dem gewachsenen Vor-A-Träger lag die Summe aus Potential und
signiertem Input bei 43,668 ms zunächst 0,00072 unter der Schwelle.
Durch die zusätzliche Ankunft in diesem Batch lag sie 0,08838 darüber.
Eine Millisekunde frühere Ankunft ließ bei 43,668 ms einen Abstand
von 0,07520 oberhalb der Schwelle. Die Zelle integriert also auch über
benachbarte Batches. Auf dem rotierten Träger unterschritt die nach
35,335 ms gemeinsam vorgezogene Erregung die Schwelle um 0,05459.
In diesem Batch traf zugleich der hemmende Eingang 1440 ein.

Im trainierten Zustand von Seed 34327 lag das Potential beim frühen
erregenden Batch um 35,335 ms zunächst 0,02894 unter der Schwelle.
Nach Vorziehen beziehungsweise Verzögern der ersten Hemmung lag
es 0,02691 beziehungsweise 0,05671 darüber. In beiden Fällen blieben
die erregenden Zeitpunkte und Amplituden unverändert. Die frühere
Ankunft der Hemmung lässt bis zu diesem Batch mehr Zeit zum Abklingen.
Die verzögerte Ankunft wirkt erst nach ihm.

Damit ist für diese eingefrorenen Zustände belegt, dass die Zeitlage
wirksamer Eingänge allein die Antwort ändern kann. Der gemeinsame
frühe Versatz auf dem rotierten Träger zeigt die Abhängigkeit vom
zellulären Kontext. Die Messung stützt als nächste Hypothese eine
lokale Bildungsregel, die wiederholte vorlaufende Aktivität vor
postsynaptischer Depolarisation bewertet. Eine prospektive Prüfung
sollte sie bei gleicher Zahl von Bildungsgelegenheiten gegen die
bestehende Auswahl und zeitlich permutierte Evidenz antreten lassen.
Erwerb, Erhalt und Weiterlernen werden auf neuen Seeds und Bildpaaren
im fortlaufenden A/B-Versuch geprüft.

## Reproduktion

Beide vollständigen Records wurden wiederholt und unabhängig geprüft.
Ein Offline-Neubau aus den versiegelten Quellen reproduzierte beide
Records bytegenau. Der vorherige Quellentausch-Record für Seed 34313
ließ sich mit demselben Runner bytegenau reproduzieren. Modell
`a194b6b` und Library `c13d9f7` waren vor dem Hauptlauf veröffentlicht.
Der Projektcheck bestand 481 Rust-, 154 Harness-Python- und 10 Dashboard-Tests.
[Kompakte Daten](dokumentation/visual_contingency_a_timing_20261124.json)

Das lokale Archiv enthält versiegelte Eingaben, Pläne, Quellen,
Binary, Rohdaten und unabhängige Auswertungen.
`research/results/archives/visual_contingency_a_timing_20261124.zip`

```bash
root=research/results/visual_contingency_a_timing_20261124
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_a_timing_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/electrical_index.py" "$root" | cmp - "$root/electrical-index.json"
sha256sum -c research/results/archives/visual_contingency_a_timing_20261124.zip.sha256
```
