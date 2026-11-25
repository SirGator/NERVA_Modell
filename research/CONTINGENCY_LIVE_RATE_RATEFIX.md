# Erwartungstreue lokale Ratenkorrektur (gepaarter Vergleich)

Die formale Prüfung hat zwei Fehler der Ratenkorrektur gezeigt: Sie
integrierte den **bloßen** exponentiellen Kernel, während die Regel seit der
stetigen Fenstermaske den **maskierten** Kernel `M(t)·e^{-t/τ}` anwendet, und
Sie teilte den Erwartungswert durch die Partnerwahrscheinlichkeit, obwohl die
Regel die Korrektur an *jedem* Ereignis abzieht. Damit überkorrigierte sie: bei
niedriger Rate `λW = 0,005` stand `0,9876` anstelle von `0,0047`, also eine
künstliche Depression statt der Hintergrundentfernung.
[Befund 5 und Korrektur C6](../../../NERVA_lean/docs/LEAN.md#befund-5-die-ratenkorrektur-integriert-einen-anderen-kernel-als-die-regel-anwendet).

Hier wird gemessen, was die korrigierte Korrektur im laufenden Modell bewirkt
und ob sie den Abruf verändert.

## Lokaler Eingriff

Die native Regel verwendet pro Prä-Ankunft oder Post-Spike den jüngsten
Gegenpartner im bisherigen Zeitfenster `W` mit exponentiellem Kernel der
Zeitkonstante `τ`. Der Korrekturarm führt zusätzlich je Synapse eine
geglättete Ankunftsrate und je Zielzelle eine geglättete Spike-Rate mit
derselben festen Zeitkonstante von zwei Sekunden.

Der Korrekturarm zieht jetzt den Erwartungswert **genau des von der Regel
angewendeten Kerns** ab:

```
K(t) = M(t)·e^{-t/τ}
I    = ∫₀^W λ·e^{-λt}·M(t)·e^{-t/τ} dt
```

Die Fenstermaske `M` ist `1` im Inneren des Fensters, fällt über das letzte
Sechzehntel quadratisch auf `0` und ist außerhalb `0`. Die Rate `λ` wird aus
derselben geglätteten Spur geschätzt wie im Vorgängerlauf. Anders als im
Lauf vom 23.11.2026 wird **nicht** durch die Partnerwahrscheinlichkeit
`1 - e^{-λW}` geteilt: die Korrektur wird an jedem Ereignis berechnet, auch
wenn der nächste Partner außerhalb des Fensters liegt; der unbedingte
Erwartungswert `I` ist damit der erwartungstreue Wert.

Die Variante ist ein experimenteller, ereignislokaler Eingriff. Tatsächliche
Netzwerkspikes können von der Unabhängigkeitsannahme abweichen. Native
Lernfaktoren, Eligibility-Integration, Gewichtsgrenzen und Rundungsreste
gelten in beiden Armen. Die Korrektur wird erst nach der gemeinsamen
Entwicklung zugeschaltet. Ihre langsamen Spuren starten leer. Bereits gelernte
Gewichte, Eligibility und Paarhistorie bleiben an diesem Übergang erhalten.

## Gepaarter Vergleich mit dem Lauf vom 23.11.2026

Es werden dieselben vier Seeds wie im Vorgängerlauf verwendet — 34211, 34223,
34237 und 34253 —, damit der Vergleich der beiden Korrekturen gepaart ist: Die
Eingänge, die Paarhistorie und die native Regel sind in beiden Läufen
identisch, und nur die Korrektur unterscheidet sich. Der native Arm des
neuen Laufs dient zugleich als Kontrolle für den Maskenwechsel der Regel
(`f32`-Berechnung gegen `f64` mit einmaliger Rundung).

Jeder Seed hat 256 Core-Zellen und durchläuft in beiden Lernvarianten die
Vorwärtszuordnung, die Umkehrzuordnung und balancierte Exposition. Die beiden
Zuordnungsarme vertauschen ausschließlich, welcher Cue welches Folgebild
ankündigt. Die balancierte Exposition zeigt für jeden Cue beide Folgebilder
gleich oft. Die Reizhäufigkeiten sind innerhalb einer Phase identisch.

Nach nativer Entwicklung lernt jeder Zweig acht Wiederholungen der Aufgabe A
und direkt anschließend acht Wiederholungen einer Aufgabe B mit vier anderen
Bildern. Die A- und B-Bilder belegen disjunkte Pixel. Jede Phase dauert vier
Sekunden. Core-Homöostase und lokale explorative Bildung bleiben über die
Grenze hinweg aktiv. Die Entfernung von Kanten ist wie im vorigen
`recent_retain`-Vergleich in allen Armen ausgesetzt. Kein Zweig erhält
Aufgabenlabels, externe Rewards oder Rückmeldungen aus der Auswertung.

Die Proben nach A und B laufen in getrennten, vollständig rekonstruierten
Kopien. Der Trainingszweig läuft zwischen A und B ohne Freeze fort. Pro Cue
enthält die eingefrorene Kopie elf Sekunden Ruhe und eine vier Fenster lange
Probe. Blank und die vier Cues werden getrennt geprüft. Die Zielpools entstehen
vor dem Lernen aus den direkten Reaktionen auf die Folgebilder. Zellen, die auf
einen Cue, Blank oder beide Folgebilder derselben Aufgabe reagieren, werden
aus den jeweiligen Zielpools ausgeschlossen.

## Endpunkte

Ein Erwerbstreffer erfordert zwei auswertbare Zielpools und im passenden Pool
mehr Spikes als im anderen, als im entwickelten Ausgangszustand, als mit
umgekehrter Zuordnung und als bei balancierter Exposition. Die Ruheaktivität
und die Blank-Probe dürfen gegenüber dem Ausgangszustand nicht zunehmen. Für
Erwerb von B muss die passende Antwort zusätzlich die B-Probe desselben Zweigs
direkt nach A übertreffen. Erhalt von A verlangt einen A-Erwerbstreffer nach A
und erneut dieselben Selektivitätskriterien nach B. Vier Treffer ergeben
vollständigen Erwerb oder Erhalt. Der gemeinsame Endpunkt verlangt
vollständigen Erwerb von A, vollständigen Erwerb von B und vollständigen
Erhalt von A. Teiltreffer und Zahl auswertbarer Vergleiche werden je Seed und
Variante separat berichtet. Gewichts- und Spikeverläufe dokumentieren die
technische Wirkung auch bei ausbleibendem Abruf.

Der Vergleich mit dem Vorgängerlauf erfolgt feldweise über die archivierten
Records desselben Seeds: native Trefferzahl gegen native Trefferzahl,
korrigierte Trefferzahl gegen korrigierte Trefferzahl, sowie die technischen
Kennzahlen der Gewichts- und Spikeverläufe.

## Reproduktion

Die Konfiguration wird vor dem Hauptlauf versiegelt. Der native Harness
wiederholt jeden Trial und prüft die Gleichheit der vollständigen Messdaten.
Der unabhängige Prüfer rekonstruiert Gewichtslebenszyklen, Eingangsereignisse,
Vorlagen, Akquisitions- und Erhaltskriterien.
[Feste Konfiguration](config/visual_contingency_live_rate_20261125.toml)

```bash
config=research/config/visual_contingency_live_rate_20261125.toml
root=research/results/visual_contingency_live_rate_20261125
research/run.sh run "$config" --output "$root"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_live_rate_analysis.py "$root"
```
