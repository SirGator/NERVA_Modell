# NERVA mit visueller QEMU-Umgebung

Das ausführbare Modell in `NERVA_Modell/Modell/` implementiert den ersten technischen Meilenstein:
NERVA erhält ausschließlich Bildschirm-Pixel und steuert den Gast mit virtuellen
Tasten. Die neuronalen Bausteine kommen aus `../../NERVA`, die QMP-, Bild- und
Tastaturadapter aus `../nerva-qemu`. Die bisherige serielle Demo bleibt als
separates Programm `nerva-serial` unter [NERVA_Modell/Experimente/](../Experimente/SERIAL_MODEL.md) erhalten.

```text
QEMU-VGA → QMP screendump → RGB-Bild → 64×64 Graustufen
    → PoissonSensoryTransducer → SensoryRoot / Mapping / Fiber
    → räumlicher rekurrenter Processing-Core mit STDP, Homöostase und Entwicklung
    → motorische Spikes → Mapping / Fiber / MotorRoot
    → Spike-Zählfenster → Tastendrücken / Loslassen über QMP
    → veränderter VGA-Bildschirm
```

Der Adapter fragt weder Gast-RAM noch Positionen, Ziele, Labels, Belohnungen
oder serielle Aufgabeninformationen ab. Das Versuchsmodell erhält nur ein
`capture::Frame`. Der Host kennt seine Verkabelung und die Tastenbelegung,
aber keine Gastposition. Der Gast besitzt keine Datenschnittstelle zum Modell.

## Start

Vom Verzeichnis `DSVLM`:

```bash
./NERVA_Modell/Modell/run-demo.sh demo
./NERVA_Modell/Modell/run-demo.sh smoke
./NERVA_Modell/Modell/run-demo.sh demo --steps 100 --seed 7
./NERVA_Modell/Modell/run-demo.sh demo --frozen
./NERVA_Modell/Modell/run-demo.sh demo --encoding change
./NERVA_Modell/Modell/run-demo.sh layout
./NERVA_Modell/Modell/run-demo.sh demo --keyboard wasd
```

Das Skript verwendet ein QEMU auf `PATH` oder die bereits vorhandene lokale
Installation unter `NERVA_Modell/Modell/.tools/qemu`. Deren separat abgelegtes VGA-ROM wird
über `QEMU_VGA_ROM` eingebunden. Voraussetzungen sind Rust Edition 2024,
GNU `as`/`ld` und QEMU mit Standard-VGA und QMP, unter Linux oder einem anderen
Unix-Host. Der Prototyp wurde mit QEMU 8.2.2 geprüft.

Die Demo startet eine eigene VM mit TCG, 32 MiB RAM, nur lesbarem Bootimage,
ohne Netzwerkkarte und ohne eingebundene Hostverzeichnisse. Der kleine BIOS-Gast
zeichnet ein weißes Quadrat auf einer Bahn mit sechs Positionen. **A/D** bewegen
es, **Leertaste** setzt es zurück. **W/S** haben in diesem Gast keine Wirkung.
Es gibt weder ein Ziel noch ein Erfolgssignal an das Netz. QEMU kann den
320×200-VGA-Modus als verdoppeltes 640×400-Bild ausgeben; der Encoder verkleinert
das tatsächlich erfasste Bild.

`smoke` prüft zunächst die Tastaturschnittstelle ohne Netz: D muss das Bild
verändern, A muss das vorherige Bild wiederherstellen. Danach läuft dasselbe
neuronale Experiment wie bei `demo`. Zusätzlich werden alle 105 Tastencodes
mit Key-down und Key-up an QEMU gesendet. Der Test verlangt sensorische und
motorische Spikes sowie neuronale Tastaturaktionen. Im direkten WASD-Kontrollmodell
verlangt er außerdem sichtbare Bildänderungen durch das Netz. Im rekurrenten
Modell werden Bildänderungen gemessen, aber nicht vorausgesetzt; dessen zufällige
Startverschaltung kann auch Tasten wählen, die das aktuelle Bild nicht verändern. Die sichtbare A/D-Rückwirkung
wird unabhängig davon durch den manuellen Screenshot-Rundlauf geprüft.
Die manuellen Prüfaktionen werden nicht als Lehrer in das Modell eingespeist.

## Ergebnisse und Grenzen

Ein Lauf erzeugt standardmäßig ein neues Verzeichnis `runs/<Zeitstempel>-<PID>`
innerhalb von `NERVA_Modell/Modell/`, unabhängig vom aktuellen Arbeitsverzeichnis. Mit `--output NEUES_VERZEICHNIS` lässt
sich der Ort wählen; ein bestehendes Verzeichnis wird nicht überschrieben.

