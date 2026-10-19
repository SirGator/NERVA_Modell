# Räumlicher rekurrenter Core und erster Entwicklungsversuch

Das Standardmodell enthält jetzt **256 Processing-Neuronen** zwischen visueller
Sensorik und Motorik. NERVA liefert Dynamik, Pair-STDP, Homöostase, lokale
Kandidatensuche, Utility, Verbindungsbildung und Pruning. Modell konfiguriert
und verbindet diese Mechanismen. Die beiden Bibliotheken bleiben unabhängig.

## Ausführen

```bash
./NERVA_Modell/Modell/run-demo.sh demo
./NERVA_Modell/Modell/run-demo.sh demo --keyboard wasd
./NERVA_Modell/Experimente/run.sh study --steps 40 --seeds 7,19,42

# Dieselbe gespeicherte Bild-/Aktionsfolge, ohne laufende VM:
./NERVA_Modell/Experimente/run.sh study-replay Messungen/Experimente/recurrent-study \
  --output Messungen/Experimente/meine-wiederholung
```

`study` verwendet fünf Motorneuronen für W/A/S/D/Leertaste. `demo` und `attach`
behalten standardmäßig das vollständige Layout mit 105 Motorneuronen.
`--processing N` erlaubt 16 bis 1.024 Processing-Zellen; Standard ist 256.

Kontrollen für eigene Live-Läufe:

```bash
./NERVA_Modell/Modell/run-demo.sh demo --fixed-topology
./NERVA_Modell/Modell/run-demo.sh demo --no-feedback
./NERVA_Modell/Modell/run-demo.sh demo --no-homeostasis
./NERVA_Modell/Modell/run-demo.sh demo --frozen
./NERVA_Modell/Modell/run-demo.sh demo --model direct
```

`--fixed-topology` lässt STDP und Homöostase aktiv. `--frozen` deaktiviert STDP,
Homöostase und Entwicklung gemeinsam. Der frühere Feedforward-Aufbau bleibt
als explizite Kontrolle `--model direct` erhalten.

`--frozen` startet ohne Lernen. Ein **bereits trainiertes Modell** wird mit
`Model::freeze()` eingefroren; der neue Retention-Versuch und seine
Aktionskontrollen stehen in [RETENTION_EXPERIMENT.md](RETENTION_EXPERIMENT.md).
`--initial-drive 0` und `--pruning-min-us 5000000` erlauben gezielte Kontrollen
des künstlichen Anfangsbedarfs beziehungsweise der frühen Pruning-Frist.

## Minimales Grundgerüst

- Sensoren liegen auf einer visuellen Ebene bei z=-1,5. Vier geseedet gewählte
  Projektionen pro Sensor gehen zu räumlich nahen Processing-Zellen.
- Processing-Zellen liegen auf einem dreidimensionalen Raster mit Abstand 0,12
  und kleinem Seed-Jitter. 51 von 256 Zellen sind inhibitorisch. Jede Zelle
  erhält anfangs höchstens vier ausgehende rekurrente Verbindungen im Radius
  0,25: insgesamt 1.024, rund 1,6 % aller möglichen gerichteten Core-Paare.
- Motorneuronen liegen bei z=1,5. Jedes besitzt acht zufällige exzitatorische
  Core-Eingänge. Rohe motorische Spikes werden zu drei zufälligen Core-Zellen
  pro Motorneuron zurückgeführt.
- Eigene Aktionskopie-Zellen liegen bei z=1,2. Nach erfolgreicher Tastaturausgabe
  meldet der Host die gesendeten/gehaltenen Tasten an diese Zellen; ihre Spikes
  erreichen dieselben drei Core-Zellen. Das ist eigene Aktionsinformation,
  keine Soll-Antwort und kein internes Datum der VM.

Es gibt keine handkodierte A/D-Politik, keine vorgegebenen Sequenzketten und
keinen direkten visuellen Sensor→Motor-Pfad. Die Projektionen an den
Populationsgrenzen bleiben feste Anschlüsse. Entwicklung betrifft nur den Core.

## Strukturentwicklung

Jede Processing-Zelle beginnt mit derselben lokalen Wachstumsanforderung 2,5.
Alle 100 ms Simulationszeit prüft Modell die Processing-Ziele mit NERVAs
`IncomingGrowthDrive` und `local_candidates`. Der lokale Score kombiniert
Aktivität, zeitliche Korrelation und Distanz. Die Schwelle 0,01 verhindert
Bildung ohne ausreichenden Kandidatenscore. Pro Ziel kann in einem Schritt
höchstens eine neue eingehende Verbindung entstehen. Eine Bildung verbraucht
0,5 Wachstumsbedarf; die neue Verbindung startet mit Gewicht 0,25 und 1 ms Delay.

`ExcitatoryCausalUtility` und `PruningController` entfernen exzitatorische
Core-Verbindungen, deren Utility mindestens 300 ms unter 0,01 bleibt.
Inhibitorische Verbindungen sind von dieser Utility-Bewertung ausgenommen.
Der `DevelopmentPlan` validiert und übernimmt alle Änderungen atomar über die
Runtime; bereits fliegende Impulse entfernter Synapsen werden storniert.

