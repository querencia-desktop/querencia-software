#!/bin/sh
# Erzeugt data/resource/software.gresource aus software.gresource.xml (Bauzeit-
# Artefakt, eingecheckt, damit der Bau ohne glib-compile-resources geht).
# Enthält das Programmsymbol von SW-5 (Fenstersymbol) und das Ersatzsymbol
# für Anwendungen ohne Symbol; beide nur SVG (Symbolregel).
set -e
cd "$(dirname "$0")/.."
glib-compile-resources --sourcedir=data/resource --target=data/resource/software.gresource data/resource/software.gresource.xml
