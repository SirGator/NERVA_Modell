# Lokales NERVA-Dashboard

Das Dashboard verbindet eine React-Oberfläche mit einem lokalen Python-Server.
Die neuronale Simulation läuft weiterhin im bestehenden Rust-Modell.
Die fertig gebaute Oberfläche liegt bei; zum Starten werden weder Node.js noch
npm benötigt. Python 3.10 oder neuer und die bisherigen Modellvoraussetzungen
reichen aus.

Vom Projektverzeichnis `DSVLM`:

```bash
./NERVA_Modell/Modell/run-demo.sh gui
# gleichwertig:
./NERVA_Modell/Modell/gui.sh
```

Der Browser öffnet `http://127.0.0.1:8765`. Erst mit **Lauf starten** werden das
Modell gebaut und die QEMU-Demo gestartet. Seed, Processing-Zellen, Schrittzahl,
Tastatur und Lernschalter sind vor dem Start einstellbar. `--port 9000` wählt
einen anderen Port; `--no-browser` unterdrückt das automatische Browserfenster.

## Live beobachten und steuern

- Eingangsbild der VM und tatsächlich ausgeführte Tasten mit Motor-Spikezahlen.
- Simulationszeit, Core-/Sensor-/Motor-Spikes, Gewichtsänderungen, Homöostase,
  Strukturwachstum und Pruning.
- Räumlich projiziertes Netzwerk: Zelle anklicken oder auswählen, um
  gespeichertes Membranpotenzial, Schwelle und präzise Verbindungsgewichte zu sehen. Die Core-Ansicht
  ist vollständig bis 1.024 Zellen; die Ansichten begrenzen die Darstellung
  auf 2.500 sichtbare Kanten. Jede Neuron-ID lässt sich direkt eingeben; der vollständige Netzsnapshot ist herunterladbar und enthält alle Zellen und Kanten.
- Aktivitätskurven der letzten 300 Schritte und bis zu 4.000 Core-Spikeereignisse
  aus den letzten fünf Simulationssekunden.
- Pause, Fortsetzen, Stop und irreversibles Einfrieren der Lernregeln für den
  aktuellen Lauf. Für einen neuen lernenden Lauf ein neues Modell starten.

Pause wird zwischen vollständigen Simulationsschritten wirksam. Gehaltene Tasten
werden dabei freigegeben. Stop speichert den bisherigen Laufbericht und beendet
die vom Modell gestartete Demo-VM. Beim Beenden des Servers mit Strg+C wird ein
zugehöriger aktiver Lauf ebenfalls gestoppt.

Die Live-Ausgabe verändert keine neuronalen Regeln oder zufälligen Zustände.
Dateiausgabe und Darstellung kosten zusätzliche reale Laufzeit. Die Simulationszeit
ist weiterhin ereignisgesteuert und unabhängig von QEMUs realer Gastzeit.
Der Telemetrietest vergleicht komplette Replayzustände mit und ohne Beobachtung.

## Gespeicherte Live-Dateien

Neue Läufe liegen unter `Messungen/Modell/live-…-seed-…/`. Zu den bisherigen
CSV-Protokollen, Bildern und `report.json` kommen bei aktiver GUI:

| Datei | Inhalt |
|---|---|
| `dashboard.json` | Atomar geschriebene Live-Metriken, Verlauf, Core-Spikes und Motorwerte |
| `network.json` | Alle Neuronen und Synapsen, in der Regel alle zehn Schritte aktualisiert |
| `dashboard-frame.ppm` | Atomar abgelegtes tatsächliches Eingangsbild |
| `dashboard-control.json` | Steuerbefehle Pause, Stop und Einfrieren |
| `dashboard.log` | Ausgaben des Modellprozesses |

Die CLI kann dieselben Beobachtungen explizit schreiben:

```bash
./NERVA_Modell/Modell/run-demo.sh demo --dashboard --steps 100
```

Der GUI-Server steuert die von ihm selbst gestarteten Prozesse. Er übernimmt
keine schon unabhängig gestartete VM oder einen fremden Modellprozess.
Ohne `--dashboard` werden keine Telemetriedateien angelegt.

## Vergangene Messungen

Im Live-Dashboard öffnet **Messarchiv** die vorhandenen Messreihen. Der reine
Archivbetrachter ist zusätzlich eigenständig im Datenpaket enthalten:

```bash
./Messungen/gui.sh
```

Er läuft auf `http://127.0.0.1:8766`, liest Daten ausschließlich und benötigt weder
Rust noch QEMU oder die Modellbibliothek. Die vollständige
[Archiv-Anleitung](../../Messungen/GUI/README.md) liegt im Messpaket.

Beide Server binden ausschließlich an `127.0.0.1`. Steueranfragen benötigen einen
Sitzungsschlüssel und werden bei fremdem Browser-Ursprung abgewiesen. Dateiwege
sind auf das Messverzeichnis begrenzt; interne Git- und Konfigurationsordner sind
ausgeschlossen. Es werden keine Messdaten an externe Dienste gesendet.

## Oberfläche weiterentwickeln

```bash
cd NERVA_Modell/Dashboard/frontend
npm ci
npm run build
cd ../../..
python3 NERVA_Modell/Dashboard/sync.py
```

`sync.py --build` baut und synchronisiert in einem Schritt. Die Archivkopie enthält
den gleichen Server und die gleichen fertigen Assets, startet aber ohne
Modellsteuerung. Änderungen immer hier vornehmen und anschließend synchronisieren.

```bash
python3 -m unittest discover -s NERVA_Modell/Dashboard/tests -v
cargo test --offline --manifest-path NERVA_Modell/Modell/Cargo.toml --bin nerva-vm telemetry::tests
```
