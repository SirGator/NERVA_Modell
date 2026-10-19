# M8: Weiterleitung erreichbar, funktionales Wirkungslernen weiter offen

Abschlussbericht vom 17.10.2026. **Mit vierfach stärkeren Core-Gewichten entstehen
zusätzliche rekurrente Spikes. Längere STDP-Zeitkonstanten verstärken separat die
synaptische Zuordnung zur erfahrenen Wirkung. Keine der 13 Bedingungen liefert
aber einen belastbaren funktionalen Vorhersagevorteil gegenüber allen Kontrollen
und bei beiden Tastenbelegungen.**

[Zehn weitere Seeds 510–519](../../Messungen/Experimente/m8-confirm/summary.md),
[Pilot mit Seeds 500–502](../../Messungen/Experimente/m8-pilot/summary.md),
[Vorhersagekontraste](../../Messungen/Experimente/m8-confirm/prediction-contrasts.csv),
[Spike-Weiterleitung und Kontrollarme](../../Messungen/Experimente/m8-confirm/propagation-summary.csv),
[Gewichtsdiagnose](../../Messungen/Experimente/m8-confirm/weight-summary.csv),
[unabhängiger Audit und deskriptive Gewichtsvergleiche](../../Messungen/Experimente/m8-confirm/diagnostics.json),
[Softwareverifikation](../../Messungen/Experimente/m8-confirm/verification.md).

## Vorgehen und vorab festgelegter Scan

M8 prüft die sechs vorgeschlagenen Faktoren mit bestehenden neuronalen Mechanismen.
Eine Referenz und jeweils zwei zusätzliche Werte ergeben 13 Bedingungen. Pro
Bedingung wird genau ein Faktor verändert. Die vollständigen Pilot- und
Bestätigungspläne wurden vor den neuronalen Messreihen gespeichert. Alle
Bedingungen werden unverändert auf neuen Seeds geprüft; es findet keine Auswahl
nach Piloterfolg statt. Separate kleine Softwareprüfungen gehören nicht zu den
wissenschaftlichen Messungen.

| Faktor | Referenz | Zusätzliche Werte |
|---|---|---|
| A: Core→Core-Gewichte, Erregung und Hemmung | Faktor 1 | 2 / 4 |
| B: Core-Schwelle | 1 | 0,8 / 0,6 |
| C: Bestehende STDP-Zeitkonstanten für LTP und LTD | 20 ms | 50 / 100 ms |
| D: Core→Core-Verzögerung | Geseedet 0,5–2 ms | Einheitlich 0,5 / 5 ms |
| E: Höchstens ausgehende lokale Core-Kanten | 4 | 8 / 12 |
| F: Visuelle Sensor→Core-Gewichte | Faktor 1 | 0,5 / 1,5 |

Fest bleiben 32 Core-Zellen, der aus aktuellen Pixeln abgeleitete 2-Kanal-Sensor,
deterministische Kodierung, drei Aktionskopien mit Gewicht 0,9, Core-Tau 20 ms,
50 ms Aktions-/Wirkungsabstand und 256 Trainingstrials. Homöostase und
Strukturentwicklung bleiben aus. STDP-Amplituden 0,0001/0,00012, Paarfenster
100 ms und Gewichtsgrenzen 0–2 ändern sich nicht. C verbreitert ausdrücklich
auch die Depression; es ist kein ausschließliches Verstärken der LTP.

Jeder Fall enthält gekoppelt gelernt, entkoppelt gelernt, gealtert eingefroren
und untrainiert eingefroren sowie normale und umgekehrte Belegung. Die
ausgeführten Tasten bleiben beim Belegungswechsel identisch. Nur die erlebten
Handlungswirkungen kehren sich um. Weltzustände, Zukunftsbilder und Klassenlabels
bleiben außerhalb des neuronalen Actors.

Der Ratenverstärker wird einmal aus dem unveränderten Referenznetz des Seeds
berechnet und über sämtliche Bedingungen festgehalten. Andernfalls würde eine
Neunormierung F teilweise wieder aufheben. Das erwartete initiale Sensorstrommittel
beträgt ungefähr 10 pro Core/s, bei F=0,5 ungefähr 5 und bei F=1,5 ungefähr 15.
Die tatsächlich erzeugten visuellen Eingangsimpulse stimmen trotzdem über alle
13 Bedingungen überein. Unterschiede entstehen durch Gewichte und Dynamik.

