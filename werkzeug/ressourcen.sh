#!/bin/sh
# Erzeugt data/resource/software.gresource aus software.gresource.xml (Bauzeit-
# Artefakt, eingecheckt, damit der Bau ohne glib-compile-resources geht).
# Enthält nur das Platzhalter-Fenstersymbol (SVG, Symbolregel: nie 48x48
# zusammen mit Scalable). Das endgültige Symbol liefert SW-5.
set -e
cd "$(dirname "$0")/.."
glib-compile-resources --sourcedir=data/resource --target=data/resource/software.gresource data/resource/software.gresource.xml
