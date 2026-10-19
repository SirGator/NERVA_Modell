# M3: bekannte Handlung, späteres Wirkungsbild

Der Versuch prüft, ob die vorhandenen lokalen NERVA-Regeln aus ausgeglichenen
A/D-Erfahrungen eine vor dem Wirkungsbild auslesbare Handlungswirkung bilden.
Er läuft in `NERVA_Modell/Experimente/`; NERVA und seine Lernregeln bleiben unverändert.
Die kleine Rust-Welt verwendet dieselben Pixel- und Tastentypen wie QEMU.

```bash
# Aus DSVLM starten; keine VM erforderlich.
./NERVA_Modell/Experimente/run.sh m3

# Den dokumentierten Lauf in einem neuen Ordner wiederholen:
./NERVA_Modell/Experimente/run.sh m3 --jobs 10 --output Messungen/Experimente/mein-m3

# Technischer Kurztest, kein Lernnachweis:
./NERVA_Modell/Experimente/run.sh m3 --seed 7 --processing 16 \
  --train-trials 8 --probe-trials 8 --calibration-trials 8 \
  --retention-windows 2 --scan-repeats 4
```

Ergebnis: [Zehn-Seed-Bericht](../../Messungen/Experimente/m3-ten-seeds/m3-summary.md),
[Messwerte und gepaarte Intervalle](../../Messungen/Experimente/m3-ten-seeds/m3.json).

## Zeitfolge und Eingangsgrenze

Die ziel- und belohnungsfreie Welt hat sieben Positionen. Jeder Trial beginnt
mit demselben 64×64-Bild einer Figur in der Mitte. A/D bewegen um eine Position.
Die zweite Belegung kehrt beide Tastenwirkungen um. Das Netz erhält ausschließlich
Bilder und die Kopie der bereits ausgeführten eigenen Taste. Position, Belegung,
Wirkungsklasse, Auswertervorhersage und Sollaktion werden nicht eingespeist.

| Phase im 50-ms-Trial | Externe Ausführung | Neuronaler Eingang |
|---|---|---|
| 0–34 ms | Welt auf Mitte setzen | Identisches Startbild; keine neue Aktionskopie |
| 34–42 ms | A oder D wurde bereits ausgeführt | Weiterhin Startbild und eigene Tastenkopie |
| Grenze 42 ms | Vorhersage aus abgeschlossenem Core-Fenster berechnen | Noch kein Wirkungsbild |
| 42–50 ms | Wirkungsbild aufnehmen/präsentieren | Neues Bild, keine neue Tastenkopie |

Die Simulationszeit beginnt bei 1 µs. Fensterenden sind inklusive; beispielsweise
liegt der erste Vorhersageabschluss bei 42.000 µs und der neue Bildeingang bei
42.001 µs. `step_for` bewahrt Zustände, Zufallsströme, Refraktärzeiten und noch
laufende Ereignisse. Die vorhandene Strukturentwicklung wird weiterhin alle
100 ms geprüft, auch wenn kurze Phasen die Grenze überschreiten.

Jeder Viererblock enthält zwei A und zwei D. Während der Exploration werden
diese Aktionen kontrolliert ausgeführt; neuronale Tastenvorschläge werden
protokolliert, aber nicht ausgeführt. Dies ist keine autonome Aktionspolitik.
Die Aktionskopie sagt, was ausgeführt wurde; sie sagt nicht, welche Wirkung folgt.

## Gepaarte Kontrollen

Zehn Seeds, 32 Processing-Neuronen, je 128 Trainingstrials, 32 rein visuelle
Kalibriertrials und 64 balancierte Prüftrials. Für jede Belegung und beide vorab
festgelegten Dynamiken werden vier Modelle mit exakt gleichem Anfangsnetz verglichen:

| Zustand | Training |
|---|---|
| `before_training` | Keine Trainingsphase; von Anfang an eingefroren |
| `aged_frozen` | Gleiche gekoppelte Bilder und Aktionen wie das lernende Modell, ohne Plastizität |
| `paired` | Zeitlich gekoppelte Aktionen und ihre Wirkungsbilder; vorhandene Lernregeln aktiv |
| `scrambled` | Lernen aktiv, aber Aktion/Bild-Zuordnung während des Trainings zerstört |

`aged_frozen`, `paired` und `scrambled` erhalten exakt dieselbe Aktionsfolge und
Fensterlängen. Im zerstörten Training enthält jeder Viererblock A/links, A/rechts,
D/links und D/rechts genau einmal. Beide Randverteilungen bleiben ausgeglichen;
die Aktionskopie verrät nicht mehr das folgende Bild. Dieser experimentelle
Eingriff ersetzt das gezeigte Bild; die tatsächlich ausgeführte Aktion bleibt
unverändert. Im Test zeigen alle Modelle wieder die echte jeweilige Wirkung.

