# Lokale Ratenkorrektur verändert Gewichte und lässt den Abruf unverändert

Auf vier vorab festgelegten Seeds bestand die lokale
Ratenkorrektur dieselben Abrufvergleiche wie die native
Paarregel. Beide erreichten **0 von 16** bei Aufgabe A
und **1 von 16** bei der direkt anschließenden Aufgabe B.
Alle Zielpools waren auswertbar. Da kein Fall A erwarb,
kam der Vergleich zum Erhalt von A auf **0 auswertbare
Fälle**. Kein Seed bestand den gemeinsamen Endpunkt.
[Versuchsplan](CONTINGENCY_LIVE_RATE.md)

Beide Varianten starteten am gleichen entwickelten
Checkpoint. Sie lernten A und B ohne Pause und ohne
Labels oder externe Rewards. Vorwärts-, Umkehr- und
balancierte Reizfolgen hatten innerhalb jeder Phase
dieselben Reizhäufigkeiten. Strukturbildung und
Core-Homöostase liefen weiter, bei ausgesetztem Abbau
von Kanten. Die getrennten, eingefrorenen Proben liefen
außerhalb der Trainingszweige.

| Seed | Zielpools A | Zielpools B | A nativ / korrigiert | B nativ / korrigiert |
| --- | ---: | ---: | ---: | ---: |
| 34211 | 7 / 8 | 2 / 6 | 0 / 0 | 0 / 0 |
| 34223 | 4 / 6 | 3 / 7 | 0 / 0 | 1 / 1 |
| 34237 | 6 / 4 | 7 / 2 | 0 / 0 | 0 / 0 |
| 34253 | 5 / 8 | 4 / 12 | 0 / 0 | 0 / 0 |

Auf Seed 34223 löste `cue_b0` nach B in der Umkehrzuordnung
zwei Spikes im passenden Pool aus. Die Vorwärtszuordnung
löste einen Spike in demselben Pool aus. Im falschen Pool,
im Ausgangszustand, bei balancierter Exposition und nach A
lagen die jeweiligen Vergleichswerte bei null. Ruhe und
Blank bestanden die Ruhebedingung. Die Zuordnung war auf
diesen einzelnen Cue beschränkt. Nach A blieb der passende
Pool in allen 16 Cue–Zuordnungs-Vergleichen stumm.

## Technische Wirkung und nächster Gegenversuch

Die Korrektur veränderte gegenüber der nativen Regel
je nach Seed und Zuordnung die Endgewichte auf 28 bis
127 ursprünglich aktiven Core→Core-Kanten nach A und
34 bis 138 nach B. Die quadratisch gemittelte Differenz
über diese ursprünglichen Kanten betrug nach A zwischen
`1,34 × 10⁻⁷` und `7,78 × 10⁻⁷` und nach B zwischen
`7,27 × 10⁻⁷` und `2,06 × 10⁻⁶`. In sämtlichen 24
gepaarten Seed–Zuordnungs–Phasenvergleichen waren die
Core-Spikezeitreihen im Training und die gebildeten
Kanten identisch. Auch die Core-Spikezeitreihen aller
120 gepaarten eingefrorenen Proben stimmten exakt überein.

Die lokale Zwei-Sekunden-Schätzung setzt einen
unabhängigen Poisson-Hintergrund voraus. Wie gut
diese Annahme zu den Netzwerkspikes passt, bleibt
offen. Der Einzelbefund bei B lässt offen, welche
Änderung im Netz den Abruf trägt.

Der nächste kausale Test kann den archivierten B-Zweig
von Seed 34223 einfrieren und bei gleicher Reizprobe
zwei Eingriffe kreuzen. Ein Eingriff setzt die Gewichte
der bereits vor B vorhandenen Kanten auf den Stand
vor B zurück. Der andere deaktiviert nur die während B
neu gebildeten Kanten. Der unveränderte Zustand, die
passenden balancierten und umgekehrten Zuordnungen
sowie die gleichen Eingriffe auf den übrigen Seeds
liefern Kontrollen. Ein Effekt auf dem bekannten
Teiltreffer wäre eine Mechanismusdiagnose. Eine
Bestätigung der Lernfähigkeit braucht neue Seeds.

## Reproduktion

Alle vier Records wurden im nativen Harness vollständig
wiederholt. Der unabhängige Python-Prüfer prüfte die
Integrität der Rohdaten und berechnete die Zielpools und
alle Erfolgskriterien neu. Ein Offline-Neubau aus beiden
Quellarchiven reproduzierte die deterministischen Felder
des Kurzlaufs. Die ausgeschaltete Korrektur reproduzierte
den archivierten Record des vorherigen Protokolls bytegenau.
Der Projektcheck bestand 474 Rust-, 127 Harness-Python- und
10 Dashboard-Tests. Library `9d60220` und Modell `e1a554e`
waren vor dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_live_rate_20261123.json)

Das lokale Rohdatenarchiv liegt unter
`research/results/archives/visual_contingency_live_rate_20261123.zip`.
Es enthält das versiegelte Protokoll, Binärdatei, Quellen,
Rohdaten, Prüfer und den ergänzenden deskriptiven Vergleich.

```bash
root=research/results/visual_contingency_live_rate_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_live_rate_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/descriptive_index.py" "$root" | cmp - "$root/descriptive-index.json"
sha256sum -c research/results/archives/visual_contingency_live_rate_20261123.zip.sha256
```
