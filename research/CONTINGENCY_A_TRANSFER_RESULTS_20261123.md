# Neue Zieleingänge tragen beide erhaltenen A-Antworten

Das Sperren der während A gebildeten Eingänge in die Zielpools
löschte beide zuvor erworbenen und nach B erhaltenen A-Antworten.
Das Zurücksetzen älterer Gewichte oder der Gewichte neuer Kanten
ließ beide Zielzellen zu ihren ursprünglichen Zeiten feuern.
Auf dem gemeinsamen entwickelten Träger erreichte die gewachsene
Kantenanordnung keinen Treffer über ihrer gepaarten Ortskontrolle.
In den beiden relevanten Zweigen ließ die Ortskontrolle sämtliche
neuen Zieleingänge an ihrem Platz. Ob deren genaue Anordnung
Zuordnungsinformation trägt, bleibt offen.
[Versuchsplan](CONTINGENCY_A_TRANSFER.md)

## Eingriffe im gewachsenen Netz

Alle sechs archivierten Seeds und alle drei A-Zuordnungen wurden
rekonstruiert. Nach elf Sekunden unveränderter Ruhe griffen die
Eingriffe unmittelbar vor der Probe ein. `cue1` rief auf Seed 34313
im A-Umkehrzweig Zelle 72 nach 35,335 ms ab. Auf Seed 34327 feuerte
Zelle 73 nach 52,001 ms. Die Spikes entsprachen den archivierten
Antworten, die auch nach B erhalten geblieben waren.

| Eingriff im nativen Nach-A-Zustand | Zielspikes 34313 | Zielspikes 34327 | Selektive A-Fälle |
| --- | ---: | ---: | ---: |
| Intakt | 1 | 1 | 2 / 24 |
| Alle während A gebildeten Kanten sperren | 0 | 0 | 0 / 24 |
| Neue Eingänge in die Zielpools sperren | 0 | 0 | 0 / 24 |
| Übrige neue Kanten sperren | 1 | 0 | 1 / 24 |
| Ältere Gewichte auf Vor-A-Werte zurücksetzen | 1 | 1 | 2 / 24 |
| Neue Gewichte auf Geburtswerte zurücksetzen | 1 | 1 | 2 / 24 |

Die beiden Gewichtseingriffe liefen separat. Bei jedem Reset waren
sämtliche 54 Core-Spikefolgen identisch zum intakten Zustand.
Die Blockade der Zieleingänge reduzierte im jeweiligen Trefferzweig
die gesamte Core-Aktivität von 50 auf 49 Spikes. Beim Sperren aller
neuen Kanten sank sie auf 37 beziehungsweise 42. Auf Seed 34327
war auch die Übertragung über die übrigen neuen Kanten für diese
Antwort nötig. Welche einzelnen Kanten diese Wirkung vermitteln,
wurde mit den Gruppenblockaden noch nicht bestimmt.

## Transfer und Abdeckung der Ortskontrolle

| Übertragung auf den gemeinsamen Vor-A-Träger | Gewachsene Anordnung | Ortskontrolle | Gewachsen und strikt über Ortskontrolle |
| --- | ---: | ---: | ---: |
| Neue Kanten mit Geburtsgewichten | 1 / 24 | 2 / 24 | 0 / 24 |
| Neue Kanten mit trainierten Gewichten | 1 / 24 | 2 / 24 | 0 / 24 |
| Neue Kanten und trainierte ältere Gewichte | 1 / 24 | 2 / 24 | 0 / 24 |

Nur ältere Gewichte zu übertragen ließ alle 54 Core-Spikefolgen
identisch zur entwickelten Basis. Die Übertragung neuer Kanten änderte
je Transferbedingung 36 von 54 Spikefolgen. Auf Seed 34313 lösten die
gewachsene Anordnung und die Ortskontrolle je zwei passende Spikes in
Zelle 72 nach 26,001 und 52,001 ms aus. Auf Seed 34327 löste nur die
Ortskontrolle einen passenden Spike aus. Dieser lag bei 52,001 ms
in Zelle 73 und entsprach damit der nativen Antwort.

Die Ortskontrolle veränderte 316 von 4.355 neuen Quellen über alle
Seed–Zuordnungs-Zweige. Das waren je Zweig 3,4 bis 10,0 Prozent.
In den beiden erfolgreichen Umkehrzweigen blieb jeder neue Eingang
in die A-Zielpools unverändert. Insbesondere behielten alle sechs
neuen Kanten mit Ankünften an den beiden spikenden Zielzellen ihre
Quellen. Gleiche Zielantworten unter der Ortskontrolle können deshalb
auch aus dem gemeinsamen Teil der Kantenanordnung entstehen.

Die zusätzliche Geometrieprüfung fand für keine dieser sechs Kanten
einen zulässigen Zweier-Tausch unter beiden Grad- und Längenbedingungen.
Für je einen Eingang pro Seed fand sie einen längenpassenden Zyklus
mit vier beziehungsweise fünf Kanten. Alle sechs Eingänge haben
mindestens eine lokale, längenpassende Ersatzquelle, wenn der
Ausgangsgrad freigegeben wird. Daraus folgt ein konkreter Gegenversuch.
Gezielte Quellenrotationen prüfen zwei Eingänge bei exakt erhaltenem
Grad. Für die übrigen Eingänge braucht es einen getrennten Vergleich,
der den Zielgrad und die Länge festhält und Änderungen des Ausgangsgrads
mit einer eigenen Kontrolle erfasst.

## Reproduktion

Der Runner wiederholte jeden vollständigen Record. Die unabhängige
Prüfung rekonstruierte Ortskontrolle, Eingriffsmengen, Gewichte, Proben
und Kriterien. Der Offline-Neubau aus den archivierten Quellen
reproduzierte den technischen Kurzlauf bytegenau. Der Projektcheck
bestand 479 Rust-, 138 Harness-Python- und 10 Dashboard-Tests.
Modell `8cb2eae` und Library `c13d9f7` waren vor dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_a_transfer_20261123.json)

Das lokale ZIP enthält die versiegelten Eingaben, Rohdaten, Quellen,
Binary und die unabhängigen Auswertungen.
`research/results/archives/visual_contingency_a_transfer_20261123.zip`

```bash
root=research/results/visual_contingency_a_transfer_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_a_transfer_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/descriptive_index.py" "$root" | cmp - "$root/descriptive-index.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/terminal_control_feasibility.py" "$root" | cmp - "$root/terminal-control-feasibility.json"
sha256sum -c research/results/archives/visual_contingency_a_transfer_20261123.zip.sha256
```
