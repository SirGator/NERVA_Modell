# Auflösung des eigenständigen Experimentcodes

Die Pakete `NERVA_Modell/Experimente` und `research/experiments` existieren nicht
mehr. Rust-Paket, Cargo-Dateien, Gastprogramm, Python-Runner, Dispatcher, eigene
Tests und Build-Artefakte sind entfernt. Auch die separaten M4–M6-Programme
und ihre Protokollimplementierungen im Modellpaket sind entfernt. Es gibt keine
Weiterleitung über `research/run.sh experiment` oder `Modell/run-demo.sh m3`.

Im Harness erhalten und tatsächlich verwendet:

- Der standardisierte externe Centroid-Decoder: `research/src/readout.rs`, direkt
  vom vorhandenen `sensorimotor_size`-Adapter verwendet. Unbenutzte historische
  Decoder-Hilfen und JSON-Snapshots sind entfernt.
- Synaptische Konkurrenz: vorhandener lokaler Modellmechanismus, im selben
  Adapter über `mechanisms.synaptic_competition` schaltbar. Standardmäßig aus;
  Strukturentwicklung und Homöostase müssen dafür deaktiviert sein. Der Schalter
  verwendet die üblichen sechs Kontrollen, Ressourcenbudgets und Replay-Prüfungen.
  Spezielle M11-Runner und Audit-JSON-Puffer sind entfernt.
- Morris-Screening, Zustandskarten, Mechanismenvergleiche, Seed-Statistik und
  unabhängige Archivauswertung sind bereits im Harness vorhanden. Die alten
  festen M3–M11-Abläufe werden nicht nochmals als Parallelstruktur übernommen.

```bash
./NERVA_Modell/research/run.sh list
./NERVA_Modell/research/run.sh plan NERVA_Modell/research/config/competition_smoke.toml
./NERVA_Modell/research/run.sh run NERVA_Modell/research/config/competition_smoke.toml
```

Die kleine Konkurrenzkonfiguration dient ausschließlich der technischen
Integration. Sie ersetzt weder den Aufgabenvertrag noch die Schlussfolgerungen
der historischen M11-Studie und reserviert keine wissenschaftlichen Seeds.

Hier erhalten bleiben Markdown-Dokumentation und die dokumentarischen
Protokollspezifikationen als JSON. Dortige alte Befehle und Quellpfade sind
historisch. Aktuelle Ausführung und Tests stehen im [Harness-Handbuch](../../README.md).

`Messungen/` und bestehende `research/results/` bleiben unverändert. Alte Manifeste,
Prüfsummen und Quellenarchive werden nicht umgeschrieben. Historische Läufe sind
nur mit ihrem passenden archivierten Programmstand reproduzierbar. Ihr Code wird
nicht als Bestandteil der aktuellen Anwendung mitgeführt.
