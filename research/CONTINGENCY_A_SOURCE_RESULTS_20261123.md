# Quellentausch verändert die zeitliche Integration beim A-Abruf

Das gemeinsame Sperren der drei aktiven neuen Zieleingänge
löschte auf beiden archivierten Seeds die A-Antwort. Der gemeinsame
Reset der älteren und neuen Gewichte erhielt beide Spikezeiten exakt.
Eine gradbewahrende Quellenrotation veränderte die Zielantwort
über den Zeitpunkt der Ankünfte. Auf Seed 34327 trafen nach dem
Tausch zwei erregende Signale bei 43,668 ms im selben Integrationsbatch
zusammen und lösten einen Spike aus. Die erregende Gesamtdosis im
festen Messfenster war dabei kleiner als in der intakten Referenz.
[Versuchsplan](CONTINGENCY_A_SOURCE.md)

## Abhängigkeiten in den beiden nativen Zuständen

Die Tabelle zeigt die Latenzen nach Beginn von `cue1` im jeweiligen
A-Umkehrzweig. Aktiv bezeichnet eine Kante mit einer nichtnull
Ankunft in der archivierten Probe derselben Zielzelle.

| Eingriff nach A | Seed 34313, Zelle 72 | Seed 34327, Zelle 73 |
| --- | ---: | ---: |
| Intakt | 35,335 ms | 52,001 ms |
| Drei aktive neue Eingänge sperren | kein Spike | kein Spike |
| Zwei neue Eingänge ohne beobachtete Ankunft sperren | 35,335 ms | 52,001 ms |
| Ältere und neue Gewichte gemeinsam zurücksetzen | 35,335 ms | 52,001 ms |

Auf Seed 34313 ließen alle drei Einzelblockaden den Spike bei
35,335 ms bestehen. Auf Seed 34327 löschte das einzelne Sperren
der erregenden Kante 1542 oder 1632 die Antwort. Die Blockade
der hemmenden Kante 1440 zog den Spike um 16,666 ms vor.
Die positive Amplitudensumme bis zur intakten Referenzlatenz
blieb bei dieser Hemmblockade exakt gleich. Die gemessene
Zielzelle blieb in allen Armen bei `cue0` und Blank stumm.

## Eingriffe auf dem gemeinsamen Vor-A-Träger

| Anordnung und Eingriff | Seed 34313 | Seed 34327 |
| --- | ---: | ---: |
| Gewachsene Anordnung, intakt | 26,001 und 52,001 ms | kein Spike |
| Frühere Ortskontrolle, intakt | 26,001 und 52,001 ms | 52,001 ms |
| Vollständige Rotation, beide Anordnungen | 27,001 und 53,001 ms | 43,668 ms |
| Nur Zieleingang der Rotation, beide Anordnungen | 27,001 und 53,001 ms | 43,668 ms |
| Nur übrige Kanten der Rotation, beide Anordnungen | 26,001 und 52,001 ms | 52,001 ms |
| Längenpassende Gruppenersetzung, beide Anordnungen | 35,335 ms | kein Spike |

Auf Seed 34327 führte die vollständige Rotation die erregenden
Ankünfte über Kanten 1542 und 1632 bei 43,668 ms zusammen.
Im gewachsenen Transfer erreichte das Potential dort ungefähr
1,35914 bei einer Schwelle von 1,31403. Die intakte Anordnung
blieb am selben Zeitpunkt mit 1,31331 knapp darunter. Ihre
positive Amplitudensumme bis zur Referenzlatenz von 52,001 ms
lag bei 3,99978. Unter der Rotation fiel sie auf 3,55335.
Die räumlichen Kontrollen halten Eingangsgrad und Gewicht fest.
Die vollständige Rotation hält auch die Ausgangsgrade fest.

Die Gruppenersetzung erreichte zwei der drei aktiven Slots auf
Seed 34313 und alle drei auf Seed 34327. In beiden Anordnungen
blieben die Ersatzplätze während `cue1` ohne wirksame Ankunft.
Die Gruppenersetzung und die Blockade derselben Slots ergaben
identische Zielzell-Ankünfte und Integrationsbatches. Dieser Arm
misst damit den Verlust wirksamer Eingänge. Die Rotationen und
die Hemmblockade zeigen zusätzlich, wie Zeitpunkt und Vorzeichen
der übertragenen Aktivität die Antwort verändern. Die Befunde
betreffen die beiden ausgewählten eingefrorenen A-Antworten.

Ein nächster Gegenversuch kann Ankunftsamplituden je Eingang
festhalten und erregende sowie hemmende Zeitpunkte getrennt
verschieben. Das prüft, ob die gemessene Bündelung die Antwort
unter gleicher Dosis trägt. Eine daraus abgeleitete lokale
Bildungsregel braucht anschließend unverbrauchte Seeds und
Bildpaare im fortlaufenden A/B-Test.

## Reproduktion

Der Runner wiederholte beide vollständigen Records. Der unabhängige
Prüfer rekonstruierte die Eingriffspläne und prüfte Topologien,
Ankünfte, Integration und Spikezeiten. Ein Offline-Neubau aus den
archivierten Quellen reproduzierte beide Records bytegenau. Der
Archivkurzlauf des vorherigen Transferprotokolls blieb bytegleich.
Der Projektcheck bestand 479 Rust-, 147 Harness-Python- und
10 Dashboard-Tests. Modell `3da3de3` und Library `c13d9f7` waren vor
dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_a_source_20261123.json)

Das geprüfte ZIP mit Eingaben, Plänen, Rohdaten, Quellen und
Auswertungen liegt unter
`research/results/archives/visual_contingency_a_source_20261123.zip`.

```bash
root=research/results/visual_contingency_a_source_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_a_source_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/electrical_index.py" "$root" | cmp - "$root/electrical-index.json"
sha256sum -c research/results/archives/visual_contingency_a_source_20261123.zip.sha256
```
