# Querencia Software

Grafischer Software-Installer für Querencia Desktop (PackageKit/apt,
AppStream, Vorbild Ubuntu Software Center). Teil von Querencia Desktop, einer
klassischen Desktopumgebung im GNOME-2-Stil für Wayland (labwc, Rust, GTK 4).
Oberfläche Deutsch und Englisch (GNU gettext).

Dieser Ordner ist der Cargo-Workspace der Integration (SW-6). Einziges
Mitglied ist die Oberfläche in `programm/`; die Bausteine `qs_typen`,
`qs_paketkit`, `qs_katalog` und `qs_transaktion` liegen in ihren eigenen Repos
(`../querencia-software-{paketkit,katalog,transaktion}`) und sind als
Pfad-Abhängigkeiten eingebunden.

| Pfad | Inhalt |
|---|---|
| `programm/` | Oberfläche: Quellen, `po/`, Starter-Vorlage, README, CHANGELOG, LICENSE, BUILDING, TESTING, INTEGRATION |
| `SCHNITTSTELLEN.md` | Vertrag zwischen Oberfläche und Bausteinen |
| `werkzeug/` | isolierte GUI-Prüfung gegen echte Backends (`gui-echt.sh <Modus>`) |
| `integration/` | Integrationsskripte mit Vorlagen und Proben (0.1.0: `software-integrieren.sh`, 0.1.1: `software-011-integrieren.sh`) |
| `doku/` | Phase-0-Befund |
| `nachweis/` | Prüfprotokolle und Belegbilder |

Bauen und Prüfen: `programm/BUILDING.md`, `programm/TESTING.md`. Formatieren
nur mit `rustfmt --edition 2021 programm/src/main.rs` (`cargo fmt` würde die
Bausteine mitformatieren).

## Lizenz

Oberfläche: GPL-3.0-or-later, siehe [programm/LICENSE](programm/LICENSE).
Bausteine: MIT OR Apache-2.0 (in ihren Repos). Copyright 2026 Querencia
Desktop contributors.

