# NERVA an einer echten QEMU-VM

Diese Rust-Anwendung verwendet die vorhandene Bibliothek aus `../../NERVA`.
Ein bootfähiger x86-Gast kommuniziert über tatsächliche `IN`/`OUT`-Befehle an
den emulierten COM1-Ports mit einem lernenden Spike-Netz auf dem Host.

Vom Projektverzeichnis `DSVLM` starten:

```bash
./NERVA_Modell/Experimente/run.sh serial
./NERVA_Modell/Experimente/run.sh serial --weights Messungen/Experimente/learned.weights
```

Der zweite Befehl lädt vorhandene Gewichte und speichert nach einem erfolgreichen
Durchlauf. Er kann wiederholt werden. Ein bereits trainierter Gewichtsstand liegt
lokal unter `Messungen/Experimente/learned.weights`. Erwartete Ausgabe:

```text
QEMU connected: guest IN/OUT 0x3F8 ↔ NERVA (12 neurons, 16 plastic synapses)
NERVA_VM_PASS
Guest verified all 4 learned decisions through COM1 I/O ports.
Completed 133 neural trials.
```

Voraussetzungen: Rust mit Edition 2024, GNU `as`/`ld` aus `binutils` und
`qemu-system-x86_64`, auf einem Linux-/Unix-Host. Für dieses Projekt wurde
QEMU 8.2.2 samt Laufzeitbibliotheken lokal unter `NERVA_Modell/Modell/.tools/qemu` bereitgestellt;
das Startskript verwendet diese Installation, falls QEMU im System fehlt.
Die lokalen Programmdateien sind in `.gitignore` ausgeschlossen.
Auf einem anderen Ubuntu-/Debian-Rechner lassen sich die Voraussetzungen mit
`sudo apt install qemu-system-x86 binutils` installieren.

Mit einer normalen QEMU-Installation geht auch:

```bash
cargo run --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --bin nerva-serial -- demo
```

## Modell und Lernaufgabe

Das Netz enthält vier sensorische Eingangsneuronen, vier sensorische
Lehrneuronen und vier motorische Ausgangsneuronen. Alle 16 Verbindungen von
Eingängen zu Ausgängen beginnen mit demselben Gewicht `0.05`. Vier feste
Lehrverbindungen erzeugen beim Training nach 1 ms einen motorischen Spike.
NERVAs lokale Pair-STDP-Regel verstärkt die zeitlich passende Verbindung.
Die Konfiguration verwendet kausale Potenzierung ohne Depression (`a_minus=0`).

Der Gast trainiert 32 Durchgänge der Zuordnung `0→2, 1→3, 2→0, 3→1`.
Diese Zielzuordnung steht ausschließlich im Gast, nicht im Rust-Modell.
Danach fragt er jede Eingabe ohne Lehrsignal ab und prüft die Antwort selbst.
Bei Inferenz sind die Gewichte eingefroren. Motorische Spikes nach dem
Lehrzeitpunkt zählen nicht als Vorhersage. Kein Spike oder mehrere
widersprüchliche Ausgangsspikes ergeben `0xFF` (Enthaltung).

Der vollständige Signalweg verwendet die bestehenden öffentlichen APIs:

```text
VM-Gast: OUT 0x3F8
    → QEMU-UART → lokaler Unix-Socket → ReceptorSignal
    → DirectSensoryTransducer → SensoryRoot → Mapping/Fiber
    → NERVA Simulation mit LIF-Neuronen und Pair-STDP
    → motorischer Spike → Mapping/Fiber → MotorRoot
    → DirectMotorTransducer → EffectorSignal → Unix-Socket
    → QEMU-UART → VM-Gast: IN 0x3F8
```

Es ist ein kleines, lernendes Assoziationsmodell mit vier Eingaben und vier
Aktionen. Es ist kein Sprachmodell und steuert noch kein Gastbetriebssystem.
Der mitgelieferte Gast ist ein 512-Byte-Bootsektor ohne Betriebssystem.
Andere stabile Zuordnungen sind über dasselbe Protokoll trainierbar;
das Umlernen widersprüchlicher Zuordnungen ist mit dieser reinen
Potenzierungskonfiguration nicht vorgesehen.

QEMU verwendet CPU-Emulation (`TCG`), daher ist `/dev/kvm` nicht erforderlich.
Die Demo startet eine eigene VM mit einem nur lesbaren Gastimage, deaktivierter
Netzwerkkarte und ohne eingebundene Hostverzeichnisse. Der Host wartet pro
Socket-I/O höchstens 15 Sekunden. Die Demo beendet ihren eigenen QEMU-Prozess
und entfernt ihr temporäres Verzeichnis auch im Fehlerfall.

## I/O-Ports und Drahtprotokoll

