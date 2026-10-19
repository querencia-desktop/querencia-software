#!/usr/bin/env python3
"""Ausschnitt eines Bildschirmfotos speichern und mit einem Bezug vergleichen.

  ausschnitt.py <bild.png> <x> <y> <b> <h> <ziel.png> [<bezug.png>]

Schreibt den Ausschnitt (dreifach vergrößert daneben als <ziel>-x3.png) und
gibt mit Bezug die Zahl der abweichenden Bildpunkte aus („abweichung <n>“).
Nur GdkPixbuf (python3-gi), kein PIL auf der VM.
"""

import sys

import gi

gi.require_version("GdkPixbuf", "2.0")
from gi.repository import GdkPixbuf  # noqa: E402


def lesen(pfad):
    return GdkPixbuf.Pixbuf.new_from_file(pfad)


def punkte(pb):
    """Zeilen ohne Auffüllbytes, je Bildpunkt RGB."""
    roh = pb.get_pixels()
    n = pb.get_n_channels()
    zeilen = []
    for y in range(pb.get_height()):
        z = roh[y * pb.get_rowstride():y * pb.get_rowstride() + pb.get_width() * n]
        zeilen.append(bytes(b for i, b in enumerate(z) if i % n < 3))
    return zeilen


def main():
    bild, x, y, b, h, ziel = sys.argv[1], *map(int, sys.argv[2:6]), sys.argv[6]
    quelle = lesen(bild)
    b = min(b, quelle.get_width() - x)
    h = min(h, quelle.get_height() - y)
    teil = quelle.new_subpixbuf(x, y, b, h).copy()
    teil.savev(ziel, "png", [], [])
    gross = teil.scale_simple(b * 3, h * 3, GdkPixbuf.InterpType.NEAREST)
    gross.savev(ziel[:-4] + "-x3.png", "png", [], [])
    if len(sys.argv) > 7:
        bezug = lesen(sys.argv[7])
        if (bezug.get_width(), bezug.get_height()) != (b, h):
            print(f"abweichung groesse {bezug.get_width()}x{bezug.get_height()} != {b}x{h}")
            return
        a, c = punkte(teil), punkte(bezug)
        n = sum(1 for za, zc in zip(a, c) for i in range(0, len(za), 3) if za[i:i + 3] != zc[i:i + 3])
        print(f"abweichung {n}")


if __name__ == "__main__":
    main()