Die höhere Dichte fügt zufällige lokale Kanten aus einer getrennten Zufallsfolge
nach dem bisherigen Grundgerüst hinzu. Alle ursprünglichen Zellen, Positionen,
Polaritäten und Anschlüsse bleiben erhalten. Neue Kanten liegen im Radius 0,25,
ohne Selbst- oder Doppelverbindung. Der tatsächlich erreichte Fan-out beträgt
bei E=8 **7–8**, bei E=12 **7–12**, entsprechend im Mittel 255,9 beziehungsweise
357,6 Core-Kanten gegenüber 128. Der Nachbarschaftsradius selbst wird nicht variiert.
Es gibt keine eigens vorgegebene A→links-Verbindung.

## Unmittelbare und unabhängige Ruheprobe

Die visuelle Kalibrierung des externen Auswerters läuft wie in M6 ausschließlich
auf einem exakten Replayzweig. Die laufende Elterninstanz bleibt eingefroren am
Trainingsendzustand. Die beiden primären Endpunkte sind:

- **Sofort-Checkpoint:** A und D aus jeweils einem eigenen exakten Replay desselben
  Trainingsendzustands.
- **Ruhe-Checkpoint:** A und D aus jeweils einem eigenen Replay dieses Zustands
  nach 2 Sekunden neutralem Bild, ohne vorangegangene Online-Testtrials.

Das sind jeweils zwei Beispiele pro Belegung und Seed. Die erste Prognose
benötigt 50 ms Startbild und 50 ms Aktionsintegration und liegt 99.999 µs nach
dem jeweiligen Checkpoint beziehungsweise dessen Ruhephase. Noch kürzere
vorübergehende Zustände bleiben außerhalb dieses Protokolls.

An beiden Zeitpunkten beginnt zusätzlich eine No-copy-Probe aus demselben
ausführbaren Zustand. Sie erhält dieselben neutralen Bilder und Zeitabschnitte,
aber keine Aktionskopie. Damit wird spontan weiterlaufende Aktivität von der
Wirkung des Aktionssignals getrennt. In allen **2.080** solchen Kontrollfenstern
der Bestätigungsreihe entstehen **null Core-Spikes**.

Sekundär werden 128 Online-Trials ab dem unveränderten Trainingsende, ihre
ersten 16 Trials sowie die bisherigen sequenziellen Ruhe- und Interferenzproben
ausgeführt. Diese sequenzielle Ruheprobe folgt auf die Online-Trials und wird
nicht mit dem unabhängigen Ruhe-Checkpoint vermischt. Der Auswerter wird nicht
neu angepasst; Lernparameter bleiben eingefroren.

## Ergebnis: elektrische Weiterleitung ist gezielt aktivierbar

Gekoppelt trainierter Arm, Mittelwerte über zehn weitere Seeds und beide Belegungen:

| Bedingung | Vorhersage sofort | Vorhersage nach 2 s Ruhe | Online-Vorhersage | Extra-Core-Spikes sofort | Extra-Core-Spikes nach Ruhe |
|---|---:|---:|---:|---:|---:|
| Referenz | 50,0 % | 50,0 % | 50,0 % | 0 | 0 |
| A: Core-Gewichte ×2 | 50,0 % | 50,0 % | 50,0 % | 0 | 0 |
| A: Core-Gewichte ×4 | 50,0 % | 50,0 % | 50,6 % | 19,48 | 19,05 |
| B: Schwelle 0,8 | 50,0 % | 50,0 % | 50,0 % | 0 | 0 |
| B: Schwelle 0,6 | 50,0 % | 52,5 % | 51,7 % | 0,13 | 0 |
| C: STDP-Tau 50 ms | 47,5 % | 47,5 % | 47,5 % | 0 | 0 |
| C: STDP-Tau 100 ms | 50,0 % | 50,0 % | 50,0 % | 0 | 0 |
| D: Verzögerung 0,5 ms | 52,5 % | 52,5 % | 52,5 % | 0 | 0 |
| D: Verzögerung 5 ms | 50,0 % | 50,0 % | 50,0 % | 0 | 0 |
| E: Fan-out höchstens 8 | 47,5 % | 47,5 % | 47,5 % | 0 | 0 |
| E: Fan-out höchstens 12 | 47,5 % | 47,5 % | 47,5 % | 0 | 0 |
| F: Visuelle Gewichte ×0,5 | 47,5 % | 47,5 % | 47,5 % | 0 | 0 |
| F: Visuelle Gewichte ×1,5 | 50,0 % | 50,0 % | 50,0 % | 0 | 0 |

