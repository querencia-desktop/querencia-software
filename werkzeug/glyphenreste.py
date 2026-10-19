#!/usr/bin/env python3
"""Nachstellung der Glyphenreste (Werkstatt QREL-05 W.5, Editor E.7) mit GTK 4.

Mehrere Statuszeilen wie in Querencia Software (9 pt, Rand, Innenabstand)
untereinander, je mit einer anderen Abhilfe. Alle zeigen zuerst den langen
Ladetext und wechseln dann gleichzeitig auf den kürzeren Endtext. Mit den
GPU-Renderern von GTK 4.22 (ngl, vulkan) bleiben ohne Abhilfe die oberen
Glyphenspitzen des alten Textes stehen; cairo zeigt das nicht.

  glyphenreste.py <ausgabe.txt>   (schreibt „bereit“, sobald fotografiert werden kann)
"""

import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk  # noqa: E402

CSS = """
box.qs-status { border-top: 1px solid #c7c2be; background-color: #f2f1f0; padding: 3px 10px; }
label.qs-status-text { color: #3c3c3c; font-size: 9pt; }
"""
LANG = ("Katalog: 0 Programme, 0 installiert · noch keine Aktualisierungsprüfung"
        " · Katalog wird geladen …")
KURZ = "Katalog: 2245 Programme, 7 installiert · letzte Aktualisierungsprüfung 00:31"
VARIANTEN = ("ohne", "deckkraft", "ueberlauf")


def neu_zeichnen(box):
    """Abhilfe: einen Frame lang mit Deckkraft knapp unter 1 zeichnen. Der
    Deckkraftknoten umfasst den ganzen Hintergrund der Leiste, also wird sie
    vollständig neu gezeichnet; im nächsten Frame wieder 1.0 (pixelgleich)."""
    box.set_opacity(0.999)

    def zurueck(widget, _uhr):
        widget.set_opacity(1.0)
        return GLib.SOURCE_REMOVE

    box.add_tick_callback(zurueck)


def main():
    ausgabe = sys.argv[1]
    app = Gtk.Application(application_id="org.querencia.Glyphenreste")

    def aktiv(app):
        prov = Gtk.CssProvider()
        prov.load_from_string(CSS)
        Gtk.StyleContext.add_provider_for_display(
            Gtk.Widget.get_display(Gtk.Window()), prov, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
        win = Gtk.ApplicationWindow(application=app, title="Glyphenreste")
        win.set_default_size(700, 300)
        vbox = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        zeilen = []
        for name in VARIANTEN:
            vbox.append(Gtk.Label(label=name, xalign=0))
            box = Gtk.Box()
            box.add_css_class("qs-status")
            label = Gtk.Label(label=LANG, xalign=0, hexpand=True)
            label.add_css_class("qs-status-text")
            if name == "ueberlauf":
                label.set_overflow(Gtk.Overflow.HIDDEN)
                box.set_overflow(Gtk.Overflow.HIDDEN)
            box.append(label)
            vbox.append(box)
            zeilen.append((name, box, label))
        win.set_child(vbox)
        win.present()

        def wechsel():
            for name, box, label in zeilen:
                label.set_text(KURZ)
                if name == "deckkraft":
                    neu_zeichnen(box)
            GLib.timeout_add(1500, fertig)
            return GLib.SOURCE_REMOVE

        def fertig():
            with open(ausgabe, "w") as f:
                f.write("bereit\n")
            return GLib.SOURCE_REMOVE

        GLib.timeout_add(1500, wechsel)

    app.connect("activate", aktiv)
    app.run([])


if __name__ == "__main__":
    main()
