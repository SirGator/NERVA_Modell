# Ablage und Reproduktion

| Inhalt | Ort |
| --- | --- |
| Aktuelle Library und lokale Mechanismen | `main/NERVA/` |
| Modell und Adapter | `main/NERVA_Modell/` |
| Versuchsprotokolle und feste Konfigurationen | `research/*.md`, `research/config/` |
| Neue Harness-Ergebnisse | `research/results/<eindeutiger-laufname>/` |
| Neue VM-/Modellprotokolle | `main/Messungen/Modell/<laufname>/` |
| Historische Messreihen | `main/Messungen/Experimente/` |
| Ergebnisberichte | `main/Doku/` |

`main/` ist der aktive Arbeitsstand. Befehle aus den Projektanleitungen werden
von dort gestartet. Die benachbarten Arbeitskopien `test/` und `test copy/`
werden durch die laufende Entwicklung nicht verändert.

## Ergebnissicherung

`research/results/` ist absichtlich von Git ausgeschlossen. Ein Quellcode-Commit
sichert diese Daten nicht. Vor dem Entfernen einer Arbeitskopie muss der gesamte
benötigte Laufordner einschließlich Rohdaten, Manifest, Konfiguration,
Quellenarchiv und Prüfbelegen separat gesichert werden. Die Sicherung muss mit
den abgelegten SHA-256-Werten überprüft werden.

Neue Versuche verwenden eigene Laufnamen. Abgebrochene Versuche und reine
Planungsstände bleiben als solche gekennzeichnet; vorhandene Ergebnisse werden
nicht überschrieben. Für quellengebundene Läufe werden vor dem Versuch die
Quellen und der Binärhash erfasst. Nachträgliche Auswertungen erhalten einen
eigenen Prüfbeleg und benennen ihren Bezug zum ursprünglichen Lauf.

Historische Quellen-, Daten- und Verifikationshashes bleiben unverändert.
Ein neuer Quellstand muss nicht mehr mit `--current` eines alten Prüfers
übereinstimmen. Die Gewichtsdiagnose erlaubt ohne diesen Schalter die erneute
Prüfung ihrer Archive und Rohdaten, unabhängig vom gegenwärtigen Build.

`Messungen/PRUEFSUMMEN.sha256` beschreibt die aktuelle Paketablage und ist von
historischen Verifikationslisten in den Laufordnern getrennt. Nur die aktuelle
Paketliste wird bei einer bewussten Änderung dieser Ablage aktualisiert.

## Build-Dateien

Rust-Builds liegen in den vier jeweiligen `target/`-Ordnern und sind von Git
ausgeschlossen. Sie lassen sich mit `cargo clean --manifest-path <paket>/Cargo.toml`
entfernen und werden beim nächsten Build neu erzeugt. Abhängigkeiten im globalen
Cargo-Cache und Messdaten sind davon getrennt. Während eines laufenden Builds
wird der betreffende Buildordner nicht bereinigt.

[Projektstartseite](../../README.md) · [Architektur](../../ARCHITECTURE.md)
