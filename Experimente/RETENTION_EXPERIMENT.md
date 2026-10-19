# Trainieren, laufendes Modell einfrieren und prüfen

`Model::freeze()` deaktiviert am **bestehenden Modell** STDP, Utility-Updates,
Homöostase und Strukturentwicklung. Gewichte, Verbindungen, homöostatische
Parameter, Membranzustände, Zufallsfolge, wartende Impulse und Aktionskopien
bleiben erhalten. Die Simulation verarbeitet weiterhin Eingänge und erzeugt
Aktionen. Der Aufruf ist idempotent.

`ModelConfig::frozen()` und `--frozen` erzeugen dagegen weiterhin ein **von Anfang
an** eingefrorenes Modell. Sie ersetzen keinen Retention-Test.

## Ausführen

Vom Projektverzeichnis aus:

```bash
# Separate Aufnahme mit einer anderen A/D-Folge; Standardfolge ist A,D,D,D,A,A,D,A.
./NERVA_Modell/Experimente/run.sh record --steps 64 --output Messungen/Experimente/meine-pruefbilder

# 1.000 Trainingsfenster, anschließend Einfrieren und Prüfen ohne laufende VM.
./NERVA_Modell/Experimente/run.sh retention Messungen/Experimente/recurrent-study Messungen/Experimente/meine-pruefbilder \
  --train-frames 1000 --seeds 42 --output Messungen/Experimente/meine-retention

# Kontrollparameter, auch für demo, study und study-replay:
./NERVA_Modell/Experimente/run.sh study-replay Messungen/Experimente/recurrent-study --seeds 42 \
  --initial-drive 0 --output Messungen/Experimente/mein-start-ohne-wachstumsbudget
./NERVA_Modell/Experimente/run.sh study-replay Messungen/Experimente/recurrent-study --seeds 42 \
  --pruning-min-us 5000000 --output Messungen/Experimente/meine-pruning-frist
```

`record --actions A,D,A,A,D,D` erlaubt eine eigene, zyklisch ausgeführte A/D-Folge.
Die Aufnahme enthält ausschließlich Bildpixel und eigene ausgeführte Tasten.
Manifestversion 2 unterstützt beliebige A/D-Folgen; Version 1 und das bisherige
strenge A,A,D,D-Studienprotokoll bleiben lesbar und unverändert validiert.

Training und Prüfung benötigen jeweils mindestens acht Aufnahmeframes, die
Prüfung mindestens 16. Die Prüfung muss mindestens eine Vierer-Aktionsfolge
enthalten, die im zyklischen Trainingsstrom nicht vorkommt. Das garantiert
neue Aktionsfolgen, keine neuen visuellen Zustände. Alle Modelle verwenden
denselben Seed und dieselbe Anfangsverschaltung. Standard sind 256
Processing-Zellen, WASD und Seeds 7,19,42; `--seeds 42` begrenzt den Versuch
auf einen Seed. Das volle Tastaturlayout bleibt in der Live-Demo verfügbar.

## Bedingungen und Kontrollen

Vier Bedingungen werden verglichen:

| Bedingung | Trainingsphase | Zeitpunkt des Einfrierens |
|---|---|---|
| `before_training` | Keine | Direkt nach Aufbau |
| `aged_frozen` | Dieselben Eingänge über 1.000 Frames ohne Plastizität | Vor dem ersten Frame |
| `trained_fixed` | STDP, Utility und Homöostase, feste Verbindungen | Nach Frame 1.000 |
| `trained_development` | Zusätzlich Wachstum und Pruning | Nach Frame 1.000 |

Pro Bedingung gibt es sechs Prüfzweige:

| Prüfzweig | Bilder | Eingespeiste eigene Aktionskopie |
|---|---|---|
| `aligned` | Separate Prüfaufzeichnung | Aufgezeichnetes A/D |
| `swapped` | Dieselben Bilder in derselben Reihenfolge | A und D vertauscht |
| `delayed_one_frame` | Dieselben Bilder in derselben Reihenfolge | Vorherige Aktion, erster Frame ohne Kopie |
| `absent` | Dieselben Bilder in derselben Reihenfolge | Keine Kopie |
| `constant_image_a` | Erstes Prüfungsbild ständig wiederholt | Immer A |
| `constant_image_d` | Dasselbe ständig wiederholte Bild | Immer D |

Die Kontrollen verändern gezielt die Aktionskopien. Das rohe neuronale
Motorfeedback bleibt aktiv. Neuronale Tastenvorschläge verändern während des
Versuchs keine VM-Bilder. Deshalb ist dies ein kontrollierter Replay-Versuch.