| Gastport | Funktion |
|---|---|
| `0x3F8` | UART-Daten: `OUT` sendet eine Anfrage, `IN` liest eine Antwort |
| `0x3F9–0x3FC` | UART-Konfiguration, 115200 Baud, 8N1 |
| `0x3FD` | UART-Status: Bit 0 Empfang verfügbar, Bit 5 Sender bereit |
| `0xE9` | Gast schreibt das Testergebnis ins Debug-Log |
| `0xF4` | QEMU-Testausgang: `0x10` erfolgreich, `0x11` fehlgeschlagen |

Jede Anfrage und Antwort besteht aus genau vier Bytes:

```text
Anfrage: [0xA5, operation, observation, teacher]
Antwort: [0x5A, status, action, operation]
```

| Operation | Bedeutung | Payload |
|---|---|---|
| `0x00` | Vorhersage ohne Lernen | observation `0..3`, teacher `0` |
| `0x01` | Lernen mit Lehrsignal | observation und teacher jeweils `0..3` |
| `0xFF` | Verbindung geordnet abschließen | beide Payload-Bytes `0` |

`status=0` bedeutet Erfolg, `status=1` eine ungültige Anfrage.
`action=0xFF` bedeutet keine eindeutige Entscheidung.
Die letzte Antwort wiederholt den Operationscode. Nach einer ungültigen
Anfrage sendet der Host eine Fehlerantwort und beendet die Verbindung.
Ein Verbindungsabbruch vor `FINISH`, unvollständige Frames und Zeitüberschreitungen
gelten als Fehler; in diesen Fällen wird kein neuer Gewichtsstand gespeichert.

Eine Anfrage bildet einen 20-ms-Versuch in **Simulationszeit**. Es gibt keine
garantierte Kopplung an die reale Gastzeit. Gewichtsdateien speichern genau
die 16 lernenden Gewichte; Membranpotentiale und Lernspuren starten beim
erneuten Laden frisch. Speichern ersetzt die Zieldatei über eine temporäre Datei.

## Eine vorhandene VM anbinden

Einem QEMU-Gast kann eine separate UART hinzugefügt werden. COM2 ist sinnvoll,
wenn COM1 bereits für die Konsole verwendet wird:

```bash
-chardev socket,id=nerva,path=/tmp/nerva-com2.sock,server=on,wait=on \
-device isa-serial,chardev=nerva,index=1,iobase=0x2f8,irq=3
```

Dann den Host verbinden:

```bash
cargo run --offline --release --manifest-path NERVA_Modell/Experimente/Cargo.toml --bin nerva-serial -- attach /tmp/nerva-com2.sock --weights Messungen/Experimente/learned.weights
```

Der Gast braucht einen passenden Client für das oben beschriebene Protokoll.
Unter Linux lässt sich die UART über `/dev/ttyS1` im Raw-Modus ohne Echo mit
115200 Baud öffnen. Direkte `IN`/`OUT`-Befehle brauchen unter einem
Gastbetriebssystem passende Treiber- oder Portberechtigungen. Der mitgelieferte
Bootsektor zeigt die direkte Portansteuerung. Der `attach`-Modus verwaltet
den verbundenen Gast nicht; nach `FINISH` beendet sich nur der Host.

QEMU-Aufruf und Socket-Optionen: [offizielle QEMU-Dokumentation](https://www.qemu.org/docs/master/system/qemu-manpage.html).

## Prüfungen

```bash
cargo test --offline --manifest-path NERVA_Modell/Experimente/Cargo.toml
cargo clippy --offline --manifest-path NERVA_Modell/Experimente/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path NERVA_Modell/Experimente/Cargo.toml -- --check

# Echter VM-Test, benötigt QEMU auf PATH oder die unten genannten Variablen:
cargo test --offline --manifest-path NERVA_Modell/Experimente/Cargo.toml --test qemu -- --ignored --nocapture
```

Die normalen Tests prüfen drei unterschiedliche gelernte Zuordnungen,
unveränderte Gewichte bei Inferenz, Speichern/Laden, fehlerhafte Gewichtsdateien,
fragmentierte Frames, ungültige Requests und abgebrochene Verbindungen.
Der QEMU-Test bootet den echten Gast und verlangt dessen `NERVA_VM_PASS`
sowie den passenden QEMU-Exitcode. Er wird ohne explizite Anforderung übersprungen.

Für manuelle Verwendung der lokalen QEMU-Installation:

```bash
export QEMU_SYSTEM_X86_64="$PWD/Modell/.tools/qemu/usr/bin/qemu-system-x86_64"
export QEMU_FIRMWARE_DIR="$PWD/Modell/.tools/qemu/usr/share/qemu"
export QEMU_MODULE_DIR="$PWD/Modell/.tools/qemu/usr/lib/x86_64-linux-gnu/qemu"
export LD_LIBRARY_PATH="$PWD/Modell/.tools/qemu/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
cargo test --offline --manifest-path NERVA_Modell/Experimente/Cargo.toml --test qemu -- --ignored --nocapture
```