Die neuronlokalen Homeostase-Uhren laufen unabhängig vom Host-Kontrollpunkt.
Die vorhandene NERVA-API aktiviert Homöostase für die gesamte Simulation;
die berichteten Homeostase-Zählungen beziehen sich auf Processing-Zellen.
Regelparameter und Zuständigkeitsgrenzen stehen in
[ARCHITECTURE.md](../../ARCHITECTURE.md).

## Messprotokoll

1. Der Host startet den visuellen QEMU-Gast und führt die Folge **A,A,D,D**
   wiederholt aus. Nach jeder Aktion wird das echte VGA-Bild gespeichert.
   Die Aufzeichnung enthält nur Pixel und die eigenen gesendeten Tasten.
2. Pro Seed werden zwei identische Anfangsnetze erstellt. Einziger
   Behandlungsunterschied ist, ob Entwicklungsschritte übernommen werden.
   STDP, Utility, Homeostase und beide Feedbackpfade bleiben in beiden aktiv.
3. Beide Modelle erhalten exakt dieselben Bilder und ausgeführten Aktionen.
   Neuronale Tastenvorschläge werden protokolliert, aber während dieser
   Messung nicht ausgeführt. So erzeugen die Varianten keine verschiedenen
   visuellen Lernfolgen.
4. Jeder vollständige Lauf wird wiederholt. Verglichen werden alle Schrittwerte,
   die Core-Spikes mit exakten Zeitstempeln und der abschließende vollständige
   `Network`-Zustand. Der Vergleich umfasst keine CPU/GPU-Abweichungen und
   keine separat gestarteten Echtzeit-VM-Läufe.

40 Bilder entsprechen zwei Sekunden Simulationszeit. Die reale Aufnahmezeit
und die Gastuhr sind davon unabhängig. Drei Seeds sind eine erste technische
Messung; sie begründen keine Aussage über andere Aufgaben oder Parameter.

## Erste Messung

Gemeinsame Aufzeichnung: [recurrent-study/recording.json](../../Messungen/Experimente/recurrent-study/recording.json).
256 Processing-Zellen, fünf Motorneuronen, 40 Bilder, Seeds 7, 19 und 42.

| Seed | Core-Spikes mit Entwicklung | Feste Topologie | Neue Core-Synapsen | Entfernte Core-Synapsen | Abweichende Neuron/Frame-Zählungen |
|---|---:|---:|---:|---:|---:|
| 7 | 3.192 | 2.549 | 169 | 760 | 741 |
| 19 | 4.275 | 2.704 | 202 | 723 | 1.119 |
| 42 | 4.136 | 2.675 | 211 | 713 | 1.160 |

Alle sechs Variantenläufe wurden jeweils exakt wiederholt. Beide Varianten
starteten pro Seed mit identischem Netz. Jeder Lauf erhielt 120 Ankünfte aus
Aktionskopien: drei pro aufgezeichneter Aktion. Die feste Kontrolle behielt
alle 1.024 rekurrenten Anfangssynapsen. In der Entwicklungsvariante blieben
433, 503 beziehungsweise 522 Core-Synapsen.

**Befund:** Strukturentwicklung verändert die Core-Aktivität in dieser
Aufzeichnung reproduzierbar. Mehr Spikes oder andere Muster bedeuten noch
keine bessere Aufgabenleistung und keinen Nachweis gelernter A/D-Wirkungen.
Auch die gespeicherten A/D-konditionierten Aktivitätsmittel können durch
unterschiedliche sichtbare Bilder entstehen. Der Versuch isoliert zunächst
den Einfluss der Strukturentwicklung auf die interne Dynamik.

## Dateien

- `recording.json` und `frames/`: wiederverwendbare VM-Bild-/Aktionsfolge.
- `study.json` und `study-summary.md`: Kontrollbedingungen und Vergleichswerte.
- `study-core.csv`: Spikezahl pro Processing-Zelle, Frame, Seed und Variante.
- `study-spikes.csv`: einzelne Core-Spikes mit exakten Simulationszeiten.
- `study-topology.csv`: abschließende synaptische Graphen und Gewichte.
- `study-structure.csv`: Bildung und Entfernung von Verbindungen pro Frame.

Live-Demos schreiben zusätzlich `core-activity.csv` und `structure.csv` neben
den bisherigen Tastatur-, Motor- und Bildprotokollen.

## Freie Aktionswahl

Im freien rekurrenten WASD-Lauf mit Seed 42 und 40 Bildern entstanden 4.353
Core-Spikes und 94 Motor-Spikes. Es gab 32 Schritte mit einer Taste, aber
**keine sichtbare Bildänderung**; die Auswahl war durch die Leertaste geprägt.
Der erfolgreiche gepaarte Entwicklungsvergleich bedeutet deshalb noch keine
brauchbare autonome Bewegungsstrategie. Dieser negative Befund bleibt im
[Live-Bericht](../../Messungen/Experimente/recurrent-wasd-live/report.json) erhalten.

`smoke` prüft im rekurrenten Modell Hardware-A/D-Rückwirkung, neuronale Aktivität
und Tastaturausgabe. Die Zahl der Bildänderungen wird berichtet, aber nicht als
Garantieleistung einer zufälligen Startverschaltung verlangt. Im bisherigen
direkten WASD-Kontrollmodell bleibt die Bewegungsprüfung bestehen.
