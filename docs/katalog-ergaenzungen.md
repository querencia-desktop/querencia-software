# Katalog-Ergänzungen

Querencia Software zeigt nur Programme, die einen AppStream-Eintrag mit
Paketnamen haben. Programme, die Querencia aus Ersatzquellen holt
(`quellen.md`), bringen keinen mit: Ubuntu beschreibt dort nur die Snap-Hülle,
und die Ersatzquellen liefern keine DEP-11-Daten. Ohne Ergänzung findet die
Suche „thunderbird“ nur Birdtray, den KMail-Import-Assistenten und DavMail.

Die Katalog-Ergänzungen schließen diese Lücke: Querencia liefert eigene
AppStream-Beschreibungen für diese Pakete mit. Das Software-Center zeigt sie
wie jeden anderen Eintrag (Suche, Kategorie, Detailseite, Installieren und
Entfernen über PackageKit, Zustand Installiert/Verfügbar, Aktualisierungen).

## Aufbau

| Datei | Inhalt |
|---|---|
| `programm/data/swcatalog/xml/querencia-ergaenzungen.xml` | AppStream-Katalog-XML (`<components origin="querencia-ergaenzungen">`) |
| `programm/data/swcatalog/icons/querencia-ergaenzungen/64x64/<name>.png` | Katalogsymbol 64 px (`<icon type="cached">`) |
| `programm/data/swcatalog/icons/querencia-ergaenzungen/128x128/<name>.png` | Katalogsymbol 128 px |
| `programm/data/ergaenzungen/symbole/<name>.svg` | Quelle der Katalogsymbole |
| `werkzeug/ergaenzungen-symbole.py` | rendert die PNGs aus den SVGs (GdkPixbuf) |
| `werkzeug/ergaenzungen-pruefen.sh` | prüft alles (siehe unten) |

Der Ordner `programm/data/swcatalog/` hat genau den Aufbau von
`/usr/share/swcatalog/` und wird unverändert dorthin installiert.

Erste Einträge: `org.mozilla.Thunderbird` (Paket `thunderbird`) und
`org.mozilla.firefox` (Paket `firefox`).

## Warum eine Katalogdatei unter /usr/share/swcatalog

Zur Wahl standen eine Katalogdatei im Standardort
`/usr/share/swcatalog/xml/` und ein eigener Ordner, den nur Querencia Software
zusätzlich einliest. Gewählt ist die Katalogdatei:

- `/usr/share/swcatalog/xml/` ist der Ort, an dem Pakete Katalogdaten
  mitliefern. libappstream liest ihn regulär (geprüft mit `appstreamcli get`),
  `qs_katalog` liest ihn schon heute als Systempfad. Es gibt keinen Sonderweg
  im Programm; auch andere AppStream-Werkzeuge sehen die Einträge.
- Das Format ist das Katalogformat von AppStream. `appstreamcli validate`
  prüft es, und ein späterer Umzug in eine echte DEP-11-Quelle ändert nur den
  Ort, nicht den Inhalt.
- Ein eigener Ordner bräuchte eine Querencia-eigene Regel, die nur dieses
  Programm kennt.

Die einzige Ergänzung im Programm betrifft die Ablage ohne Systemrechte:
Liegt neben der Binary ein Ordner `share/swcatalog/` (Aufbau wie oben), liest
das Programm ihn zusätzlich, genauso wie die Übersetzungen unter
`share/locale/`. Gesucht wird in dieser Reihenfolge:

1. `QUERENCIA_SOFTWARE_SWCATALOG` (Prüfläufe),
2. `share/swcatalog` neben der Binary,
3. `../share/swcatalog` im Präfix der Binary. Für `/usr/bin` ist das
   `/usr/share/swcatalog`, also ohnehin ein Systempfad; das Programm liest ihn
   dann nicht doppelt.

Die Ergänzungen kommen als letzte Quelle. Bei gleicher ID gewinnt die
Ergänzung, fehlende Felder kommen aus früheren Quellen.

## Snap-Hüllen

