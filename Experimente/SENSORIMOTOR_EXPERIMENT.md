# Motorische Kontrolle und Wirkungslernen getrennt prüfen

Der Versuch führt die vorgeschlagenen Tests A–D und einen eigenen
Wirkungsversuch **ohne Zielmarker** aus. Neuronale Regeln in `NERVA/` bleiben
unverändert. Der Netzaufbau liegt in `NERVA_Modell/Modell/`, Aufzeichnung und Auswertung in `NERVA_Modell/Experimente/`;
der Adapter liefert Bilder, Tasten und die experimentellen Decoder.

```bash
# Zehn Seeds, 32 Processing-Zellen; vier parallele Worker.
./NERVA_Modell/Experimente/run.sh sensorimotor

# Vollständigen gemessenen Lauf in einem neuen Verzeichnis wiederholen:
./NERVA_Modell/Experimente/run.sh sensorimotor --jobs 10 --output Messungen/Experimente/mein-motortest

# Kurzer technischer Lauf, kein Zehn-Seed-Nachweis:
./NERVA_Modell/Experimente/run.sh sensorimotor --seed 7 --processing 16 \
  --explore-steps 16 --probe-steps 16
```

Standard sind 64 eigene Explorationsaktionen und 64 eigene Prüfaktionen pro
Seed und Variante. Alle acht Aktionen wird die Welt auf eine neue Position
zurückgesetzt; das Netz läuft weiter. Die Folge enthält auch eine Beobachtung
nach der letzten Aktion, also 72 Bilder pro 64 Aktionen. Die Welt hat sieben
Positionen, aber **kein Ziel und keine Belohnung**. Es gibt ausschließlich
A/D und die Möglichkeit, keine Taste zu drücken. Startpositionen werden nur
dem Renderer übergeben; NERVA erhält allein Pixel und eigene zuvor gesendete
Tasten.

Zum Zeitpunkt dieses historischen Versuchs verwendeten `demo` und `agency`
den Zähldecoder. Seit dem [M3-Folgeversuch](M3_EXPERIMENT.md) ist dort Timing
Standard; `--motor-decoder count` wählt die frühere Regel. Die vier Varianten
dieses Versuchs wählen weiterhin ihre hier benannten Regeln explizit. Die neuen
Varianten sind explizite Experimente. Ein erfolgreicher Motorvergleich wird
nicht als neue Zielerreichungsrate ausgegeben.

## Test 1: Motorische Kontrolle

| Variante | Änderung |
|---|---|
| `count` | Bisheriger Zähldecoder; Gleichstand und Stille bedeuten keine Taste |
| `count_then_first_spike` | Bei nichtleerem Zählgleichstand gewinnt der frühere erste Spike; bei identischem ersten Zeitpunkt weiterhin keine Taste |
| `inhibitory_competition` | Bisheriger Zähldecoder plus zwei symmetrische inhibitorische Vermittler |
| `random_tie_control` | Ausschließlich bei nichtleerem Zählgleichstand zufälliges A/D |

Die Timingregel ersetzt keine eindeutigen Zählentscheidungen. Sie bevorzugt
keine bestimmte Taste und erhält keine Informationen über die Umgebung.
Der Zufallsdecoder erzeugt bei neuronaler Stille ebenfalls keine Aktion.
Seine Zufälligkeit ist nur eine Kontrolle, kein als Lernen ausgegebenes Verhalten.

Für den direkten Vergleich wird eine gemeinsame Bild-/Aktionsfolge aus der
eigenständigen Count-Exploration aufgezeichnet. Alle vier Netze werden auf
dieser Folge trainiert und nach Freeze auf denselben Prüfbildern verglichen.
Count, Timing und Random-Tie müssen dabei **exakt gleiche neuronale Ereignisse**
liefern: Nur die Auswertung dieser Ereignisse ändert sich. Die hier vorgeschlagenen
Decoderaktionen werden während des gemeinsamen Replays nicht ausgeführt.
Sichtbare Bewegung wird separat in ihren eigenen geschlossenen Schleifen gemessen.

Die Konkurrenz benutzt vorhandene exzitatorische/inhibitorische LIF-Dynamik:

