# Lokale Ratenkorrektur beim fortlaufenden Lernen

Der nachträgliche Abzug des balancierten Gewichtsmusters
verbesserte den Abruf auf den gewachsenen Graphen nicht.
Hier wird die alternative Hypothese geprüft, dass die
Trennung während der Plastizität entstehen muss.
[Ausgangsbefund](CONTINGENCY_GROWN_RESIDUAL_RESULTS_20261123.md)

## Lokaler Eingriff

Die native Regel verwendet pro Prä-Ankunft oder Post-Spike
den jüngsten Gegenpartner im bisherigen Zeitfenster `W`
mit exponentiellem Kernel der Zeitkonstante `τ`. Der
Korrekturarm führt zusätzlich je Synapse eine geglättete
Ankunftsrate und je Zielzelle eine geglättete Spike-Rate
mit derselben festen Zeitkonstante von zwei Sekunden. Für
eine lokale Rate `r` setzt er `q = r + 1/τ` und zieht
vom Kernel den Hintergrundwert `r·(1 − exp(−q·W))/q`
ab. Dieser Wert ist das erwartete Integral für einen
unabhängigen Poisson-Prozess mit der geschätzten Rate.
Existiert kein Gegenpartner im Fenster, ist der beobachtete
Kernel null. Ein gleichzeitig eingetroffener Prä-Impuls
bleibt beim kausalen Paar zulässig und wird bei der
zugehörigen Hintergrundschätzung ausgenommen.

Die Variante ist ein experimenteller, ereignislokaler
Eingriff. Tatsächliche Netzwerkspikes können von der
Unabhängigkeitsannahme abweichen. Native Lernfaktoren,
Eligibility-Integration, Gewichtsgrenzen und
Rundungsreste gelten in beiden Armen. Die Korrektur
wird erst nach der gemeinsamen Entwicklung zugeschaltet.
Ihre langsamen Spuren starten leer. Bereits gelernte
Gewichte, Eligibility und Paarhistorie bleiben an
diesem Übergang erhalten.

## Prospektiver Vergleich und nachfolgendes Lernen

Die Seeds 34211, 34223, 34237 und 34253 werden vorab
festgelegt. Jeder Seed hat 256 Core-Zellen und durchläuft
in beiden Lernvarianten die Vorwärtszuordnung,
die Umkehrzuordnung und balancierte Exposition. Die
beiden Zuordnungsarme vertauschen ausschließlich,
welcher Cue welches Folgebild ankündigt. Die
balancierte Exposition zeigt für jeden Cue beide
Folgebilder gleich oft. Die Reizhäufigkeiten sind
innerhalb einer Phase identisch.

Nach nativer Entwicklung lernt jeder Zweig acht
Wiederholungen der Aufgabe A und direkt anschließend
acht Wiederholungen einer Aufgabe B mit vier anderen
Bildern. Die A- und B-Bilder belegen disjunkte Pixel.
Jede Phase dauert vier Sekunden. Core-Homöostase und
lokale explorative Bildung bleiben über die Grenze
hinweg aktiv. Die Entfernung von Kanten ist wie im
vorigen `recent_retain`-Vergleich in allen Armen
ausgesetzt. Kein Zweig erhält Aufgabenlabels, externe
Rewards oder Rückmeldungen aus der Auswertung.

Die Proben nach A und B laufen in getrennten,
vollständig rekonstruierten Kopien. Der Trainingszweig
läuft zwischen A und B ohne Freeze fort. Pro Cue
enthält die eingefrorene Kopie elf Sekunden Ruhe und
eine vier Fenster lange Probe. Blank und die vier
Cues werden getrennt geprüft. Die Zielpools entstehen
vor dem Lernen aus den direkten Reaktionen auf die
Folgebilder. Zellen, die auf einen Cue, Blank oder
beide Folgebilder derselben Aufgabe reagieren, werden
aus den jeweiligen Zielpools ausgeschlossen.

## Endpunkte

Ein Erwerbstreffer erfordert zwei auswertbare Zielpools
und im passenden Pool mehr Spikes als im anderen, als
im entwickelten Ausgangszustand, als mit umgekehrter
Zuordnung und als bei balancierter Exposition. Die
Ruheaktivität und die Blank-Probe dürfen gegenüber
dem Ausgangszustand nicht zunehmen. Für Erwerb von B
muss die passende Antwort zusätzlich die B-Probe
desselben Zweigs direkt nach A übertreffen.
Erhalt von A verlangt einen A-Erwerbstreffer nach A
und erneut dieselben Selektivitätskriterien nach B.
Vier Treffer ergeben vollständigen Erwerb oder Erhalt.
Der gemeinsame Endpunkt verlangt vollständigen Erwerb
von A, vollständigen Erwerb von B und vollständigen
Erhalt von A. Teiltreffer und Zahl auswertbarer
Vergleiche werden je Seed und Variante separat
berichtet. Gewichts- und Spikeverläufe dokumentieren
die technische Wirkung auch bei ausbleibendem Abruf.

## Reproduktion

Die Konfiguration wird vor dem Hauptlauf versiegelt.
Der native Harness wiederholt jeden Trial und prüft
die Gleichheit der vollständigen Messdaten. Der
unabhängige Prüfer rekonstruiert Gewichtslebenszyklen,
Eingangsereignisse, Vorlagen, Akquisitions- und
Erhaltskriterien.
[Feste Konfiguration](config/visual_contingency_live_rate_20261123.toml)

```bash
config=research/config/visual_contingency_live_rate_20261123.toml
root=research/results/visual_contingency_live_rate_20261123
research/run.sh run "$config" --output "$root"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_live_rate_analysis.py "$root"
```
