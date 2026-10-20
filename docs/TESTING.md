# Testen

*Seit 0.1.1 unter `docs/` (Doku-Standard, ENTSCHEIDUNGEN Nr. 17); Pfade wie `werkzeug/…`, `data/…`, `src/…` gelten relativ zu `programm/`.*

```bash
rustfmt --edition 2021 --check src/main.rs
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline     # Logik und Attrappen, ohne Display
```

GUI nur isoliert (eigener headless labwc 0.9.8, eigener D-Bus, eigenes
XDG-Profil, virtuelle Eingaben; nie die laufende Sitzung):

```bash
bash werkzeug/bauen.sh                        # einmalig: qeingabe, qausgabe
bash werkzeug/gui-pruefung.sh 1024x768        # Release-Binary, sonst Debug
bash werkzeug/gui-pruefung.sh 1920x1080
```

Ergebnis je Auflösung in `nachweis/gui/<Auflösung>/`: `ergebnis.txt`,
Bilder, `software.log` (Debug-Ausgabe `QS_DEBUG=1`, daraus werden Zustände
und Positionen gelesen), `software.err`, `lauf-*.log`. Wiederholungsläufe in
`nachweis/gui/wiederholung/`.

Das Szenario (`werkzeug/szenario.sh`, 67 Fälle) prüft: Start und
Fenstergröße, eingebettete Symbole, Symbolwahl (Thema, Datei, ohne Symbol,
unbekannter Themenname → Ersatzsymbol), Katalog und Statuszeile, Seitenleiste, Kategorienwechsel (alle
10), Sofortsuche mit Verzögerung, Detailseite (Eingabetaste, Escape,
Paketdetails, Homepage), Installieren bis Fertig mit Zustandswechsel,
Bedienbarkeit während eines Vorgangs, Abbrechen, Entfernen mit Bestätigung
(Abbruch und Bestätigung), belegte Sperre mit Warten, KeinAgent,
NichtBerechtigt, Aktualisierungen, Menü Ansicht/Datei/Bearbeiten,
Expertenliste mit Suche und Details, Info-Dialog mit Programmsymbol und Q dreimal mit Tastaturfokus
geschlossen (Eingabetaste, Escape, Alt+S), zweite Instanz, Strg+Q.

Menüs werden per Maus bedient (Tastaturnavigation in Popover-Menüs ist im
headless labwc unzuverlässig, Befund G2-H).

## Prüfprotokoll (`QS_DEBUG=1`)

Tab-getrennte Zeilen auf stdout, Koordinaten relativ zum Fensterinhalt
(x, y, Breite, Höhe). Das Szenario liest nur dieses Protokoll.

| Zeile | Bedeutung |
|---|---|
| `katalog n mit_paket mit_symbol deutsch quellen` | nach `Katalog::laden` |
| `symbole fenstersymbol true/false ersatzsymbol true/false` | Ressourcen im Symbolthema |
| `ansicht kennung n suchtext` | Liste neu aufgebaut (`alle`, `kategorie:<Name>`, `installiert`, `aktualisierungen`, `experten`) |
| `status text` | Statuszeile |
| `seitentext aktualisierungen n` | Zähler in der Seitenleiste |
| `details titel zustand knopf` / `details zu` | Detailseite geöffnet / geschlossen |
| `detailsymbol titel quelle`, `zeilensymbol i titel quelle` | Bildquelle `thema:<name>`, `datei:<pfad>` |
| `paketdetails name bytes` | Paketdetails nachgeladen |
| `transaktion gestartet vorgang paket` / `laeuft prozent status` / `simuliert pakete` / `wartet-auf-bestaetigung` / `wartet-auf-sperre` / `fertig paket` / `abgebrochen` / `fehler Fehler` / `wieder-angebunden n` | Transaktionszustände |
| `dialog kennung` / `dialog zu kennung` | `info`, `entfernen`, `sperre`, `kein-agent`, `nicht-berechtigt`, `fehler`, `laeuft` |
| `info-symbole programmsymbol querencia-q` | Inhalt des Info-Dialogs |
| `fokus suche` | Suchfeld hat den Fokus |
| `zeiger x y` | Zeigerbewegung (Kalibrierung) |
| `layout anfang` … `layout ende` | Block mit `menue i …`, `seite kennung …`, `suche …`, `liste …`, `zeile i titel …`, `knopf i text …`, `zurueck …`, `aktion text …`, `homepage …`, `abbrechen …`, `fenster w h`, `toplevel typ titel sichtbar w h` |

SW-5 hat in `~/projects/querencia-software-paket/werkzeug/faelle/FAELLE.md`
ein eigenes Format für seinen Platzhalter vorgeschlagen. Laut FAELLE.md gilt
das Format von SW-3; die Angleichung der SW-5-Fälle an diese Tabelle ist
Sache der Integration.
