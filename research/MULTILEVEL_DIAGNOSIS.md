# Mehrstufige kausale Lerndiagnose

`hippocampal_multilevel_diagnosis` verbindet native Lernhistorie, Nachrechnung
lokaler Regeln auf derselben Ereignisfolge, echte Gegenversuche und einen
Vergleich der Gewichtsrichtungen bei gleicher L2-Stärke. Die Untersuchung
ändert keine Defaults des laufenden Modells.

## Feste Gruppen

| Arm | Eingriff | Interpretation |
| --- | --- | --- |
| A | Lernen eingefroren, native Chemie | Referenz ohne Gewichtslernen |
| B | Drei-Faktor-STDP mit synapsenlokalem f64-Restbetrag | Unveränderte Ausgangsregel |
| C | Dieselbe Regel, **nur effektiver Lernfaktor konstant +1** | Lernkopplung bei erhaltener zellulärer Chemie |
| D | Native unmittelbare Nearest-Neighbor-Pair-STDP | Andere Aktualisierungsart und anderer Zeitpunkt |
| E | Feste künstliche Sensor+Core-Referenzrichtung, Dosis 1, plus vertauschte Kontrolle | Funktionskontrolle, keine gelernte Erinnerung |

A–D starten mit identischem initialem Netzwerk und identischen generierten
Eingaben. Ihre Regeln dürfen danach unterschiedliche Trajektorien erzeugen.
E und sämtliche normierten Gewichtseingriffe starten dagegen am **gleichen
vollständigen trainierten B-Zustand**. Ein Gewichts-Rücktausch und
`same_replay_state` prüfen, dass Warteschlangen, Uhren, Chemie, neuronale Zustände,
Lernmarkierungen und Rundungsreste dabei erhalten bleiben.

Für jeden trainierenden Arm wird sein beobachteter Zustand gegen eine
unbeobachtete Rekonstruktion geprüft. Der Abruf friert Plastizität,
Homöostase und Entwicklung ein. Jede Probe umfasst Schwarzbild, Vollmuster A/B
und Cue A/B mit der konfigurierten Ruhe vor jedem Bild. Die festen Zielvektoren
sind die Vollmusterantworten des B-Zweigs mit Anfangsgewichten. Ein eigener
Gewichts-Reset isoliert den gemeinsamen Gewichtseffekt von C und D an ihrem
jeweiligen Zustand; Transplantationen ihrer Gewichte in B isolieren dagegen
die Gewichtsrichtung von den übrigen Trainingsfolgen.

## Nur den dritten Faktor verändern

Die Library ergänzt `PairStdp::with_learning_signal_override(1.)`. Die Methode
verändert ausschließlich die Zahl im verzögerten lokalen Lernprodukt.
`Neuron::modulation`, Konzentrationen, Rezeptorantworten, Schwellen-, Strom-
und Übertragungsmodulation bleiben unverändert. Ohne expliziten Aufruf bleibt
der native Faktor aktiv. `LearningRule::ThreeFactorConstantPositive` bietet
diesen künstlichen Testeingriff im Modell an; er ist kein neuer Live-Default.

Die Lern-Audits protokollieren den **effektiv verwendeten** Faktor. Die
Chemie-Audits behalten gleichzeitig die natürliche Rezeptorantwort. Deshalb
können diese beiden Zahlen im C-Arm absichtlich voneinander abweichen.
Ein Test mit eingefrorenem Lernen vergleicht native Chemie und sämtliche
Elektroereignisse beider Varianten exakt. Bei aktivem Lernen sind indirekte
Chemieänderungen durch später veränderte Spikes Teil des Gegenversuchs.

## Nachrechnung ohne erneute Netzwerksimulation

`enable_learning_replay_audit()` ergänzt die gewöhnlichen Audits um genaue
Zeitpunkte der verzögerten Integrationen. Nur dieser explizite Auditmodus
speichert solche Grenzen; sie zählen gegen das gleiche Speicherbudget.
Das Aufzeichnen fügt keine Simulationsereignisse hinzu.

Das Ereignisband enthält tatsächliche Integrationsgrenzen, chemische Proben,
synaptische Prä-Ankünfte und Post-Spikes. Die lokale Nachrechnung verwendet
die **nativen** `PairStdp`-Hooks und Gewichtspublikation für:

1. originale Drei-Faktor-Regel mit natürlichem gehaltenem Faktor;
2. gleiche Regel mit Faktor +1;
3. unmittelbare Pair-STDP, deren positive und negative Hook-Änderungen getrennt
   bilanziert werden.

Sie ruft weder `Model::step` noch `Simulation::run` auf und propagiert keine
neuen Spikes. Pro Zeitbatch gilt die tatsächliche Runtime-Reihenfolge:
verzögerte Integration unter vorher gehaltener Chemie, chemische Aktualisierung,
Prä-Ankünfte, Post-Spike-Hooks. Die Ordnung innerhalb einer Phase bleibt erhalten.
Ein bloßes Sortieren getrennter Audits nach Zeit wäre dafür unzureichend.

Die Original-Nachrechnung muss Gewichte, Rundungsreste, Eligibility und
Prä-Traces exakt reproduzieren. Erst danach gelten die beiden Gegenrechnungen
als verwendbare Diagnosen. Ihre Ergebnisse sind **keine** kontrafaktischen
Spikefolgen. C und D werden deshalb zusätzlich wirklich trainiert; Unterschiede
zwischen ihren Endgewichten und der Nachrechnung werden ausdrücklich gemessen.