Die Snap-Hüllen `thunderbird 2:1snap1-…` und `firefox 1:1snap1-…` haben im
Ubuntu-Katalog keinen Eintrag; als eigene Anwendung tauchen sie also nicht
auf. Sie können aber über den Paketnamen durchscheinen: PackageKit löst
`thunderbird` ohne Ersatzquelle zur Hülle auf, und die Expertenliste sucht
über alle Pakete. Querencia Software behandelt deshalb jede nicht installierte
Paketfassung, deren Upstream-Version die Form `<Zahl>snap<Zahl>` hat, als
nicht vorhanden:

- In Liste und Detailseite steht dann „Nicht verfügbar“, der Knopf bleibt
  gesperrt.
- Eine Aktualisierung auf eine Hülle wird nicht angeboten.
- In der Expertenliste fehlt die Hülle.

Eine schon installierte Hülle bleibt sichtbar und lässt sich entfernen.

## Einen Eintrag hinzufügen

1. **Paket und Quelle klären.** Das Paket muss aus einer Quelle kommen, die
   Querencia einrichtet (`quellen.md`), und darf kein eigenes AppStream haben.
   Prüfen mit `appstreamcli search <name>` oder einer Suche im Programm.
2. **Komponente schreiben** in `querencia-ergaenzungen.xml`, nach dem Muster
   der vorhandenen:
   - `<id>`: die Upstream-ID, sonst Reverse-DNS der Projektseite;
   - `<pkgname>`: der Debian-Paketname, genau einer, kein `<bundle>`;
   - `<name>`;
   - `<summary>` englisch und mit `xml:lang="de"`;
   - `<description>` englisch und deutsch, jeweils als ganzes Element mit
     `xml:lang`, wie im Katalogformat üblich;
   - `<developer>`, `<project_license>` (SPDX), `<url type="homepage">`;
   - `<launchable type="desktop-id">` mit dem Namen der `.desktop`-Datei des
     Pakets;
   - `<categories>` mit Freedesktop-Kategorien; `Network` ergibt die
     Kategorie „Internet“;
   - `<keywords>` englisch und deutsch;
   - Symbole: `<icon type="stock">` mit dem Symbolnamen aus dem Paket (gilt
     nach der Installation) und `<icon type="cached">` in 64×64 und 128×128.
3. **Symbol zeichnen** als `programm/data/ergaenzungen/symbole/<name>.svg` im
   Stil der Querencia-Symbole: flächig, Querencia-Orange `#F07746` und Grau,
   kein Logo eines Dritten. Dann rendern:
   `python3 -I werkzeug/ergaenzungen-symbole.py`.
4. **Prüfen:** `bash werkzeug/ergaenzungen-pruefen.sh` muss „in Ordnung“
   melden. Das Werkzeug prüft vier Dinge:
   - `appstreamcli validate`: Fehler und Warnungen brechen ab, pedantische
     Hinweise nicht;
   - Pflichtfelder und Symbolgrößen;
   - ob die PNGs zu den SVGs passen;
   - ob libappstream die Datei als `/usr/share/swcatalog` liest
     (Pool-Probe in bwrap, das System bleibt unverändert).
5. **Tests:** Die Unit-Tests in `programm/src/echt.rs`
   (`ergaenzung_*`) laden die Datei aus dem Repo. Für einen neuen Eintrag
   ergänzt man dort die erwarteten IDs, falls er sich in Suche oder Kategorie
   mit vorhandenen überschneidet. `cargo test`.
6. **Änderung festhalten:** Eintrag in `programm/CHANGELOG.md`. Wenn eine neue
   Quelle nötig ist, auch in `quellen.md`.

## Installation

- **Paket:** `programm/data/swcatalog/` nach `/usr/share/swcatalog/`
  installieren (XML nach `xml/`, Symbole nach
  `icons/querencia-ergaenzungen/`). Danach liest libappstream die Daten beim
  nächsten Aufbau seines Zwischenspeichers.
- **Ablage ohne Paket:** `share/swcatalog/` neben die Binary legen. Das
  Programm findet den Ordner selbst (siehe oben).
