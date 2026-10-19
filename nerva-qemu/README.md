# QEMU-Schnittstellenbibliothek

`nerva-qemu` stellt die technischen Adapter bereit. Das ausführbare neuronale
Modell liegt unter **[NERVA_Modell/Modell/](../Modell/README.md)**, die Versuche unter
**[NERVA_Modell/Experimente/](../Experimente/README.md)**.

Die Bibliothek enthält:

- `qmp`: lokale QMP-Verbindung mit Handshake, Zeitlimits und Antwortzuordnung.
- `capture`: Bildschirmaufnahme und validierte RGB-/PPM-Bilder.
- `vision_encoder`: Graustufen und Kanalraten aus Helligkeit oder Änderungen.
- `signals`: eigene Kanalraten und zeitgestempelte Motorpulse.
- `motor_decoder`: Spike-Zählung im Zeitfenster; optional entscheidet bei nichtleeren Gleichständen der eindeutig früheste erste Spike (`with_timing_ties`). Stille und unauflösbare Gleichstände enthalten sich.
- `decode_ad`: explizite A/D-Versuchsregeln mit Erstspike-Auswertung und zufälligem Tie-Break als Kontrolle.
- `keyboard`: Tastendrücken, Halten und Loslassen.
- `test_world`: unabhängige Rust-Welten mit sichtbarem Ziel oder ohne Zielmarker.

Die Tastatur unterstützt das vollständige deutsche ISO-105-Layout. Modifier
werden vor normalen Tasten gedrückt und danach losgelassen. Die vollständige
Belegung steht in [KEYBOARD_LAYOUT.md](../Modell/KEYBOARD_LAYOUT.md).

Hier gibt es kein ausführbares Modell und keine festgelegte Netzwerktopologie.
Die Bibliothek ist unabhängig von NERVA. Die Übersetzung ihrer Kanalraten und
Motorpulse in NERVA-Typen erfolgt ausschließlich in `NERVA_Modell/Modell/`.
`run-demo.sh` leitet ältere Aufrufe an das Startskript in `NERVA_Modell/Modell/` weiter.

```bash
./NERVA_Modell/Modell/run-demo.sh
./NERVA_Modell/Modell/run-demo.sh smoke
cargo test --offline --manifest-path NERVA_Modell/nerva-qemu/Cargo.toml
```
