# Querencia Software (Oberfläche)

Grafischer Software-Installer für Querencia Desktop, Teilprojekt SW-3:
Fenster, Ansichten und Dialoge. Rust + GTK 4 ohne libadwaita,
Server-Fensterrahmen von labwc, Menüleiste, Oberfläche Deutsch und
Englisch (GNU gettext, ab 0.1.1), App-ID `org.querencia.Software`. Verbindliche Schnittstellen:
`../docs/SCHNITTSTELLEN.md`; Typen und Traits aus
`qs_typen` (SW-1).

**Umfang 0.1.0:** Menüleiste Datei/Bearbeiten/Ansicht/Hilfe; Seitenleiste mit
„Alle Programme“, 10 Kategorien, „Installiert“, „Aktualisierungen (n)“,
„Expertenliste“; Sofortsuche (300 ms); Liste mit Symbol 48, Name,
Zusammenfassung, Zustand und Knopf; Detailseite mit Symbol 64, Beschreibung,
Version, Größe, Lizenz, Homepage-Link; nicht modale Fortschrittsleiste mit
Abbrechen; Bestätigung vor dem Entfernen mit Simulationsliste; Dialoge für
belegte Paketsperre, fehlende Berechtigung und fehlenden polkit-Agenten;
Statuszeile; Info mit Programmsymbol und Querencia-Q. Programmsymbol von SW-5
als eingebettetes Fenstersymbol; eigenes graues Ersatzsymbol für Anwendungen
ohne (vorhandenes) Symbol. Tastatur: Strg+F, Escape, Eingabetaste,
F5, Strg+Q.

**Backends:** Stand 0.1.0 nur Attrappen (`src/attrappen.rs`, 30 Anwendungen,
simulierte Transaktionen 2 s). Austauschstellen: `../docs/INTEGRATION.md`.

| Datei | Inhalt |
|---|---|
| `src/schnittstellen.rs` | Re-Export der Traits aus `qs_typen` |
| `src/attrappen.rs` | `AttrappeQuelle`, `AttrappeKatalog` |
| `src/monitor.rs` | lokaler Übergangs-Transaktionsmonitor (SW-4 ersetzt ihn) |
| `src/logik.rs` | Ansichten, Filter, Sortierung, Suche, Fehlertexte; Unit-Tests |
| `src/i18n.rs` | gettext: Textdomäne `querencia-software`, Suche der Kataloge, Platzhalter |
| `src/fremdtexte.rs` | Übersetzung der deutschen Texte der Bausteine (Kategorien, Statustexte, Fehlerdialoge von SW-4) |
| `po/` | `querencia-software.pot` (erzeugt), `de.po`, `LINGUAS`; Pflege mit `werkzeug/i18n.py` |
| `data/org.querencia.Software.desktop.in` | englische Vorlage des Starters; `…desktop` daraus mit `[de]`-Zeilen |
| `src/fenster.rs` | Hauptfenster |
| `src/dialoge.rs` | Info, Bestätigung, Meldungen, Fenstersymbol |
| `src/style.css` | klassische Optik, Orange nur für Auswahl und Fortschritt; Dunkelblock für Thema 0.2 (0.1.4) |
| `data/resource/` | eingebettete Symbole (SW-5-Programmsymbol, Ersatzsymbol), `HERKUNFT` |
| `werkzeug/` | isolierte GUI-Prüfung (`../docs/TESTING.md`) |
| `nachweis/` | Release-Binary mit SHA256, Prüf- und GUI-Protokolle |

Desktop-Datei, Themen-Symbol für Starter und Panel und MetaInfo liefert SW-5;
die Übersetzung der Desktop-Datei kommt seit 0.1.1 aus `po/de.po`
(`vorschlag-erzeugen.sh` gleicht sie ab).

## Sprache

Englische msgids im Code, Deutsch aus `po/de.po` (Muster: Querencia Editor,
`docs/i18n-muster.md`). Die Sprache folgt `LANGUAGE`, `LC_ALL`,
`LC_MESSAGES`, `LANG`; die Querencia-Sitzung setzt `LANGUAGE=de`. Die Kataloge
liegen neben der Binary (`share/locale/<lang>/LC_MESSAGES/querencia-software.mo`
in der Ablage), sonst im Präfix (`/usr/share/locale`);
`QUERENCIA_SOFTWARE_LOCALEDIR` setzt das Verzeichnis ausdrücklich.

```bash
python3 werkzeug/i18n.py pot && python3 werkzeug/i18n.py update   # nach neuen Texten
python3 werkzeug/i18n.py pruefen && msgfmt -c -o /dev/null po/de.po
```

## Lizenz

GPL-3.0-or-later (GNU General Public License, Version 3 oder neuer), siehe
[LICENSE](LICENSE). Copyright 2026 Querencia Desktop contributors. Die
Bausteine `qs_typen`, `qs_paketkit`, `qs_katalog` und `qs_transaktion` stehen
unter MIT OR Apache-2.0 (in ihren eigenen Repos).