```text
Motor A → inhibitorische Zelle A ─| Motor D
Motor D → inhibitorische Zelle D ─| Motor A
```

Die ursprünglichen Neuronen, Synapsen, Gewichte und Zufallsinitialisierung
bleiben unverändert. Es kommen zwei inhibitorische Neuronen und vier
nichtplastische Verbindungen hinzu. Anregung des Vermittlers: Gewicht 1,2;
Hemmung des anderen Motors: standardmäßig 2,0; je Verbindung 500 µs Delay.
`--inhibition-weight` erlaubt einen expliziten Parameterscan. Die vorhandene
globale Homöostase gilt auch für diese zusätzlichen Zellen. Sie gehören nicht
zur entwicklungsplastischen Core-Population. Symmetrische Hemmung ist ein
zu prüfender Mechanismus, keine garantierte Winner-Take-All-Lösung.

Test D regt A und D jeweils einzeln an, außerdem beide gleichzeitig und keinen
Motor. Das geschieht in separaten, frisch aufgebauten, eingefrorenen Netzen bei
schwarzem Bild und ausgeschaltetem Motorfeedback. `diagnostic_motor_input`
verweigert die Benutzung bei aktivem Lernen oder Feedback. Diese Stimuli gelangen
nie in Explorations- oder Lernfolgen. Geprüft werden Kanal, Decoder und sichtbare
Bewegungsrichtung in der Rust-Welt. Der bisherige reale QEMU-Tastaturpfad wird
zusätzlich durch die bestehenden VM-Regressionstests geprüft.

## Test 2: Eigene Erfahrung und Wirkungszuordnung

Jede Variante sammelt zunächst **ihre eigenen** Bild-/Aktionsfolgen. Die Aktionen
stammen aus ihren neuronalen Motorpulsen und dem benannten Decoder. Anschließend
wird das laufende Modell eingefroren und eine separate Folge durch eigene
Aktionen erzeugt. Sie enthält keine vorgegebenen richtigen Handlungen.

Diese zweite Folge dient als gemeinsame Prüfaufzeichnung für drei Zustände:

1. Vor Training eingefroren.
2. Von Anfang an eingefroren, aber mit genau denselben Trainingsbildern und
   ausgeführten Tasten bis zum gleichen Alter versorgt.
3. Auf dieser eigenen Erfahrung trainiert, anschließend im laufenden Zustand
   eingefroren. Der wiederholte Trainingspräfix muss den vollständigen
   gespeicherten `Network`-Zustand exakt reproduzieren.

Jeder Zustand erhält die gleichen Prüfbilder mit vier Aktionskopie-Kontrollen:
ausgerichtet, A/D vertauscht, um einen Beobachtungsschritt verzögert oder entfernt.
Bei Episodenstarts gibt es keine Aktionskopie des vorherigen Weltzustands.
Membranen und Zufallsfolge werden durch das Einfrieren nicht zurückgesetzt.
Alle gelernten Parameter bleiben während der Prüfung erhalten.

Zielgröße der externen Vorhersage ist die Richtung der **folgenden sichtbaren
Bildänderung**, gemessen ausschließlich aus den Pixeln: links, still, rechts.
Eine andere Auswertung liest die bereits erfolgte Bildbewegung aus dem aktuellen
Core-Zustand. Das ist Nachlesen einer Wirkung und wird getrennt berichtet.

Die externe Auswertung verwendet Klassenmittelwerte aus der ersten Hälfte der
Prüfaktionen und bewertet die zeitlich spätere Hälfte. Core-Merkmale sind entweder
Spikezahlen pro Neuron oder zehn 5-ms-Zeitbins pro Neuron. Absolute Simulationszeit,
zukünftige Bilder und die nächste ausgeführte Aktion sind keine Core-Merkmale.
Labels und Vorhersagefehler werden nicht an NERVA zurückgegeben.

Kontrollen sind aktuelle Bildpixel allein, Mehrheitsvorhersage und die tatsächliche
ausgeführte Aktion allein. Letztere ist ein **externer Referenzwert**, den der
Core beim Vorhersagezeitpunkt noch nicht als Aktionskopie erhält. Sie ist deshalb
keine symmetrische neuronale Eingangsbedingung.

