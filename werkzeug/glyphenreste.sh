#!/bin/bash
# Glyphenreste nachstellen (Werkstatt QREL-05 W.5): isolierter headless labwc mit
# gles2 (QS_GPU=1, wie live über die GPU), darin glyphenreste.py mit den
# GTK-Renderern cairo, ngl, vulkan und dem Standard. Bilder unter
# nachweis/gui/glyphenreste/. Berührt die laufende Sitzung nicht.
#
#   bash werkzeug/glyphenreste.sh
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
P=$E/programm/werkzeug
PROFIL=$E/gui-test/glyphenreste
OUT=$E/nachweis/gui/glyphenreste
case $PROFIL in "$E"/gui-test/?*) rm -rf -- "$PROFIL" ;; *) exit 2 ;; esac
case $OUT in "$E"/nachweis/gui/?*) rm -rf -- "$OUT" ;; *) exit 2 ;; esac
mkdir -p "$PROFIL" "$OUT"
cat >"$PROFIL/env.sh" <<X
OUT=$OUT
E=$E
RENDERER="cairo ngl vulkan standard"
X
QS_GPU=1 python3 -I "$P/isoliert.py" "$PROFIL" "$E/werkzeug/szenario-glyphenreste.sh" 120
ls "$OUT"
