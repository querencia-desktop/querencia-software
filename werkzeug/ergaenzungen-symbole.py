#!/usr/bin/env python3
"""Katalog-Ergänzungen: rendert die Symbole aus programm/data/ergaenzungen/symbole/*.svg
in den Symbolcache der Ergänzungen,
programm/data/swcatalog/icons/querencia-ergaenzungen/<N>x<N>/<name>.png (N = 64, 128),
so wie AppStream `<icon type="cached">` sie erwartet.

Offline über GdkPixbuf (SVG-Lader librsvg); rsvg-convert fehlt auf der VM.
Aufruf (aus dem Repo-Root): python3 -I werkzeug/ergaenzungen-symbole.py [--pruefen]
--pruefen schreibt nichts und meldet nur, ob die PNGs zum SVG passen (Exit 1 sonst).
"""
import os
import sys

import gi

gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf  # noqa: E402

ORIGIN = "querencia-ergaenzungen"
STUFEN = (64, 128)


def main() -> int:
    pruefen = "--pruefen" in sys.argv[1:]
    wurzel = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    quelle = os.path.join(wurzel, "programm", "data", "ergaenzungen", "symbole")
    ziel = os.path.join(wurzel, "programm", "data", "swcatalog", "icons", ORIGIN)
    svgs = sorted(n for n in os.listdir(quelle) if n.endswith(".svg"))
    abweichend = 0
    for svg in svgs:
        name = svg[: -len(".svg")]
        for n in STUFEN:
            bild = GdkPixbuf.Pixbuf.new_from_file_at_scale(os.path.join(quelle, svg), n, n, True)
            if bild.get_width() != n or bild.get_height() != n:
                print(f"{svg} {n}: {bild.get_width()}x{bild.get_height()}", file=sys.stderr)
                return 1
            pfad = os.path.join(ziel, f"{n}x{n}", f"{name}.png")
            if pruefen:
                if not os.path.isfile(pfad):
                    print(f"fehlt: {pfad}")
                    abweichend += 1
                    continue
                alt = GdkPixbuf.Pixbuf.new_from_file(pfad)
                if alt.get_width() != n or alt.read_pixel_bytes().get_data() != bild.read_pixel_bytes().get_data():
                    print(f"weicht ab: {pfad}")
                    abweichend += 1
                continue
            os.makedirs(os.path.dirname(pfad), exist_ok=True)
            bild.savev(pfad, "png", [], [])
    print(("geprüft" if pruefen else "gerendert") + f": {len(svgs)} Symbole × {len(STUFEN)} Stufen")
    return 1 if abweichend else 0


if __name__ == "__main__":
    sys.exit(main())
