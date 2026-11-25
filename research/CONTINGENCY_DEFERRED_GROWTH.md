# Aufgeschobene explorative Kantenbildung im A/B-Lernen

Im früheren Kontrastversuch entstanden 1.577 von 2.063 explorativen
A-Kanten während der 157-ms-Anlaufzeit. Der spätere Gegenversuch
zeigte, dass eine vorlaufbasiert gewählte Quelle einen B-Abruf
kausal tragen kann. Hier prüfen wir, ob eine längere Evidenzphase
die Quellenwahl und den Erwerb auf neuen Seeds verbessert.
[Kontrastversuch](CONTINGENCY_CONTRAST_GROWTH_RESULTS_20261124.md) ·
[Kausaldiagnose](CONTINGENCY_B_SOURCE_RESULTS_20261124.md)

## Vier gepaarte Bedingungen

Die Quellenwahl nach letzter Aktivität oder nach der bisherigen
Vorlauf-Evidenz wird mit sofortiger oder um 500 ms aufgeschobener
explorativer Bildung gekreuzt. Die feste Anlaufzeit entspricht
zwei Cue–Outcome-Darbietungen samt Blankfenstern. Die Vorlaufregel
sammelt wie zuvor ab 157 ms lokale Evidenz und kann sie im
Aufschub bereits vor der ersten Kantenwahl aktualisieren.
Die Sperre beginnt am gemeinsamen entwickelten Checkpoint und
wird bei B nicht erneut gesetzt. Während des Aufschubs laufen
native Kantenbildung, Homöostase und Gewichtslernen weiter.
Pruning bleibt im gesamten Test ausgesetzt. Die Grenze von
einer explorativen Kante je Ziel gilt gemeinsam über A und B.

Die unmittelbaren Bedingungen prüfen den Ausgangseffekt der
Quellenwahl. Der gleiche Aufschub unter Aktualitätswahl
kontrolliert den geänderten Zeitpunkt eines frühen Eingangs.
Die tatsächliche Zahl neuer Eingänge, ihre Zeitpunkte und
spätere Rückwirkungen auf das Netz werden ausgewiesen. Die
feste Wartezeit ist ein diagnostischer Eingriff. Die Vorlauf-
Evidenz selbst nutzt ausschließlich lokale Netzereignisse.

## Unabhängige Seeds und Endpunkte

Die sechs Seeds und beide Bildabbildungen sind vor dem Hauptlauf
festgelegt. Für die ersten drei Seeds wird `(r,c)` auf
`(r,(3c+2r+1) mod 8)` abgebildet. Für die letzten drei gilt
`((3r+c+1) mod 8,7-c)`. Die bijektiven Abbildungen erhalten
die Bildgröße, die Zahl aktiver Pixel und die Disjunktheit
der acht Reize. Beide Layouts sind mit neuen Seeds belegt.
[Konfiguration](config/visual_contingency_deferred_growth_20261125.toml)

Die vier Bedingungen starten auf identischen entwickelten
Checkpoints. Vorwärts, umgekehrt und balanciert trainierte
A-Historien werden mit allen drei B-Historien gekreuzt.
Die eingefrorenen Proben erfolgen nach elf Sekunden Ruhe.
Die bisherigen Kontrollen für Aktivität, passende Zielpools,
balancierte Reizexposition und gegensätzliche Zuordnungen
gelten weiter.

Die Auswertung vergleicht den Erwerbsvorteil von Vorlauf
gegenüber Aktualität mit und ohne Aufschub je Seed.
A- und B-Erwerb, bedingter A-Erhalt und vollständige
A→B-Pfade werden separat ausgewiesen. B-Zweige derselben
A-Historie sind keine unabhängigen Wiederholungen. Die
technische Auswertung prüft identische frühe Dynamik der
beiden Aufschubarme, fehlende explorative Geburten vor
500 ms sowie Änderungen in der Quellenwahl nach Freigabe.

```bash
config=research/config/visual_contingency_deferred_growth_20261125.toml
root=research/results/visual_contingency_deferred_growth_20261125
research/run.sh run "$config" --output "$root"
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_deferred_growth_analysis.py "$root"
```