Alle 1.320 Verbindungen des 256-Zellen-Modells erhalten verknüpfte Statistiken
und lokale Gegenrechnungen. Vollständige native Detailereignisse bleiben auf die
acht stärksten Nettogewichtsänderungen je Arm und die zellulären Ereignisse ihrer
Endpunkte begrenzt. Pair-STDP liefert in dieser API keine ungerundeten
Einzelanforderungen: entsprechende Felder bleiben `null`, statt tatsächliche
f32-Änderungen als ungerundete Wünsche auszugeben.

## Richtung und Stärke

Die E-Referenz verwendet ausschließlich die zuvor auf Seed 17027 erfolgreiche
Methode `sensory_and_recurrent` mit **vorab fester Dosis 1**. Es gibt keine
weitere Dosis-/Methodensuche. Verifizierter Erfolg verlangt aktive und korrekte
Ergänzung beider Cues, positive Gewinne gegenüber Anfangsgewichten, Überlegenheit
gegenüber der vertauschten Dosis und keine erhöhte cuefreie Aktivität.
Die Ziel- und Gegenrichtung werden aus den festgelegten Bildantworten dieses
Seeds konstruiert; dies ist eine künstliche, auf diese Bilder bezogene Referenz.

Für B, C, D, die beiden Offline-Varianten und E werden normierte Richtungen bei
natürlicher B-Stärke und gemeinsamer hoher Stärke verglichen. Die hohe Stärke
ist das Minimum aus E-Norm und allen zulässigen Richtungsradien. Jede Installation
prüft die **tatsächlichen sichtbaren f32-Deltas**: höchstens 1 % Normabweichung
und Richtungskosinus mindestens 0,999. Normierte Richtungen werden nicht durch
Clipping gedreht. Unzulässige oder ungenau publizierbare Varianten bleiben
unausgeführt. A besitzt keine von null verschiedene Lernrichtung.

Wenn die feste E-Referenz scheitert, bleiben die normierten Funktionsvergleiche
offen. Daraus wird kein unmöglicher Netzwerkpfad abgeleitet. Die Bildantworten
und Zielvektoren sind keine unabhängigen Testdaten für E.

## Budget und sequenzieller Ablauf

Pro vollständiger Ausführung gelten höchstens 30 native Trainingsrekonstruktionen,
mit Replay höchstens 60. Identische f32-Gewichtsvektoren teilen eine Beobachtung.
Das Trainingsband ist auf 2.000.000 Quellzeilen beschränkt; die bestehenden
Fensterbudgets bleiben aktiv. Native Detailhistorien sind auf 250.000 Zeilen je
Arm und acht ausgewählte Synapsen begrenzt. Ein Überlauf macht den Trial ungültig.

Der Hauptlauf verwendet Seed 17027. Seed 23003 wurde vor dessen Ausführung
reserviert und war in 214 vorhandenen Manifesten/Kontrollplänen nicht enthalten.
Sein unveränderter Folgelauf wird nur gestartet, wenn die verifizierte Referenz
und ein neuer Effekt von C oder ein unterscheidender Normvergleich im Hauptlauf
vorliegen. Die Entscheidung wird mit dem Hash des Hauptrecords und der bereits
festgelegten Bestätigungskonfiguration dokumentiert.

Eine neue Modellzufallsinitialisierung prüft die Übertragbarkeit des Protokolls.
Sie bestätigt kein Lernen auf einer unabhängigen Aufgabe. Seed 7 bleibt die
bereits analysierte Ergänzung zur stark negativen lokalen Eligibility; diese
Beobachtung wird nicht als Gesetz für alle Seeds vorausgesetzt.

## Start und Auswertung

Ab `DSVLM/main/`:

```bash
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/multilevel_smoke.toml --output /tmp/nerva-multilevel
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/multilevel_smoke.toml --output /tmp/nerva-multilevel
python3 NERVA_Modell/research/multilevel_report.py /tmp/nerva-multilevel
```

Der Hauptlauf verwendet [multilevel_seed17027_20261111.toml](config/multilevel_seed17027_20261111.toml),
der vorab festgelegte Folgelauf
[multilevel_confirmation_23003_20261111.toml](config/multilevel_confirmation_23003_20261111.toml).
Beide enthalten 256 Verarbeitungsneuronen, acht Trainingswiederholungen und
elf Sekunden Ruhe. Ein neuer Quellstand benötigt einen neuen Ausgabeordner.
Quellen und ausführbare Datei müssen vor längeren Messungen wie in
[ARCHIVING.md](ARCHIVING.md) gesichert und gehasht werden.

`multilevel_report.py` importiert keine Simulations- oder Konstruktionshelfer.
Es prüft lokale Gewichte/Restbeträge gegen die native Bilanz, die feste
Referenzentscheidung sowie Norm und Richtung jeder tatsächlich ausgeführten
Installation unabhängig nach. Die Potenzialauswertung vergleicht native
**gespeicherte Membranpotenziale** und nennt abweichende lokale Abtastzeiten;
sie setzt Unterschiede anderer Bilanzfelder nicht mit einer Vm-Änderung gleich.
Der native Rohrecord enthält zusätzlich einen allgemeinen neuronalen
Bilanzvergleich unter `potential_effect`; für reine Membranbefunde ist die
strengere Auswertung des Berichts maßgeblich.
