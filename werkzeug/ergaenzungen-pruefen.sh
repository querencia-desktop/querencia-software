#!/bin/bash
# Katalog-Ergänzungen prüfen (programm/data/swcatalog/, docs/katalog-ergaenzungen.md).
#
#   bash werkzeug/ergaenzungen-pruefen.sh
#
# 1. appstreamcli validate (ohne Netz; Fehler und Warnungen brechen ab,
#    pedantische Hinweise und Infos werden nur gezeigt)
# 2. je Komponente: Paketname, deutsche Zusammenfassung, Kategorie, Symbol
#    „stock“ und „cached“ 64x64 und 128x128 als PNG in passender Größe
# 3. PNGs passen zu den SVG-Quellen (werkzeug/ergaenzungen-symbole.py --pruefen)
# 4. Pool-Probe: libappstream liest die Datei als /usr/share/swcatalog
#    (bwrap mit Overlay, nichts am System geändert) und findet jede ID mit
#    ihrem Paketnamen. Ohne bwrap wird die Probe übersprungen (Hinweis).
# Exit 0 in Ordnung, 1 Befund, 2 Aufruf/Umgebung.
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
D=$E/programm/data/swcatalog
X=$D/xml/querencia-ergaenzungen.xml
ORIGIN=querencia-ergaenzungen
[ -f "$X" ] || { echo "fehlt: $X" >&2; exit 2; }
command -v appstreamcli >/dev/null || { echo "appstreamcli fehlt" >&2; exit 2; }
W=$(mktemp -d)
export W
trap 'rm -rf -- "$W"' EXIT
# appstreamcli mit Wegwerf-HOME und -Cache: nie ~/.cache des Benutzers.
export HOME=$W/home XDG_CACHE_HOME=$W/cache XDG_DATA_HOME=$W/data LANGUAGE=en LC_ALL=C.UTF-8
mkdir -p "$HOME"
befund=0
fehler() { echo "FEHLER $*"; befund=1; }

echo "== 1. appstreamcli validate ($(appstreamcli --version | head -1))"
if ! appstreamcli validate --no-net --pedantic --no-color "$X"; then
    fehler "appstreamcli validate"
fi

echo "== 2. Komponenten"
python3 -I - "$X" "$D/icons/$ORIGIN" "$ORIGIN" <<'P' || befund=1
import os, struct, sys
import xml.etree.ElementTree as ET

datei, symbole, origin = sys.argv[1:4]
LANG = "{http://www.w3.org/XML/1998/namespace}lang"
wurzel = ET.parse(datei).getroot()
schlecht = 0
def fehler(text):
    global schlecht
    print("FEHLER", text)
    schlecht = 1
if wurzel.tag != "components" or wurzel.get("origin") != origin:
    fehler(f"Wurzel <components origin=\"{origin}\"> erwartet")
ids = []
for c in wurzel.findall("component"):
    cid = c.findtext("id", "").strip()
    ids.append(cid)
    paket = c.findtext("pkgname", "").strip()
    de = [s for s in c.findall("summary") if s.get(LANG) == "de"]
    kategorien = [k.text for k in c.findall("categories/category")]
    stock = [i.text for i in c.findall("icon") if i.get("type") == "stock"]
    cached = [(i.text, i.get("width"), i.get("height")) for i in c.findall("icon") if i.get("type") == "cached"]
    if not paket:
        fehler(f"{cid}: <pkgname> fehlt")
    if c.find("bundle") is not None:
        fehler(f"{cid}: <bundle> – Ergänzungen beschreiben Debian-Pakete")
    if not de:
        fehler(f"{cid}: deutsche Zusammenfassung fehlt")
    if not kategorien:
        fehler(f"{cid}: <categories> fehlt")
    if not stock:
        fehler(f"{cid}: Symbol type=stock fehlt")
    groessen = sorted((int(w), int(h)) for _, w, h in cached)
    if groessen != [(64, 64), (128, 128)]:
        fehler(f"{cid}: Symbol type=cached 64x64 und 128x128 erwartet, ist {groessen}")
    for name, w, h in cached:
        pfad = os.path.join(symbole, f"{w}x{h}", name)
        try:
            with open(pfad, "rb") as f:
                kopf = f.read(24)
        except OSError:
            fehler(f"{cid}: Symbol fehlt: {pfad}")
            continue
        if kopf[:8] != b"\x89PNG\r\n\x1a\n" or struct.unpack(">II", kopf[16:24]) != (int(w), int(h)):
            fehler(f"{cid}: {pfad} ist kein PNG {w}x{h}")
    print(f"{cid}\t{paket}\t{','.join(kategorien)}\t{len(cached)} Symbole")
if len(set(ids)) != len(ids):
    fehler("doppelte ID")
with open(os.environ["W"] + "/ids", "w") as f:
    for cid, c in zip(ids, wurzel.findall("component")):
        f.write(f"{cid}\t{c.findtext('pkgname', '').strip()}\n")
sys.exit(schlecht)
P

echo "== 3. Symbole aus den SVG-Quellen"
python3 -I "$E/werkzeug/ergaenzungen-symbole.py" --pruefen || fehler "Symbole weichen von den SVG-Quellen ab (neu rendern: python3 -I werkzeug/ergaenzungen-symbole.py)"

echo "== 4. Pool-Probe (libappstream liest /usr/share/swcatalog)"
if command -v bwrap >/dev/null && [ -s "$W/ids" ]; then
    while IFS=$'\t' read -r cid paket; do
        aus=$(bwrap --ro-bind / / --dev /dev --proc /proc \
            --overlay-src /usr/share --tmp-overlay /usr/share \
            --ro-bind "$D" /usr/share/swcatalog \
            --tmpfs /var/cache/swcatalog --bind "$W" "$W" \
            appstreamcli get "$cid" 2>&1) || true
        if printf '%s\n' "$aus" | grep -qxF "Package: $paket"; then
            echo "$cid: gefunden, Paket $paket"
        else
            fehler "$cid: libappstream findet die Komponente nicht ($(printf '%s' "$aus" | head -1))"
        fi
    done <"$W/ids"
else
    echo "HINWEIS bwrap fehlt – Pool-Probe übersprungen"
fi

if [ "$befund" = 0 ]; then echo "Ergebnis: in Ordnung"; else echo "Ergebnis: BEFUND"; fi
exit "$befund"