Extra-Core-Spikes sind Spikes **außerhalb der drei direkt an die betreffende
Aktionskopie angeschlossenen Zellen**. Die visuelle Auslesbarkeit beträgt in
sämtlichen Bedingungen 100 %. Die No-copy-Aktivität ist jeweils null.

Bei vierfachen Core-Gewichten haben **36/40** Aktionsproben zusätzliche
Core-Spikes, sowohl unmittelbar als auch nach Ruhe. Die höhere Aktivität
entsteht also durch das eingespeiste Aktionssignal. Sie ist aber bereits ohne
gelernte Zuordnung vorhanden:

| Arm bei Core-Gewichten ×4 | Extra-Spikes sofort | Extra-Spikes nach Ruhe | Proben mit Extra-Spikes nach Ruhe |
|---|---:|---:|---:|
| untrainiert eingefroren | 19,70 | 19,70 | 36/40 |
| gealtert eingefroren | 19,33 | 19,70 | 36/40 |
| gekoppelt gelernt | 19,48 | 19,05 | 36/40 |
| entkoppelt gelernt | 20,20 | 19,55 | 36/40 |

Der bisherige Weiterleitungsengpass lässt sich somit mit vorhandener Dynamik
beheben. Die bloße zusätzliche Weiterleitung erklärt noch keinen Lernerfolg.
Zweifache Gewichte reichen hier nicht. Die Schwelle 0,6 erzeugt lediglich in
**5/40** unmittelbaren gekoppelten Proben je einen Extra-Spike; nach Ruhe
verschwindet dieser Befund. Ähnliche kurze Aktivität gibt es auch in gealtert
eingefrorenen und entkoppelten Kontrollen (4/40 beziehungsweise 6/40).

## Ergebnis: längere STDP-Zeitskalen verstärken die synaptische Zuordnung

Der aus M7 bekannte topologische Zuordnungswert summiert Gewichtsänderungen von
aktionsnahen Core-Zellen zu visuell angeschlossenen Zellen: zur erfahrenen Wirkung
positiv, zur Gegenwirkung negativ. Er wird extern berechnet und verändert keine
Verschaltung. Vergleich der C-Bedingungen bei identischen Anfangskanten:

| STDP-Tau | Mittlere Zuordnung im gekoppelten Arm | Differenz zur Referenz, deskriptives 95-%-Intervall |
|---|---:|---|
| 20 ms | +0,007322 | Referenz |
| 50 ms | +0,032556 | +0,025234 [ +0,017877; +0,032810 ] |
| 100 ms | +0,053964 | +0,046642 [ +0,033042; +0,060658 ] |

Die Zuordnung steigt auf etwa das **4,4- beziehungsweise 7,4-Fache** der Referenz.
Bei entkoppelten und eingefrorenen Erfahrungen ist der über beide Belegungen
gemittelte Wert null. Diese gepaarten Bootstrapintervalle sind deskriptiv und
nicht für die mechanistischen Mehrfachvergleiche korrigiert; alle zwölf
Parameterkontraste stehen im Datenaudit.

Das stärkt die Erklärung, dass die bisherige 20-ms-Zeitskala die späten
Handlungs-/Wirkungspaare nur schwach gewichtet. Es macht die Regel nicht allgemein
„richtig eingestellt“: Auch LTD wird verbreitert. Die visuellen Eingangsgewichte
behalten nach Training im Mittel nur **58,4 %** ihres Anfangswerts in der
Referenz, **49,4 %** bei 50 ms und **45,8 %** bei 100 ms. Trotz stärkerer
Zuordnung und vollständiger visueller Auslesbarkeit entsteht in diesen
C-Bedingungen keine zusätzliche rekurrente Spike-Weiterleitung.

