# Dosisgleicher Test der Ankunftszeiten beim A-Abruf

Quellentausch und Hemmblockade änderten in zwei archivierten Fällen
Zeitpunkt oder Auftreten eines A-Zielspikes. Dieser Versuch stellt
Ankunftszeiten gezielt um und hält Anzahl, Vorzeichen und Amplituden
je Eingangsplatz fest. Die beiden Fälle bleiben eine retrospektive
Mechanismusdiagnose. [Ausgangsbefund](CONTINGENCY_A_SOURCE_RESULTS_20261123.md)

## Gemeinsamer Eingriff

Der Runner rekonstruiert das native A-Training und die zuvor geprüften
Transferträger. Er kopiert den eingefrorenen Zustand nach elf Sekunden
unveränderter Ruhe. In der Kopie ersetzt er die eingehenden synaptischen
Übertragungen zu genau einer Zielzelle durch ihre aufgezeichneten
Ereignisse. Die Zielzelle, das übrige Netz und die native Modulation
laufen weiter. Ausgehende Spikes der Zielzelle wirken weiter auf das
übrige Netz. Ihre eigenen Eingangssignale folgen der festgelegten Folge.

Jede Folge deckt das vollständige 200-ms-Auswertefenster ab. Der Runner
verändert nur die explizit angegebenen Zeitpunkte. Bei Gleichzeitigkeit
bleibt die relative Zeilenreihenfolge des Archivs erhalten. Die
unveränderte Folge muss auf jedem Träger für Blank, `cue0` und `cue1`
den archivierten Eingang, die Core-Spikes, die Ruhe-Spikezahl und die
Integrationsbatches der Zielzelle exakt reproduzieren. Abweichungen
machen den betroffenen Record ungültig. Die Änderungen betreffen nur
`cue1`. Blank und `cue0` bleiben in jedem Arm unveränderte Kontrollen.

## Festgelegte Kontraste

Auf Seed 34313 wird der trainierte Zustand mit Zielzelle 72 geprüft.
Drei erregende Ankünfte der neuen Zieleingänge trafen beim archivierten
Zielspike zusammen. Ein Arm verschiebt den kleinsten Platz um 4 ms
nach vorn und den größten um 4 ms nach hinten. Der mittlere Platz
bleibt auf seiner Zeit. Die Verschiebungskontrolle verlegt alle drei
Ankünfte gemeinsam um 4 ms nach hinten.

Auf Seed 34327 werden drei Träger mit Zielzelle 73 geprüft.

| Träger | Dosisgleiche Zeitänderung |
| --- | --- |
| Trainierter Zustand | Erste hemmende Ankunft an Platz 1440 um denselben Betrag vor- oder zurückverlegen. Die verzögerte Ankunft liegt 1 ms nach dem früheren Spike der archivierten Hemmblockade. |
| Gemeinsamer Vor-A-Träger mit gewachsenen Kanten | Eine erregende Ankunft an Platz 1632 auf den Zeitpunkt verschieben, an dem die erregenden Eingänge im archivierten Rotationsarm zusammentrafen. Zwei Kontrollen legen dieselbe Ankunft jeweils 1 ms davor und danach. |
| Gemeinsamer Vor-A-Träger mit rotierten Quellen | Eine der zwei gebündelten Ankünfte auf den ursprünglichen früheren Zeitpunkt zurücklegen. Die Verschiebungskontrolle verlegt beide Ankünfte zusammen auf diesen früheren Zeitpunkt. |

Die Pläne werden deterministisch aus dem vorherigen Archiv erzeugt.
Sie verwenden die vorhandenen Ankunftszeiten, die Archivantwort nach
Hemmblockade und die festgelegten Zeitabstände. Pro Träger wird aus
derselben Ereignisliste und demselben Startzustand geprüft, ob ein
Zielspike auftritt und wann er auftritt. Die unabhängige Prüfung
vergleicht die vollständigen Integrationsbatches und die vorzeichen-
getrennten Amplituden je Eingangsplatz. Die elektrische Dosis ist
über das gesamte 200-ms-Fenster festgehalten. Die zeitliche Verteilung
innerhalb dieses Fensters bildet die Intervention.

Die Effekte gelten für die gewählten archivierten Zustände und die
kontrollierte Eingangsfolge einer Zielzelle. Ob dieselben zeitlichen
Zusammenhänge eine lokale, fortlaufend lernende Bildungsregel
verbessern, braucht einen anschließenden Versuch auf unabhängigen Seeds
und Bildpaaren.

## Reproduktion

Der Lauf bindet beide archivierten Eingaberecords, den vorigen
Quellentauschplan und den abgeleiteten Zeitplan mit SHA-256. Der native
Runner wiederholt jeden Record. Ein unabhängiger Prüfer erzeugt die
Zeitpläne neu, prüft die ausgelieferten Ereignisse und rekonstruiert
die Summation der Integrationsbatches.

```bash
prior=research/results/visual_contingency_a_source_20261123
root=research/results/visual_contingency_a_timing_20261124
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_a_timing_plan.py "$prior" "$root/plans"
mkdir -p "$root/raw"
for source in "$root"/growth_input/*.json; do
  name=${source##*/}
  research/target/release/nerva-research contingency-a-timing "$source" "$root/plans/$name" > "$root/raw/$name.partial" || exit
  mv "$root/raw/$name.partial" "$root/raw/$name"
done
PYTHONDONTWRITEBYTECODE=1 python3 research/contingency_a_timing_analysis.py "$root"
```
