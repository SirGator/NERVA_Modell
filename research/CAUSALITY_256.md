# Kausale Wirksamkeit der lokalen Plastizität

Festes Diagnoseprotokoll, vor Sichtung der neuen Versuchsdaten, 09.11.2026.

256 Processing-Zellen, Seeds 42/73, acht Trainingswiederholungen und dieselben
8×8-Pixelbilder wie im archivierten 256-Zellen-Piloten. Keine neuen Areale,
Lernregeln, Zellparameter, Dendriten oder automatische Suche. Vier Kontrollen:
Cortical, Smooth, No-Chemistry und Frozen. Acht Fälle, je ein zweiter exakter
Replay aller zurückgegebenen Messungen. Maximal 600 Sekunden reale Laufzeit.

## Messkette während des Trainings

Die Beobachtung wurde zuerst in NERVA, anschließend im Modell ergänzt.
Sie ist standardmäßig abgeschaltet und hat einen geprüften identischen nativen
Replayzustand mit und ohne Beobachtung. Ein überlaufender Puffer macht den Lauf
ungültig; er zählt nicht als negativer wissenschaftlicher Befund.

- Eligibility: native signierte Tags vor/nach Pre-/Post-Hooks, Zeitanker und IDs.
- Neuromodulation: bei jedem chemischen Ereignis die tatsächlichen lokalen
  Konzentrationen und berechneten Rezeptorantworten aller Zellen; der Faktor
  wird anschließend wie bisher zwischen diesen Ereignissen gehalten.
- Gewicht: alle tatsächlich geänderten Einzelgewichte. Zusätzlich exakte
  Summen jeder zeitverzögerten Integration je Synapse und 50-ms-Fenster:
  Tagintegrale, angefragte ungerundete Deltas, tatsächliche positive/negative
  Änderungen, Versuche ohne Gewichtsänderung, Faktor- und Gewichtsextrema.
- Neuronale Wirkung: Core-/Motor-Spikezeiten und Bildzählvektoren,
  Ankunftsamplituden, Membranpotenziale an den Bildfensterenden und Tasten.

Unveränderte zeitverzögerte Einzelintegrationen werden zu Fenstersummen
zusammengefasst; tatsächliche Gewichtsänderungen bleiben als Einzelereignisse
sichtbar. Grenze: 100.000 Zeilen zwischen zwei Drains, eine Million Zeilen je
aufgezeichnetem Training. Die Bibliothek benötigt keine neue externe Abhängigkeit.
Rezeptorfreie Motorzellen werden ebenfalls gemessen. Der Faktor am Ende ersetzt
keine Messung während des Trainings. Die Konzentration wird am postsynaptischen
Zellort ausgewertet; separate dendritische Synapsenorte sind nicht implementiert.

## Kontrollierte Gewichtseingriffe

Für jede Probeart werden drei Modelle deterministisch bis zu demselben
trainierten Checkpoint ausgeführt. Vor jedem Eingriff muss der vollständige
`Model::same_replay_state` übereinstimmen: Neuronen, chemisches Feld, Tags,
STDP-Regelzustand, Warteschlange, lokale Buchungen, sensorische Generatorphase,
Decoder und Uhren. Die festen CPU-/Utility-Implementierungen sind zustandslos.
Der Vergleich ersetzt die Einführung einer weiteren Speicher-/Klonarchitektur.

Gewichtlernen, Strukturentwicklung und Homöostase werden gemeinsam eingefroren.
Ein Modell behält gelernte Gewichte, eines erhält nur die Gewichte unmittelbar
vor Training zurück. Durch vorübergehendes Wiederherstellen der gelernten
Gewichte wird zusätzlich geprüft, dass alle übrigen Replayzustände unverändert
sind. Laufende Impulse behalten ihre gemeinsamen, früher erfassten Amplituden.

Zwei getrennt rekonstruierte Checkpoints verwenden dieselbe Folge aus vier
Bildern: einmal mit den ursprünglichen elf Sekunden Schwarz vor jedem Bild,
einmal unmittelbar ohne diese Ruhe. Nach der ersten Wirkung dürfen Zustände
innerhalb einer Folge kausal auseinanderlaufen; die Folge wird nicht nach
jedem Bild zurückgesetzt. Ruhe-Spikes und Bildzählungen bleiben unterscheidbar.

Nur wenn der native Gewichtsvergleich einer Probeart weder Core- noch
Motor-Spikezeiten ändert, wird der zurückgehaltene dritte Checkpoint geprüft:
`w_diag = clamp(w_vorher + 100 * (w_gelernt - w_vorher), 0, 2)`.
Gain 100 bleibt fest; kein weiteres Raster. Clipping und tatsächlich eingesetzte
Gewichte werden gespeichert. Das ist ein künstlicher diagnostischer Kontrast,
kein stärkeres Lernen und kein Gedächtnisnachweis.

## Auswertung und Grenzen

Positive/negative Bruttoänderungen werden von der Endgewichtsdifferenz getrennt.
Faktor-/Eligibility-Überlappung und angefragte Deltas werden vom f32-Rundungs-
und Begrenzungsergebnis getrennt. Die vorhandenen 256-Zellen-Rohdaten müssen für
dieselben Kontrollen unveränderte Trainingsgewichte und Standard-Probevektoren
liefern; Diagnoseaktivierung darf keine neue Dynamik einschleusen.

Ein Gewichtseinfluss auf Eingang oder Potenzial ist von einer Spike-/Tastenwirkung
zu unterscheiden. Auch eine native Spikeänderung belegt noch keine sinnvolle
Ausrichtung, Fehlererkennung oder funktionelles Gedächtnis. Die zwei schon
verwendeten Seeds liefern keine unabhängige Bestätigung. Quellen, Producer,
Konfiguration und Prüfbelege werden vor der gültigen Ausführung archiviert.
