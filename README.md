# Querencia Software

Grafischer Software-Installer für alle, die Querencia Desktop benutzen
(PackageKit/apt und AppStream). Teil von
[Querencia Desktop](https://querencialinux.org) – einer
klassischen Desktopumgebung im GNOME-2-Stil für Wayland (labwc, Rust, GTK 4).
Installiert wird es zusammen mit Querencia Desktop aus der APT-Quelle;
Anleitung auf <https://querencialinux.org>.

*English summary:* A graphical software installer (PackageKit, AppStream) in
the spirit of the Ubuntu Software Center. Part of Querencia Desktop, a classic
GNOME-2-style desktop for Wayland. Alpha software; documentation is in German,
the user interface is available in English and German.

> **Alpha.** Die Bausteine (`qs_typen`, `qs_paketkit`, `qs_katalog`,
> `qs_transaktion`) sind noch Pfad-Abhängigkeiten auf Nachbar-Repos; dieses
> Repo baut erst, wenn sie daneben liegen. Keine Sicherheits- oder
> Datenverlustgarantie.

## Zweck

- Programme suchen (Sofortsuche), nach Kategorien, „Installiert“,
  „Aktualisierungen“ und „Expertenliste“ ansehen; Detailseite mit
  Beschreibung, Version, Größe, Lizenz und Homepage.
- Installieren, Entfernen (mit Bestätigung und Simulationsliste) und
  Aktualisieren über PackageKit, Fortschritt nicht modal mit Abbrechen;
  Meldungen bei belegter Paketsperre, fehlender Berechtigung und fehlendem
  polkit-Agenten.
- Oberfläche auf Englisch, Deutsch, Italienisch, Spanisch und Französisch über
  GNU gettext (englische msgid, `programm/po/*.po`), auch für die deutschen
  Texte der Bausteine.
- Bewusst **nicht** Teil dieses Programms: Paketquellen verwalten,
  Bewertungen, Konten, Snap/Flatpak.

Anwendungskennung (App-ID): `org.querencia.Software` · Binary:
`querencia-software` · Debian-Paket: `querencia-software`

| Pfad | Inhalt |
|---|---|
| `programm/` | Oberfläche: `src/`, `po/`, `data/` (Starter-Vorlage, eingebettete Symbole, Katalog-Ergänzungen unter `data/swcatalog/`), `werkzeug/` (isolierte GUI-Prüfung, i18n), README, CHANGELOG |
| `docs/` | `schnittstellen.md` (Vertrag Oberfläche–Bausteine), `building.md`, `testing.md`, `integration.md`, `katalog-ergaenzungen.md`, `quellen.md`, `phase0-befund.md` |
| `werkzeug/` | isolierte GUI-Prüfung gegen echte Backends (`gui-echt.sh <Modus>`), Szenarien; Katalog-Ergänzungen prüfen und Symbole rendern (`ergaenzungen-pruefen.sh`, `ergaenzungen-symbole.py`) |
| `Cargo.toml` | Workspace; einziges Mitglied `programm/`, Bausteine als Pfad-Abhängigkeiten `../querencia-software-{paketkit,katalog,transaktion}` |

## Stand

| | |
|---|---|
| Version | 0.1.8 (Tag `software/0.1.8`; jedes Programm nur noch einmal im Katalog, GIMP wieder sichtbar); 0.1.7 (Italienisch, Spanisch, Französisch); 0.1.6 (Katalog-Ergänzungen für Thunderbird und Firefox, Snap-Hüllen ausgeblendet); 0.1.5 (Info-Dialog mit Alpha-Hinweis, Website, Copyright und Lizenz, Hilfe-Menü mit Online-Hilfe und „Fehler melden“); seit 0.1.4 dunkle Darstellung (Tags `software/0.1.4`, `software/0.1.4-f2`) |
| Letzte Prüfung | 23.10.2026 (0.1.6): Unit-Tests mit Katalog-Ergänzungen (Suche, Kategorie, Symbol, Paketstand, Snap-Hüllen), `ergaenzungen-pruefen.sh`; 22.10.2026 (0.1.5): Unit-Tests, darunter ein Test gegen interne Kennungen in den Nutzertexten unter `po/`; 21.10.2026 (0.1.4): Unit-Tests, `i18n.py pruefen`; isolierte GUI-Prüfung hell und dunkel (Stand 0.1.3: online 11/0, offline 18/0, Regression Attrappe 68/0) |
| Bekannte Lücken | Bausteine noch in eigenen Repos (siehe oben); GUI-Prüfung gegen echte Backends nur in der Entwicklungsumgebung |
| Getestet auf | Ubuntu 26.04, labwc 0.9.8, GTK 4.22, PackageKit, Rust 1.93.1 |
| Nicht getestet | andere Distributionen, HiDPI, Mehrmonitor |

Änderungen: [programm/CHANGELOG.md](programm/CHANGELOG.md).

## Bauen

Voraussetzungen (Ubuntu 26.04):

```bash
sudo apt install build-essential pkg-config rustc cargo libgtk-4-dev
```

Die Bausteine müssen neben diesem Repo liegen
(`../querencia-software-paketkit` Tag `paketkit/0.1.0`,
`../querencia-software-katalog` Tag `katalog/0.1.1`,
`../querencia-software-transaktion` Tag `transaktion/0.1.1`), dann:

```bash
cargo build --release --locked
```

Das Ergebnis liegt unter `target/release/querencia-software`. Einzelheiten:
[docs/building.md](docs/building.md).

## Prüfen

```bash
rustfmt --edition 2021 --check programm/src/main.rs   # nicht cargo fmt: es würde die Bausteine mitformatieren
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
(cd programm && python3 werkzeug/i18n.py pruefen)
```

Eine CI gibt es noch nicht: sie folgt, sobald die Bausteine in diesem Repo
liegen. Die **GUI-Prüfung** läuft in einer abgeschotteten Sitzung (eigener
labwc ohne Bildschirm, eigenes `HOME` und `XDG_RUNTIME_DIR`, eigener
Systembus mit Attrappen oder echten Backends), niemals in der laufenden
Desktopsitzung: [docs/testing.md](docs/testing.md). Arbeitsweise:
[CONTRIBUTING.md](CONTRIBUTING.md).

## Vorbild und Abgrenzung

Vorbild für Bedienung und Aussehen sind **das Ubuntu Software Center (Ubuntu
10.04) und gnome-packagekit**. Querencia übernimmt davon die Bedienideen
(Seitenleiste mit Kategorien, Liste, Detailseite, Fortschritt), **keinen
Quellcode, keine Texte, keine Grafiken**; genutzt werden PackageKit und
AppStream über ihre Schnittstellen. Das Projekt ist mit Canonical, GNOME und
deren Autoren weder verbunden noch von ihnen unterstützt.

Abweichungen vom Vorbild und Dinge, die bewusst fehlen:

- GTK 4 statt GTK 2, Fensterrahmen vom Compositor (labwc), kein libadwaita.
- Keine Bewertungen, keine Konten, keine Paketquellenverwaltung.

Fremdbestandteile: Rust-Abhängigkeiten (gtk4-rs, gettext-rs, async-trait, der
Bausteine und Umfeld) unter MIT, MIT OR Apache-2.0 und
verwandten freien Lizenzen; Liste mit `cargo metadata --locked`. Dynamisch
gelinkt: GTK 4 und das gettext der glibc; zur Laufzeit über D-Bus: PackageKit,
AppStream-Daten, polkit. In `programm/werkzeug/` liegt eine gekürzte Fassung
des wlroots-Protokolls `virtual-keyboard-unstable-v1.xml` mit eigenem Vermerk
(MIT, siehe Dateikopf).

## Lizenz

Oberfläche (`programm/`) und Werkzeuge: GPL-3.0-or-later, siehe
[LICENSE](LICENSE). Copyright 2026 Querencia Desktop contributors;
Urheberhinweis in [COPYRIGHT](COPYRIGHT). Die Bausteine
(querencia-software-katalog, -paketkit, -transaktion) liegen in eigenen Repos
und stehen derzeit unter GPL-3.0-or-later.

Name und Logo „Querencia“ sind keine freien Inhalte: [MARKE.md](MARKE.md).

## Fehler melden

Fehler, Wünsche und Fragen zu Querencia gehen an die gemeinsame Meldestelle:
<https://github.com/querencia-desktop/querencia/issues/new/choose>.
Sicherheitslücken bitte nicht öffentlich melden, siehe [SECURITY.md](SECURITY.md).