Neben Trefferquote werden Konfusionsmatrix und **balancierte Trefferquote**
berichtet: Die Trefferquoten der im Test vorhandenen Bewegungsarten werden
gleich gewichtet. Viele Stillstände können diese Zahl nicht einfach dominieren.
Unbekannte Testklassen zählen als Fehler. Fehlende Klassen werden ausgewiesen;
ein Lernkriterium verlangt alle drei Klassen in Kalibrierung und Test über alle
Seeds. Die Schwierigkeit unterscheidet sich zwischen eigenen Variantenfolgen;
kontrollierte Vergleiche erfolgen deshalb innerhalb einer Variante.

Die konservativen Kriterien verlangen mindestens zehn Seeds, volle Klassenabdeckung
und positive gepaarte Vorteile gegenüber den beiden eingefrorenen Zuständen,
Bildpixeln sowie fehlenden/vertauschten Aktionskopien. Beschreibende
95-%-Bootstrapintervalle über Seed-Differenzen müssen vollständig positiv sein.
Count- und Zeitbin-Merkmale werden getrennt bewertet.

Auch ein positives Ergebnis wäre zunächst Evidenz für eine nutzbare interne
Assoziation, kein vollständiger Nachweis kausalen Verständnisses. Insbesondere
kann die Vorhersage der nächsten Bewegung auch die eigene Aktionspolitik
widerspiegeln. Gutes Nachlesen einer vergangenen Bewegung kann aus visueller
Verarbeitung entstehen. Die 5-ms-Bins erfassen außerdem nicht jede feinere
Zeitkodierung. Ein negatives Readout-Ergebnis schließt daher andere interne
Repräsentationen nicht aus.

## Ergebnisse über zehn Seeds

[Vollständiger Bericht](../../Messungen/Experimente/sensorimotor-ten-seeds/sensorimotor-summary.md),
Seeds 0–9, 32 Processing-Zellen, je 64 Explorations- und Prüfaktionen:

| Variante | Auswahl auf gleichem Replay | Eigene sichtbare Bewegungen nach Freeze |
|---|---:|---:|
| Zählung | 262/720 = 36,4 % | 210/640 = 32,8 % |
| Zählung plus erster Spike | 684/720 = 95,0 % | 545/640 = 85,2 % |
| Inhibitorische Konkurrenz | 393/720 = 54,6 % | 308/640 = 48,1 % |
| Zufälliger Tie-Break, Kontrolle | 720/720 = 100,0 % | 560/640 = 87,5 % |

Die Replay-Spalte enthält auch Endbeobachtungen ohne weitere Ausführung;
die Bewegungsspalte enthält ausschließlich tatsächlich ausgeführte Entscheidungen.
Blockaden an Weltgrenzen können auch bei gewählter Taste Bewegung verhindern.
Die 80 isolierten Aktuatorfälle bestanden vollständig; beide Tasten sind korrekt
ansteuerbar. Die Unterschiede zwischen den Decoderregeln entstehen nicht durch
defekte A/D-Kanäle.

**Zeitinformation:** Alle 458 nichtleeren Zählgleichstände im Count-Replay hatten
unterschiedliche vollständige Spikefolgen. In 422 Fällen unterschieden sich
bereits die ersten Spikezeiten. Deren absoluter Abstand lag im Median bei
1,365 ms. Die Timingregel löst genau diese 422 Fälle zusätzlich auf; 36 exakte
Erstspike-Gleichstände bleiben bestehen. Damit ist ein Informationsverlust durch
die reine Zählung direkt gemessen. Vollständig synchrone Motorfolgen erklären
diese Gleichstände nicht.

**Vier Spikes:** 379 der 458 Zählgleichstände enthielten jeweils vier Spikes.
Der gepoolte Motor-Spikeabstand lag im Median bei 12,375 ms, Minimum 10,008 ms,
bei eingestellter Refraktärzeit 10 ms. Keiner der 5.527 Abstände war exakt 10 ms;
18,2 % lagen zwischen 10 und 11 ms. 35.845 von 47.378 nichtleeren motorischen
Eingangsimpulsen (75,7 %) kamen während einer Refraktärphase an. Diese hohe
Eingangsanregung und ähnliche, regelmäßige Ausgangsraten passen zu häufig vier
Spikes in einem 50-ms-Fenster trotz verschiedener Phasen. Eine vollständige
Sättigung ist damit nicht bewiesen; insbesondere liegt der typische Abstand
oberhalb des Refraktärminimums. Die Ankunftszählung beweist auch nicht für jeden
einzelnen Impuls, wie ihn die Membrandynamik verarbeitet hat.

