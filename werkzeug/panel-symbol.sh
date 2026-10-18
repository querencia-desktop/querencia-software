#!/bin/bash
# SW-6: Live-Panel isoliert starten und den Menüeintrag „Software“ fotografieren.
#   bash werkzeug/panel-symbol.sh <vorher|nachher|beruehrt> [svg|png] [SV_DY]
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
P=$E/programm/werkzeug
FALL=$1; ART=${2:-svg}; SVDY=${3:-60}; R=1920x1080
[ -x "$P/bin/qeingabe" ] || bash "$P/bauen.sh" >/dev/null
PROFIL=$E/gui-test/panel-$FALL-$ART
OUT=$E/nachweis/panel-symbol/$FALL-$ART
case $PROFIL in "$E"/gui-test/?*) rm -rf -- "$PROFIL" ;; esac
case $OUT in "$E"/nachweis/panel-symbol/?*) rm -rf -- "$OUT" ;; esac
mkdir -p "$PROFIL/data/applications" "$PROFIL/config/labwc" "$OUT"
cp "$E/integration/vorschlag/org.querencia.Software.desktop" "$PROFIL/data/applications/"
PNGS=; [ "$ART" = png ] && PNGS=$E/integration/vorschlag/icons/hicolor
cat >"$PROFIL/env.sh" <<X
Q=$P/bin/qeingabe
QA=$P/bin/qausgabe
P=$P
OUT=$OUT
R=$R
SVG=$E/integration/vorschlag/org.querencia.Software.svg
PNGS=$PNGS
PANEL_FALL=$FALL
SYS_X=269
SV_DY=$SVDY
X
python3 -I "$P/isoliert.py" "$PROFIL" "$E/werkzeug/szenario-panelsymbol.sh" 90 || true
cat "$OUT/ergebnis.txt"; ls "$OUT"