Jeder Prüfzweig wiederholt den identischen Trainingspräfix und friert anschließend
**dieselbe laufende Instanz** ein. So müssen weder Scheduler noch Zufallszustand
aus einem Gewichts-Export rekonstruiert werden. Die vollständigen `Network`-Zustände
vor dem Einfrieren werden zwischen allen sechs Präfixen exakt verglichen. Das
Einfrieren selbst muss den vollständigen Netz-Zustand unverändert lassen.
Die mehrfach ausgeführten Trainingspräfixe erhöhen die Rechenzeit.

In jedem Prüfframe wird geprüft, dass folgende Größen erhalten bleiben:

- Alle synaptischen Identitäten, Endpunkte, Gewichte, Delays und Flags.
- Gespeicherte STDP-Traces, Utility und Eligibility einschließlich Zeitankern.
- Neuronengeometrie, elektrische Parameter, Basisschwellen, intrinsische Ströme
  und struktureller Drive, für **alle** Populationen.
- Keine WeightChanged-/HomeostasisChanged-Ereignisse und kein Wachstum/Pruning.

Membranen, Spike-/Eingangstraces und Übertragungszähler entwickeln sich weiter.
Eine zeitabhängige, nur lesende Projektion der Utility kann weiterhin zerfallen;
gespeicherte Utility-Werte erhalten keine neuen Lernupdates und Pruning ist aus.

## Messungen

Die Core-Zählungen und exakten Spikezeiten werden gespeichert. Für die
Aktionskontrollen werden geänderte Neuron/Frame-Zählungen und ihre L1-Differenz
zur passenden `aligned`-Bedingung berichtet. Der Festbildvergleich misst
A/D-Unterschiede bei identischen visuellen Eingängen. Solche Unterschiede
können bereits aus der zufälligen, festen Feedbackverschaltung entstehen.

Eine externe Auswertung untersucht die nächste visuelle Änderung:

1. Bilder werden unabhängig vom neuronalen Modell auf 16×16 Graustufen gemittelt.
   Die nächste Bilddifferenz wird auf Schritte von 1/255 quantisiert.
2. Jede unterschiedliche Differenz bildet eine Zielklasse. Merkmale sind die
   aktuellen Spikezahlen pro Core-Neuron; zukünftige Bilder, nächste Aktionen
   und Frameindex werden nicht als Merkmale benutzt.
3. Auf der ersten Hälfte der Prüfsequenz wird pro Klasse ein Mittelwert der
   Merkmale bestimmt. Die zeitlich spätere Hälfte wird anhand des nächsten
   Klassenmittelwerts zugeordnet. Im Test erstmals auftretende Zielklassen
   zählen als Fehler. Klassenanzahl, unbekannte Testklassen und Treffer werden
   mitgespeichert.
4. Dieselbe Auswertung mit ausschließlich aktuellen Bildpixeln und eine
   Mehrheitsvorhersage dienen als Kontrollen. Ein zusätzlicher Readout misst,
   ob die aktuell aufgezeichnete A/D-Aktion aus dem Core unterscheidbar ist.

Diese Kalibrierung trainiert ausschließlich den externen Auswerter; keine
Labels, Fehler oder Belohnungen gelangen in NERVA. Sie misst diagnostische
Vorhersagbarkeit auf eingefrorenen Aktivitätsmustern, keine autonome
Aufgabenleistung. Bei vertauschten Aktionskopien beziehen sich die Labels weiter
auf die ursprüngliche Aufzeichnung. Die nächste Aktion ist nicht vorgegeben;
auch perfekte Repräsentation einer früheren Aktion garantiert deshalb keine
Vorhersage ihrer noch unbekannten Nachfolgeaktion.

## Grenzen und Parameterkontrollen

1.000 Frames bedeuten 50 Sekunden simulierte Eingangsfenster. Eine kürzere
Aufzeichnung wird ausdrücklich **wiederholt**, nicht als neue Erfahrung gezählt.
Beim vorhandenen 40-Frame-Datensatz sind das 25 Wiederholungen von zwei Sekunden
einmalig aufgenommener Erfahrung. Training und Prüfaufnahme werden mit eigenen
Aktionslisten und stabilen Bild-Fingerprints im Bericht identifiziert.

Parametererhalt ist ein technischer Retention-Nachweis. Er belegt noch keinen
stabilen funktionalen Vorteil. Vorher/Nachher-Unterschiede können auch aus
Membranzuständen und Eingangshistorie entstehen. Die zeitlich passende
`aged_frozen`-Bedingung kontrolliert einen Teil davon. Für autonome Leistung
wären ein definiertes Aufgabenziel und freie Modellaktionen nötig; der bisherige
VGA-Gast besitzt kein Erfolgskriterium.

`--initial-drive 0` entfernt das vorgegebene Startbudget, lässt homöostatische
Bedarfsentwicklung aber zu. Standard bleibt 2,5. Die Wachstumsschwelle ist 0,5.
`--pruning-min-us 5000000` verlangt fünf Sekunden unter der Utility-Schwelle;
Standard sind 300.000 µs. Dies ist keine globale Schonfrist: Die Frist gilt
pro Synapse nach deren Utility-Verlauf. Die NERVA-Uhr ist ereignisbasiert; bei
vollständiger Stille ohne Homeostase-Uhren verstreicht durch weitere Aufrufe von
`run_until` allein keine neue Ereigniszeit. Kontrollläufe müssen deshalb eine
vergleichbare Ereignisfolge behalten.