**Konkurrenz:** 5.292 negative motorische Ankünfte wurden im gemeinsamen Replay
gemessen. Die Hemmung erreicht also die Gegenmotoren und verbessert in dieser
Konfiguration die Eindeutigkeit, lässt aber 327/720 Zählgleichstände übrig.

**Wirkungslernen:** Die balancierte Vorhersage aus Core-Zählungen ergibt:

| Eigene Erfahrungsfolge | Vor Training | Gleich alt, ohne Plastizität | Trainiert, eingefroren | Aktuelle Pixel allein |
|---|---:|---:|---:|---:|
| Zählung | 34,4 % | 32,4 % | 31,7 % | 28,2 % |
| Timingregel | 37,1 % | 40,0 % | 36,1 % | 45,3 % |
| Konkurrenz | 39,6 % | 40,5 % | 36,3 % | 37,0 % |
| Zufalls-Tie-Kontrolle | 54,2 % | 51,0 % | 47,1 % | 56,6 % |

Die Mittelwerte sind innerhalb jeder Zeile vergleichbar; die Zeilen haben
unterschiedliche selbst erzeugte Handlungsfolgen und teilweise unterschiedliche
Klassenabdeckung. Keine Variante erfüllt die kontrollierten Lernkriterien,
auch nicht mit den zeitlich aufgelösten Core-Merkmalen. Beim Nachlesen der bereits
erfolgten Bewegung erreicht die Timingvariante 42,7 % gegenüber 42,2 % im gleich
alten eingefrorenen Netz; auch das liefert keinen belastbaren Lernnachweis.

Aktionskopien wirken auf den Core: Bei der trainierten Timingvariante verändert
Vertauschen der Kopien die exakten Core-Spikefolgen in 709/720 Beobachtungsfenstern.
Sie kommen also an und beeinflussen die Dynamik, einschließlich späterer
Zustandsfolgen. Dieser Einfluss ergibt noch keinen nachgewiesenen Vorhersagegewinn.

Der konkrete Fortschritt ist **bessere Handlungskontrolle durch bereits vorhandene
Spikezeitinformation**. Wirkungswissen, Zielbildung und zielgerichtete Auswahl
bleiben offen. Entsprechend wurde kein neuer Zielverfolgungsversuch durchgeführt.

## Dateien

- `src/sensorimotor/`: Versuch, Aufzeichnung, Kontrollen und Auswertung.
- `src/sensorimotor_main.rs`: eigenständiger ausführbarer Einstieg.
- `sensorimotor.json`, `sensorimotor-summary.md`: alle Bedingungen und Ergebnisse.
- `motor-actuator.csv`: isolierte Einzelmotor-/Stille-/Gleichzeitigkeitstests.
- `motor-windows.csv`, `motor-spikes.csv`: Zählungen, erste Zeitpunkte,
  nichtleere Eingänge während Refraktärphasen und alle exakten Motorpulse.
- `effect-streams.csv`: eigene Bild-/Aktionsfolgen mit Bild-Fingerprints und
  ausschließlich für die Auswertung bestimmten Bewegungslabels.
- `effect-core.csv`: Core-Spikezeiten relativ zum jeweiligen Beobachtungsfenster,
  für jede Lern- und Aktionskopie-Kontrolle.
- `frames/`: Beispielbilder; `worker-*/` enthält bei parallelen Läufen Teilberichte.

Die Motorzeiten stammen aus dem `MotorRoot` einschließlich des konstanten
Faser-Delays von 1 µs. Unterschiede und Interspike-Abstände bleiben dadurch
unverändert. Tests prüfen Decoder-Symmetrie, Zufall nur bei echten Gleichständen,
Kanalrouting, inhibitorische Vorzeichen, unveränderte Anfangsnetze, Parametererhalt,
zeitliche Merkmalszuordnung und reproduzierbare separate CLI-Ausführung.