Nach Training wird die laufende Instanz eingefroren. Die Parameter werden nach
Kalibrierung, unmittelbarer Prüfung, zwei Sekunden ohne Aktionen und erneuter
Prüfung auf Erhaltung kontrolliert. Die zweite Prüfung verwendet denselben
Auswerter und eine andere balancierte Aktionsreihenfolge, ohne erneutes Anpassen.

Die zwei Dynamiken sind das bisherige Netz und Core→Motor-Gewichte ×0,5.
Diese Bedingungen stehen vor der Messung fest; der Parameterscan wählt keine
Behandlung nachträglich anhand ihrer Erfolgswerte aus.

## Was die externe Auswertung prüft

Der primäre Auswerter wird ausschließlich auf Core-Reaktionen auf linke/rechte
**Bilder ohne Aktionen** kalibriert. Ein standardisierter Nächste-Zentroid-Auswerter
prüft anschließend, ob der Zustand nach bekannter Aktion und vor dem Wirkungsbild
bereits eher dem linken oder rechten visuellen Wirkungszustand entspricht.
Gemessen werden Spikezählungen sowie acht Zeitbins pro Core-Neuron, bei der
Standardphase jeweils 1 ms breit. Beide Klassen sind ausgeglichen; Zufallsniveau
ist 50 %. Der Auswerter wirkt niemals auf Neuronen, Gewichte oder Aktionen zurück.

Ein zusätzlicher überwacht trainierter Aktionsfenster-Auswerter steht ausdrücklich
nur als Referenz im JSON. Er kann bereits aus unverändertem A/D-Feedback eine
externe Tastenzuordnung lernen. Eine hohe Leistung dieses Auswerters allein
belegt deshalb kein in NERVA gelerntes Wirkungswissen.

Für die Primärauswertung werden alle visuellen Kalibrierbeispiele genutzt; eine
getrennte chronologische Halbierung prüft zunächst ihre visuelle Auslesbarkeit.
Prüfvorhersagen werden im Code vor der Aufnahme/Präsentation des neuen Bildes
berechnet. Diese Reihenfolge und die Ankunft der Aktionskopien werden geprüft.

Die konservative M3-Bedingung verlangt mindestens zehn Seeds, einen positiven
gepaarten Vorteil gegenüber **allen drei Kontrollen**, für beide Belegungen und
beide Prüfzeitpunkte. Die unteren Grenzen der 95-%-Bootstrapintervalle über
Seeds müssen jeweils über null liegen. Zusätzlich muss die mittlere visuelle
Validierung des trainierten Modells mindestens 80 % erreichen. Das ist eine
Arbeitsdefinition für diesen Versuch, kein allgemeiner Kausalitätsbeweis.

## Gemessene Ergebnisse

Im vollständigen Lauf wurden 160 Modelle geprüft: zehn Seeds × zwei Dynamiken ×
zwei Belegungen × vier Zustände. Es wurden 35.840 Trainings-/Prüftrials protokolliert.
Alle hatten dasselbe Startbild, die drei erwarteten Aktionskopie-Ankünfte vor dem
Bild und eine Vorhersagezeit vor dem Wirkungsbild. Die eingefrorenen Parameter
blieben erhalten. In den 40 gekoppelt lernenden Instanzen wurden insgesamt
17.306.748 Gewichtsänderungsereignisse, 5.103 neue und 39 entfernte Verbindungen
registriert. Eine Änderungsanzahl ist kein Lernqualitätsmaß.

Balancierte Genauigkeit, Mittel über Seeds und beide Belegungen:

| Dynamik | Merkmale | Prüfung | Vor Training | Gleich alt, eingefroren | Gekoppelt gelernt | Zuordnung zerstört |
|---|---|---|---:|---:|---:|---:|
| Bisherig | Zählung | Sofort | 49,7 % | 50,0 % | 49,6 % | 51,0 % |
| Bisherig | Zeitbins | Sofort | 48,4 % | 52,5 % | 50,9 % | 49,7 % |
| Gewichte ×0,5 | Zählung | Sofort | 49,5 % | 50,8 % | 50,0 % | 51,3 % |
| Gewichte ×0,5 | Zeitbins | Sofort | 50,2 % | 49,4 % | 51,9 % | 51,3 % |
| Bisherig | Zählung | Nach Pause | 48,5 % | 50,3 % | 50,8 % | 46,9 % |
| Bisherig | Zeitbins | Nach Pause | 50,2 % | 49,2 % | 49,9 % | 47,6 % |
| Gewichte ×0,5 | Zählung | Nach Pause | 47,7 % | 52,0 % | 52,1 % | 50,1 % |
| Gewichte ×0,5 | Zeitbins | Nach Pause | 48,7 % | 50,9 % | 51,6 % | 46,9 % |