## Dateien

- `retention.json`, `retention-summary.md`: Bedingungen, Messwerte und Grenzen.
- `retention-core.csv`, `retention-spikes.csv`: alle Prüfzweige, Zählungen und Zeiten.
- `retention-training.csv`: Verlauf des identischen Trainingspräfixes, einmal
  pro Bedingung gespeichert; enthält auch den frühen Pruning-Verlauf.
- `retention-topology.csv`, `retention-neurons.csv`: Parameter beim Einfrieren.
  Dies sind Diagnoseexporte, keine wieder ladbaren Scheduler-Checkpoints.

Die reproduzierbare Wiederholung erfolgt aus Aufnahme und Konfiguration.
Normale Tests prüfen Freeze nach echtem Training, Parametererhalt, Aktionskontrollen,
Pruning-Fristen und Readout-Trennung. Ein explizit aktivierter QEMU-Test prüft
Aufnahme und Retention-CLI einschließlich einer Wiederholung im separaten Prozess.

## Gemessener Lauf mit Seed 42

Gespeichert unter [retention-1000/](../../Messungen/Experimente/retention-1000/retention-summary.md):
256 Processing-Zellen, 1.000 Trainingsframes aus 25 Wiederholungen der bisherigen
40 Bilder und [64 separat aufgenommene Prüfungsbilder](../../Messungen/Experimente/retention-probe/recording.json).
Die Prüfaufnahme enthält 51 Viererfenster, die nicht im Trainingsstrom vorkommen.

| Bedingung | Core-Synapsen beim Einfrieren | Richtige Vorhersagen der nächsten Bildänderung |
|---|---:|---:|
| Vor dem Training | 1.024 | 15/32 |
| 1.000 Frames ohne Plastizität | 1.024 | 14/32 |
| Trainiert, feste Topologie | 1.024 | 14/32 |
| Trainiert, Strukturentwicklung | 476 | 15/32 |
| Ausschließlich aktuelle Bildpixel | – | 14/32 |
| Mehrheitskontrolle aus der Kalibrierung | – | 2/32 |

Die Entwicklungsvariante bildete 256 Verbindungen und entfernte 804; bereits
704 Entfernungen lagen in den ersten acht Frames (400 ms). Alle sechs Prüfzweige
behielten die gelernten Parameter unverändert. Die Aktionskopien kamen an:
192 Ankünfte bei ausgerichteten/vertauschten Kopien, 189 bei Verzögerung,
null bei fehlenden Kopien. Trotzdem blieben beim **trainierten** Modell sämtliche
Core-Spikefolgen inklusive Zeitstempeln unter diesen vier Aktionsbedingungen
identisch. Auch A gegenüber D bei konstantem Bild erzeugte keine Unterschiede.
Das wurde zusätzlich direkt aus `retention-spikes.csv` verglichen.

Damit ist die Retention-Schnittstelle technisch nachgewiesen. Ein funktionaler
Lernvorteil oder eine gelernte Nutzung der Aktionskopien ist für diesen Seed und
diesen Versuch **nicht nachgewiesen**. Die 15/32 Treffer nach Entwicklung
erreichen lediglich den Wert des untrainierten Modells. Eine Verbesserung um
einen Treffer gegenüber der Pixelkontrolle wäre bei 32 Testbeispielen ohnehin
keine belastbare allgemeine Aussage.

Kurze Parameterkontrollen mit derselben 40-Frame-Aufzeichnung:

| Bedingung | Neue Core-Synapsen | Entfernte | Verbleibende | Core-Spikes |
|---|---:|---:|---:|---:|
| Bisheriger Standard, Drive 2,5 / 300 ms | 211 | 713 | 522 | 4.136 |
| [Drive 0 / 300 ms](../../Messungen/Experimente/drive-zero-audit/study-summary.md) | 0 | 717 | 307 | 2.675 |
| [Drive 2,5 / 5 s](../../Messungen/Experimente/pruning-grace-audit/study-summary.md) | 214 | 0 | 1.238 | 4.118 |

Ohne Startdrive blieben die exakten Core-Spikefolgen trotz 717 Entfernungen
identisch zur festen Kontrolle. Der bisherige frühe Aktivitätsunterschied hängt
hier also mit dem angestoßenen Wachstum zusammen; die Anzahl entfernter Synapsen
allein beschreibt keinen funktionalen Verlust. Zwei Sekunden ohne Wachstum
belegen noch nicht, dass später kein homöostatisch ausgelöstes Wachstum entsteht.