- `first.ppm` und `last.ppm`: echte QEMU-Bildschirmaufnahmen.
- `current.ppm`: wiederverwendete Datei für die Bildschirmaufnahme.
- `trace.csv`: Spikes, lokale Gewichtsänderungen, gewählte Taste und sichtbare
  Bildänderung pro Schritt.
- `report.json`: Summen, Seed, Kodierung, Lernmodus, Tastaturbelegung und Abbruchstatus.
- `keyboard-layout.csv`: motorische Kanäle mit Beschriftung und QMP-Code.
- `motor-activity.csv`: Spikezahlen pro Tastenkanal und Schritt.
- `core-activity.csv`: Spikezahlen pro Processing-Zelle und Schritt.
- `structure.csv`: gebildete und entfernte Core-Verbindungen.

Ein verifizierter früherer WASD-Lauf mit fünf Motorneuronen, Seed 42 und
40 Schritten erzeugte 5.337
sensorische und 234 motorische Spikes, 11 Schritte mit gewählter Taste,
9 Bildänderungen und 17.806 lokale Gewichtsänderungen. Diese Werte dokumentieren
einen einzelnen Schnittstellentest und sind keine Leistungskennzahlen für
zielgerichtetes Verhalten.

Das Standardmodell besitzt 4.096 visuelle Sensorneuronen (8.192 bei ON/OFF),
256 räumlich angeordnete Processing-Neuronen und 105 motorische Neuronen.
Zusätzliche Aktionskopie-Zellen kodieren eigene erfolgreich ausgeführte Tasten.
Es gibt keinen direkten visuellen Sensor→Motor-Pfad. Die Processing-Population
beginnt mit höchstens vier lokalen ausgehenden Verbindungen pro Zelle; weitere
Verbindungen können durch NERVAs Entwicklungsregeln entstehen und entfernt werden.

Details, geometrische Anordnung, Rückführung, Regelparameter und der erste
gepaarte Versuch stehen in [RECURRENT_EXPERIMENT.md](../Experimente/RECURRENT_EXPERIMENT.md).
`--model direct` erhält den früheren Schnittstellentest als Kontrolle.
`--fixed-topology` schaltet nur Strukturänderungen ab; `--no-feedback` entfernt
motorische Rückführung und Aktionskopien; `--no-homeostasis` deaktiviert lokale
Wartung. `--frozen` deaktiviert STDP, Homöostase und Strukturänderungen gemeinsam.

**Gewichtsänderungen, Strukturänderungen und sichtbare Bewegung belegen noch
kein Lernen von Handlungswirkungen.** Die Studie vergleicht interne Aktivität
bei identischem Bild-/Aktionsstrom, keine Aufgabenleistung.

## Kodierung, Motorik und Zeit

`nerva-qemu::vision_encoder` mittelt Quellbildbereiche zu 64×64 Graustufen in
`[0,1]` und liefert modellunabhängige Kanalraten mit maximal 120 Hz.
`Modell` übersetzt diese Raten mit NERVA in unabhängige, reproduzierbar geseedete
Poisson-Spiketrains. Es werden NERVAs vorhandene kontinuierliche
Transducer verwendet, keine Bernoulli-Näherung mit einer festen neuronalen
Zeitschrittweite.

`--encoding change` verwendet getrennte ON/OFF-Kanäle für positive und negative
Helligkeitsänderungen. Dadurch entstehen 8.192 sensorische Kanäle. Das erste
Bild setzt eine stille Vergleichsbasis. Bei einer vollständig statischen
Umgebung entsteht deshalb zunächst keine Aktion; für eine Lernstudie mit
dieser Kodierung wäre zusätzliche autonome Exploration zu untersuchen.

Die vollständige Kanalbelegung mit deutscher ISO-Tastatur steht in
[KEYBOARD_LAYOUT.md](KEYBOARD_LAYOUT.md). `./NERVA_Modell/Modell/run-demo.sh layout` gibt
alle 105 Kanäle aus. Die ursprünglichen Kanäle 0–4 bleiben W, A, S, D und Leertaste.

Der Motor-Decoder zählt Spikes in einem 50-ms-Fenster. Ein eindeutiger Gewinner
unter den normalen Tasten mit mindestens einem Spike bestimmt die normale
Taste. Aktive Shift-, Strg-, Alt-/AltGr- und Windows-Kanäle werden unabhängig
hinzugenommen; dadurch sind Tastenkombinationen möglich. Bei nichtleeren
Zählgleichständen gewinnt standardmäßig die normale Taste mit dem eindeutig
frühesten ersten Spike. Exakt gleiche erste Spikezeiten enthalten sich.
`--motor-decoder count` wählt die frühere reine Zählregel. Aktive Modifier
bleiben unabhängig. Stille lässt alle
Tasten los. Der Adapter drückt Modifier vor normalen Tasten und lässt normale
Tasten vor Modifiern los. Längeres Halten unterliegt der Tastaturwiederholung
des Gastes. Die Zeicheninterpretation übernimmt dessen Tastaturbelegung.

