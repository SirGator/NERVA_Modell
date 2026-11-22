# Visuelle Kontingenz nach der Entwicklung

Auf fünf neuen Seeds mit je 256 Core-Zellen erfüllte **kein Seed** das
Abrufkriterium für die visuelle Zuordnung. Nach elf Sekunden Ruhe blieben
die vollständigen Core-Spikefolgen mit übertragenen Trainingsgewichten
gegenüber der Kontrolle exakt gleich. Das gilt für Abstände von 50 und
150 ms zwischen Cue- und Folgebildbeginn sowie für Überträge der
rekurrenten und der gesamten Gewichte. [Vorregistrierter Versuchsplan](VISUAL_CONTINGENCY.md)

## Was gemessen wurde

Zwei Cues gingen zwei räumlich getrennten Folgebildern voraus. In den beiden
Trainingsarmen waren die Zuordnungen gegensätzlich. Die balancierte Kontrolle zeigte jedes
Folgebild gleich häufig nach jedem Cue. Der unabhängige Prüfer zählte in jedem
Arm 1.536 Root-Spikes und bestätigte gleiche Exposition pro Eingangskanal.
Die native Drei-Faktor-STDP war nach der gemeinsamen Entwicklungsphase aktiv.
Die Auswertung verwendete feste Zielgruppen, die im Reset jeweils ausschließlich
auf eines der Folgebilder feuerten. Alle fünf Seeds hatten beide Zielgruppen.

| Seed | Zellen der Zielgruppen 0 / 1 | Original-Cue-Fälle mit Ankunft an passenden Zellen, 50 / 150 ms | Davon mit gelerntem Gewichtsdelta, 50 / 150 ms |
| --- | --- | --- | --- |
| 33207 | 8 / 8 | 4 / 4 | 3 / 3 |
| 33211 | 4 / 8 | 3 / 3 | 2 / 3 |
| 33223 | 10 / 4 | 3 / 3 | 3 / 2 |
| 33247 | 11 / 11 | 4 / 4 | 4 / 3 |
| 33287 | 5 / 7 | 2 / 2 | 1 / 1 |

Das Abrufkriterium blieb für Original-Cues und für die vier getrennt geprüften
Cue-Hälften in beiden Gewichtsumfängen auf **0 von 5 Seeds pro Zeitabstand**.
In 540 gepaarten Core-Spikefolgenvergleichen einschließlich der separaten
Folgebild- und Schwarzproben war jede Folge aus Neuronen-IDs und Zeitstempeln
identisch zu Reset. Die Reset-Proben beider Zeitabstände waren je Seed gleich.

## Wo die Wirkung ausblieb

Je Trainingsarm änderten sich 32 bis 98 rekurrente Gewichte. Die L1-Summe
der Änderungen lag zwischen 0,0000230 und 0,000233. Das größte beobachtete
Einzeldelta betrug 0,0000226 in nativen Gewichtseinheiten. Die gegensätzlichen
Zuordnungen ergaben verschiedene Endgewichte. Daraus folgt keine
Zuordnungsspezifität der Abrufwirkung.

Die Tabelle enthält eine nachträgliche Pfadauswertung. Pro Seed und Zeitabstand
werden die zwei Original-Cues unter beiden festen Zuordnungen betrachtet.
Ein Fall zählt, wenn innerhalb der 200-ms-Probe mindestens eine direkte
Core→Core-Kante einen Impuls mit Amplitude ungleich null an eine Zelle der
passenden Zielgruppe liefert. Über beide Zeitabstände trifft das auf 32 von
40 Fällen zu. In 25 Fällen trifft es auch auf mindestens eine Kante mit
sichtbarem Gewichtsdelta zu. Die Arrivals belegen, dass veränderte Kanten
teils auf tatsächlich genutzten Pfaden lagen. Die Auswertung misst dafür
weder die Differenz der Ankunftsamplituden noch das Potenzial zur
Integrationszeit. Bei fehlenden direkten Ankünften bleiben indirekte Pfade
und eine andere Spikefolge unter einem späteren Eingriff möglich.

Damit ist Plastizität nach Entwicklung in diesem Test technisch belegt.
Ein persistentes visuelles Assoziationsgedächtnis war im nativen Spikeabruf
unter den getesteten Bedingungen nicht nachweisbar. Der Übertrag erfasste
sichtbare Gewichte. Die trainierten chemischen und zellulären Zustände wurden
dabei durch den gemeinsamen entwickelten Ausgangszustand kontrolliert.
Der Versuch prüft weder laufenden Abruf im trainierten Actor noch biologische
Gültigkeit oder fortlaufenden Erwerb neuer Aufgaben. In der Lückenbedingung
ändert sich neben der Cue-Folgebild-Pause auch die Zeit bis zum nächsten Cue.

Die nächste kausale Frage ist, ob die beobachteten Kanten die passenden
Zielzellen messbar näher an die Schwelle bringen. Dafür lassen sich bei
identischen Cue-Spikefolgen die signierten Ankunftsamplituden und
Integrationsgrenzen der beiden gegensätzlichen Zuordnungen direkt mit Reset
und der balancierten Kontrolle paaren. Das trennt fehlende aktive Ankünfte,
subschwellige Wirkung und Effekte auf die jeweils falschen Zielzellen.

## Reproduktion

Zehn native Records, 20 Ausführungen einschließlich vollständigem Replay und
60 Trainingsphasen nach dem Entwicklungscheckpoint. Der Code und das
Protokoll waren vor der Hauptmessung auf `NERVA_Modell 8ddc0eb` veröffentlicht,
mit `NERVA f973a24` als Library. Die archivierten Quellen erzeugten beim
Offline-Neubau ein byteidentisches natives Manifest.
Die Projektprüfung bestand mit 462 Rust-, 88 Harness-Python- und
10 Dashboard-Tests sowie Formatierung, Clippy und Linkprüfung.

Der vollständige Lauf und das geprüfte ZIP liegen lokal unter
`research/results/visual_contingency_20261122/` und
`research/results/archives/visual_contingency_20261122.zip`. Das Archiv
enthält beide Quellstände, den nativen Runner, alle Rohdaten, die unabhängige
Auswertung und das Skript der nachträglichen Pfadauswertung. Git enthält die
kompakte [maschinenlesbare Auswertung](dokumentation/visual_contingency_results_20261123.json).

Die folgenden Befehle laufen aus `main/NERVA_Modell/`.

```bash
root=research/results/visual_contingency_20261122
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
./research/run.sh report research/config/visual_contingency_20261122.toml --binary "$root/nerva-research"
python3 "$root/verify_contingency.py" "$root"
python3 "$root/describe.py" "$root"
sha256sum -c research/results/archives/visual_contingency_20261122.zip.sha256
```

| Beleg | SHA-256 |
| --- | --- |
| Vorregistrierte Dateiliste | `dc3f49038e25f16c69f66d366876a1f05155c292913466eb525a7a6847bbd871` |
| Auswertungs- und Rohdatenbeleg | `100c53fa4245c3bd00df3fabf8d5782f3e936fccfbad1f8e3a2591ce35a5709b` |
| Archiv | `bf03aff6008fa9b007bdfeebe5fe9685c8c9d7861d10571614f6e532126801f4` |