**M3 bleibt offen.** Keine Bedingung erfüllt die Kriterien. Sämtliche einzelnen
Vergleiche mit dem gleich alten eingefrorenen Netz haben Bootstrapintervalle,
die null einschließen. Die visuellen Zählungs-Auswerter erreichen vor Training
92,2 %, bei gleich alten eingefrorenen Netzen 93,9 %, nach gekoppeltem Training
nur 68,0 %; bei zerstörter Zuordnung 65,9 %. Bei Zeitbins sind es nach gekoppeltem
Training nur 52,5 %. Diese schwache visuelle Validierung begrenzt den Test
zusätzlich: Das Ergebnis zeigt keinen Vorteil, beweist aber nicht, dass jede
mögliche interne Darstellung von Handlungswirkungen fehlt.

## Kleiner Motor-Parameterscan

Acht vorab festgelegte Einstellungen, je Seed 56 Bilder: acht Wiederholungen der
sieben Positionen in zufälliger Reihenfolge, identisch zwischen Einstellungen.
Die Modelle bleiben eingefroren; es gibt keine ausgeführten Aktionen oder
Aktionskopien. Die erste Hälfte kalibriert einen siebenklassigen Auswerter,
die zweite prüft bisher unbenutzte Beispiele. Motor-Zeitmerkmale sind zehn
5-ms-Bins für A und D. Zufallsniveau bei der Positionserkennung ist 14,3 %.

| Einstellung | Motor-Spikes/Bild | Eingänge während Refraktärphase | Tastenauswahl | Position aus Zählung | Position aus Zeitbins |
|---|---:|---:|---:|---:|---:|
| Bisherig | 7,96 | 77,7 % | 96,4 % | 16,4 % | 17,1 % |
| Gewichte ×0,25 | 5,14 | 47,5 % | 99,1 % | 18,6 % | 16,4 % |
| Gewichte ×0,5 | 6,88 | 66,0 % | 97,5 % | 18,2 % | 14,3 % |
| Refraktärzeit 5 ms | 12,94 | 64,0 % | 97,1 % | 18,6 % | 15,0 % |
| Refraktärzeit 20 ms | 4,44 | 87,5 % | 97,1 % | 16,1 % | 15,0 % |
| Inhibition 0,5 | 7,89 | 77,0 % | 98,0 % | 18,2 % | 13,6 % |
| Inhibition 2 | 7,75 | 76,0 % | 97,7 % | 15,0 % | 12,1 % |
| Inhibition 4 | 7,49 | 73,3 % | 98,9 % | 17,9 % | 16,1 % |

Weniger motorische Sättigung ist messbar, besonders bei Viertelgewichten.
Die Auslesbarkeit bleibt in diesem kleinen deskriptiven Scan nahe dem
Zufallsniveau. Die Prozentwerte belegen keinen statistisch abgesicherten
Gewinner; weder hohe Aktionsrate noch geänderte Spikezahl bedeuten mehr nutzbare
Information. Die Standardgewichte und Lernregeln wurden deshalb nicht verändert.

## Dateien, Tests und Grenzen

`src/m3/mod.rs` führt die Phasen und Kontrollen aus; `readout.rs` enthält die
externe Auswertung, `scan.rs` den Motor-Scan, `m3_main.rs` das getrennte Programm.
Gespeichert werden `m3.json`, `m3-summary.md`, `m3-trials.csv` mit Zeitfolge und
Vorhersagen, `m3-spikes.csv` mit Core-Spikes sowie `motor-scan.csv`.

Tests prüfen exakte Aktionsfolge trotz zerstörter Zuordnung, vollständige
Kontingenzbalance, unveränderte Nicht-Motor-Projektionen im Gewichtsscan,
Ereigniserhaltung über kurze Phasen, zeitliche Eingangsgrenze, Freeze und bytegleiche
CLI-Wiederholungen. Eine separate vollständige Wiederholung von Seed 7
(`runs/m3-seed7-replay`) ist in sämtlichen fünf Ergebnisdateien bytegleich zu
`runs/m3-ten-seeds/worker-07`. Alle 37 Modell-/Experimenttests, 20 Adaptertests und vier
explizit ausgeführten QEMU-Integrationstests bestanden; Clippy und Formatprüfungen
sind fehlerfrei. Der Timingdecoder wird auch mit Funktionstasten, Umlauten,
Modifiern, Stille und exakt gleichzeitigen ersten Spikes geprüft.

Dies ist ein einzelner Zeitmaßstab und ein einfacher linearer Ähnlichkeitsauswerter.
Erlernte Darstellung muss nicht die Form einer visuellen Reaktivierung besitzen.
Kurze Phasen behalten den 50-ms-Rückblick des Tastendecoders; für M3 werden die
Tastenvorschläge ignoriert. Der Versuch belegt weder autonome Zielverfolgung
noch deren Unmöglichkeit. Die nächsten Untersuchungen können sich gezielt den
vorhandenen Lernregeln und ihrer zeitlichen Signalverarbeitung widmen.

Die VM-Regression deckte außerdem Strg+Alt+Entf aus neuronaler Tastaturaktivität
auf. Die eigene Demo-QEMU-Instanz erlaubt jetzt Gastneustarts, statt sich durch
`-no-reboot` dabei zu beenden. Tastencodes werden dafür nicht gefiltert.
