# M4: Implementierungs- und Ausführungsstand

**Die JSON-CLI ist implementiert:**

```bash
./NERVA_Modell/Modell/run-demo.sh m4 --config /tmp/config.json --output /tmp/result.json
```

Der neuronale Einzelfall läuft in `NERVA_Modell/Modell/src/m4/`, die Definitionen und der
Python-Orchestrator liegen unter `NERVA_Modell/Experimente/m4/`. Die 20 ursprünglich
bereitgestellten Konfigurationen sind übernommen. Die spätere Ergänzung wird
als eigenes `gate` mit 2/7 Kanälen bei 8/20 ms ausgeführt.

Implementiert sind vier bitgleich initialisierte Arme, beide Belegungen,
kontrollierte balancierte Exploration, unabhängige visuelle Kalibrierung,
Vorhersagen vor Bewegung/Wirkungsbild, echte verzögerte Weltübergänge,
Freeze ohne Reset, Ruhe- und Interferenzprüfungen, Count-/Zeitbin-Auswerter,
Pixel-Baseline sowie Strom-, STDP-, Struktur-, Membran- und Ereignisprotokolle.
Unbekannte Konfigurationen führen zu Fehlern, ohne stillen Default.

**Wissenschaftlich ausgeführt:** vier Gate-Bedingungen mit je drei Seeds,
danach direkte Signal-/Feedback-/Aktuator- und STDP-Diagnosen, anschließend
alle 20 ursprünglichen Bedingungen mit Seeds 0–2 sowie mit neuen Seeds 100–109.
Je Instanz 256 Trainingstrials, 128 Kalibriertrials und 128 Prüftrials pro
Belegung und Prüfzeitpunkt. Keine Bedingung bestätigt Wirkungslernen.
Im neutralen 2-Kanal-Zustand verändert die Aktionskopie Membranpotentiale,
erzeugt aber keine Core-Spikes. Homöostase und ungleiche räumliche Versorgung
beeinflussen zusätzlich die tatsächlich am Core ankommende Sensoranregung.

**Technisch ausgeführt:** alle ursprünglichen 20 Konfigurationen mit kurzen
Acht-Trial-Läufen, strenge Konfigurations-/Leakage-/Freeze-Prüfungen und
bytegleiche serielle/parallele Wiederholungen. Diese Kurzläufe sind keine
Bestätigung von Lernleistung.

**Noch offen:** ein positiver Lernnachweis und darauf aufbauende Zielversuche.
Die zuletzt angeforderte Reihenfolge wurde eingehalten: zuerst kleine
Sensorfälle, dann direkte Signal-/STDP-Prüfung, danach Skalierung, Kodierung
und Lernmechanismen. Die zehn neuen Seeds wurden separat ausgewertet.

[Vertrag und Startbefehle](../Experimente/m4/README.md) ·
[Ergebnisse und Grenzen](../Experimente/M4_EXPERIMENT.md) ·
[Zehn-Seed-Bericht](../../Messungen/Experimente/m4-confirm/summary.md) ·
[Technische Verifikation](../../Messungen/Experimente/m4-confirm/verification.md)