Unter F=0,5 beziehungsweise F=1,5 bleiben im Mittel 45,2 % beziehungsweise
70,6 % der jeweiligen anfänglichen visuellen Gewichtssumme erhalten. Auch hier
bleibt die visuelle Auslesbarkeit 100 %. F prüft Startgewichte bei unveränderter
sensorischer Plastizität; es ist keine isolierte Ablation der sensorischen STDP.
Eine Übertragung dieses Befunds auf vollständige Pixelbilder ist nicht geprüft.

## Kein funktionaler Lernnachweis

Für jeden primären Zeitpunkt gelten separat die M6-Anforderungen: mindestens
zehn Seeds, ausreichende visuelle Auslesbarkeit, Vorhersage oberhalb Zufall und
aller drei Kontrollen in beiden Belegungen sowie Verbesserungen beim
Belegungswechsel und bei gleichzeitig richtigen Antworten für beide Belegungen.
Alle unteren Grenzen der gepaarten 95-%-Bootstrapintervalle müssen positiv sein.

Zusätzlich muss jeder erforderliche gerichtete Kontrast einen einseitigen
exakten gepaarten Vorzeichentest mit **p ≤ 0,05/26** bestehen. Die 26 entstehen
aus 13 Bedingungen und zwei primären Endpunkten; Bonferroni berücksichtigt diese
Familie. Die Kontrollkontraste innerhalb eines Endpunkts sind sämtlich erforderlich.
Gleichstände werden ausgelassen. Bei zehn Seeds ist diese Forderung streng und
begrenzt die Teststärke. Die exakten Tests prüfen gerichtete Seedunterschiede,
die Bootstrapintervalle beschreiben deren Mittelwert.

**Keine Bedingung besteht den funktionalen Vergleich**, auch nicht schon dessen
unkorrigierte vollständige Kontroll-/Belegungskriterien. Vereinzelte 52,5 % sind
kein belastbarer Vorteil. Ein positiver unmittelbarer Endpunkt würde nicht wegen
eines später negativen Ergebnisses verworfen; die Endpunkte bleiben getrennt.
Die sekundären Online-/Ruhe-/Interferenzproben liefern ebenfalls keinen Vorteil
gegenüber allen Kontrollen in beiden Belegungen.

Das Beispiel F=0,5 zeigt den Nutzen neuer Seeds: Im Drei-Seed-Pilot lag die
Vorhersage bei 58,3 %, in der Zehn-Seed-Reihe bei 47,5 %. Es wurde aufgrund des
Piloten weder ausgewählt noch nachjustiert. Die neuen Seeds bestätigen diesen
scheinbar günstigen Pilotmittelwert nicht.

## Bedeutung und Grenzen

M8 trennt erstmals gezielt zwei beeinflussbare Voraussetzungen: ausreichende
rekurrente Signalstärke und stärkere zeitliche Gewichtszuordnung. Ihre jeweilige
Verbesserung führt in diesem Versuch noch nicht zu funktionalem Wirkungslernen.
Es folgt daraus weder, dass die vorhandene STDP grundsätzlich ungeeignet ist,
noch dass zusätzliche Core-Spikes allein Wirkungswissen darstellen.

**Kombinationen der sechs Eingriffe wurden nicht untersucht.** Insbesondere wurden
stärkere Rekurrenz und längere STDP-Zeitskalen hier nicht gemeinsam optimiert.
Die Verteilung zusätzlicher Kanten bleibt lokal; andere räumliche Radien oder
Verbindungstypen wurden nicht getestet. Der topologische Zuordnungswert ist ein
Summenmaß über vorhandene Anschlusszellen und bei verschiedenen Dichten nicht
automatisch direkt vergleichbar. Diese Grenzen rechtfertigen keinen allgemeinen
Ausschluss funktionaler Selbstorganisation mit den bestehenden Mechanismen.

Die Ausführung bleibt im Experimentpaket. Das Modell erhält konfigurierbare
Parameter seiner vorhandenen Regeln und optionale Replay-/Weiterleitungsmessungen.
NERVA und der QEMU-Adapter bleiben unverändert. Es wurden keine neuen biologischen
Lernfunktionen und keine feste innere Aufgabenarchitektur eingeführt. Die
Standardwerte des laufenden VM-Modells bleiben erhalten.

Pläne, Parameter und Aufrufe: [m8/README.md](m8/README.md).