Pro Bild werden 50 ms **Simulationszeit** verarbeitet. Anschließend wartet der
Host 50 ms, damit der Gast die Eingabe verarbeiten kann. Erfassung und Compute
kommen zur realen Laufzeit hinzu. Es gibt keine exakte Kopplung an die Gastuhr;
ein Seed garantiert keine identischen QEMU-Bilder über getrennte Echtzeitläufe.
Identische Pixel- und Zeitfolgen liefern im CPU-Modell reproduzierbare Spikes.
Unterwegs befindliche Synapsen- und Motorimpulse bleiben über Framegrenzen
erhalten.

Die eigene Demo-VM bleibt bei Gastneustarts, etwa durch Strg+Alt+Entf,
verbunden. Der Gast kann dabei vorübergehend seinen Startbildschirm zeigen.

Bei normalem Ende sowie Strg+C, SIGTERM oder SIGHUP versucht der Adapter alle
unterstützten Tasten loszulassen und beendet seine eigene VM. Bei einem
Signal während des Experiments wird ein Teilbericht gespeichert. Eine
abgebrochene QMP-Verbindung kann das Loslassen verhindern. `attach` beendet
die verbundene VM nicht. QMP-Lesevorgänge haben eine gemeinsame Frist von
fünf Sekunden pro Austausch; Nachrichten und Bilder sind größenbegrenzt.

## Eine vorhandene VM verwenden

Die VM braucht eine grafische Karte, eine Tastatur und einen lokalen QMP-Socket.
Beim Start beispielsweise ergänzen:

```bash
-qmp unix:/tmp/nerva-qmp.sock,server=on,wait=off
```

Dann:

```bash
cargo run --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml -- \
  attach /tmp/nerva-qmp.sock --steps 100 --frozen
```

Der Gast und der Adapter müssen für diesen `screendump`-Prototyp auf dieselben
lokalen Dateipfade zugreifen können; die Dumpdatei schreibt der QEMU-Prozess.
Ein Gast-Agent wird nicht benötigt. Der erste Prototyp nimmt die primäre
Anzeige auf und verwendet das QMP-Standardrouting für die Tastatur. Andere
Displays oder mehrere Seats benötigen explizites Routing in einem weiteren
Adapterausbau.

Die Befehle und Ereignisformate folgen der offiziellen QEMU-Dokumentation:
[`screendump`](https://www.qemu.org/docs/master/interop/qemu-qmp-ref.html#command-screendump)
und [`input-send-event`](https://www.qemu.org/docs/master/interop/qemu-qmp-ref.html#command-input-send-event).

## Prüfungen

```bash
cargo test --offline --manifest-path NERVA_Modell/Modell/Cargo.toml
cargo test --offline --manifest-path NERVA_Modell/nerva-qemu/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/Modell/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path NERVA_Modell/Modell/Cargo.toml -- --check
./NERVA_Modell/Modell/run-demo.sh smoke
```

Die normalen Tests prüfen QMP-Handshake, fragmentierte Antworten,
asynchrone Ereignisse, Antwort-IDs, Fehlerantworten und Verbindungsabbrüche;
außerdem Key-up/Key-down und Freigabe beim Drop, PPM-Parsing, Graustufenmittelung,
Poisson-Replay, ON/OFF-Kanäle, Zeitgrenzen und Enthaltung. Modelltests prüfen
stille schwarze Bilder, deterministische Wiederholung und eingefrorene Gewichte.
Die Sockettests benötigen lokale Unix-Sockets.

Verifiziert: Modell- und Adaptertests, der explizite QEMU-Integrationstest, Clippy
ohne Warnungen und die Formatprüfung. Ein echter eingefrorener Kontrolllauf
erzeugte motorische Aktionen und Bildänderungen bei **null Gewichtsänderungen**.
Ein SIGINT-Test speicherte den Teilbericht und bestätigte, dass der eigene
QEMU-Prozess danach beendet war.

Der Integrationstest mit echter VM ist standardmäßig ignoriert. Mit einer
systemweiten QEMU-Installation oder den passenden QEMU-Umgebungsvariablen:

```bash
cargo test --offline --release --manifest-path NERVA_Modell/Modell/Cargo.toml \
  --test visual_qemu -- --ignored --nocapture
```
