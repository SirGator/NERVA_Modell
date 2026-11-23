# Zwei A-Teiltreffer bleiben nach weiterem Training erhalten

Auf sechs neuen Seeds bestand NERVA **2 von 24** A-Erwerbsvergleichen.
Beide erworbenen Cue–Zuordnungen blieben in jedem ihrer neun B-Folgezweige
abrufbar. Bei B erzielten die beiden aktiven Bildungsregeln denselben
**1 von 72** Teiltreffer. Bei ausgesetzter Bildung waren es **0 von 72**.
Kein Zweig erfüllte den gemeinsamen Endpunkt für beide A- und B-Cues.
[Versuchsplan](CONTINGENCY_LIVE_GROWTH.md)

## Erwerb und Erhalt auf denselben Checkpoints

Jede B-Zuordnung startete innerhalb einer A-Historie aus einer exakten
Kopie desselben laufenden Zustands. Lernspuren, Ereigniswarteschlange,
Zufallszustand und Homöostaseuhren gingen mit. Die drei B-Zuordnungen
hatten je Bild und Phase dieselben Reizhäufigkeiten. Die Kopien für
Abrufproben wurden unabhängig eingefroren. Alle Zielpools waren auswertbar.

| Bildung während B | B-Erwerb | A-Erhalt nach B | Vollständige A→B-Pfade | In B neu gebildete Kanten |
| --- | ---: | ---: | ---: | ---: |
| Jüngste Aktivität | 1 / 72 | 6 / 6 | 0 | 2.781 |
| Räumliche Nähe | 1 / 72 | 6 / 6 | 0 | 2.763 |
| Ausgesetzt | 0 / 72 | 6 / 6 | 0 | 0 |

Die Erhaltsvergleiche sind wiederholte Gegenversuche mit zwei erworbenen
A-Fällen. Sie stammen von Seed 34313 und 34327. Auf beiden Seeds löste
`cue1` in der A-Umkehrzuordnung einen Spike im passenden Pool aus.
Ausgangszustand, falscher Pool, Gegenrichtung und balancierte Exposition
lieferten jeweils null. Nach B feuerten auf 34313 Zelle 72 bei 35,335 ms
und auf 34327 Zelle 73 bei 52,001 ms nach Probe-Beginn. Das waren dieselben
Zellen und relativen Spikezeiten wie nach A. Insgesamt blieb der Abruf in
**18 von 18** Folgezweigen erhalten. Auf beiden Seeds kamen in der A-Probe
Signale über je drei während A gebildete Eingänge an der jeweiligen
spikenden Zielzelle an. Diese Beobachtung bestimmt noch keine kausale Rolle.

Der B-Teiltreffer lag auf Seed 34301 nach vorwärts gelerntem A und B.
`cue_b1` löste in beiden Bildungsbedingungen zwei passende Zielspikes aus.
Die Umkehr- und die balancierte B-Zuordnung lösten je einen Spike im
selben Pool aus. Vor B blieb dieser Pool auf den Cue stumm. Der Befund
stützt einen Beitrag der Bildung zu diesem B-Abruf. Ein Wiederholungstreffer
auf den anderen fünf Seeds und ein vollständiges B-Stratum blieben aus.

## Wirkung der Kandidatenauswahl

Die aktive Wahl nach jüngster Aktivität und die räumliche Kontrolle
bildeten in allen 54 gepaarten B-Zweigen unterschiedliche Verbindungen.
Die Zahl gebildeter Kanten unterschied sich in 18 Zweigen. In 44 Zweigen
änderte sich die Core-Spikefolge beim Training. In 85 von 270 gepaarten
Abrufproben änderte sich die Core-Spikefolge ebenfalls. Ob die Auswahl
bei anderen Aufgaben den selektiven Erwerb verbessert, bleibt offen.

Der nächste kausale Test sollte die neu gebildeten A-Kanten aus den
beiden Erhaltsfällen auf denselben eingefrorenen Träger übertragen.
Zuordnungs- und kantenzahlgleiche Ortskontrollen können prüfen, ob die
Kantenanordnung genügt, um den passenden Abruf mitzunehmen. Die Auswertung
sollte die tatsächlich gewachsenen Zweige als Referenz nutzen und den
Beitrag der Gewichte separat prüfen. Für einen belastbaren Nachweis
fortlaufenden Lernens bleibt ein vollständiger A→B-Pfad auf neuen Seeds
offen.

## Reproduktion

Alle sechs Läufe wurden vollständig wiederholt und anhand
ihrer Rohdaten unabhängig ausgewertet. Ein Offline-Neubau
beider versiegelter Quellarchive reproduzierte sämtliche
deterministischen Felder des technischen Kurzlaufs mit 64 Core-Zellen.
Die Regression reproduzierte die deterministischen Felder
des archivierten Raten-Kurzlaufs und den B-Faktor-Kurzlauf
bytegenau. Die Projektprüfung bestand 477 Rust-, 134
Harness-Python- und 10 Dashboard-Tests. Library `c13d9f7`
und Modell `a5c772f` waren vor dem Hauptlauf veröffentlicht.
[Kompakte Daten](dokumentation/visual_contingency_live_growth_20261123.json)

Das lokale ZIP enthält Protokoll, Spezifikationen, Binary,
Quellen, Rohdaten und beide Auswertungen.
`research/results/archives/visual_contingency_live_growth_20261123.zip`

```bash
root=research/results/visual_contingency_live_growth_20261123
(cd "$root" && sha256sum -c preregistration.sha256 && sha256sum -c verification.sha256)
PYTHONDONTWRITEBYTECODE=1 python3 "$root/contingency_live_growth_analysis.py" "$root" | cmp - "$root/analysis.json"
PYTHONDONTWRITEBYTECODE=1 python3 "$root/descriptive_index.py" "$root" | cmp - "$root/descriptive-index.json"
sha256sum -c research/results/archives/visual_contingency_live_growth_20261123.zip.sha256
```
