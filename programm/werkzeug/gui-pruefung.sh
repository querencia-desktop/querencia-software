#!/bin/bash
# Isolierte GUI-Prüfung von Querencia Software (SW-3). Nur Wegwerfprofil,
# eigener headless labwc, eigener D-Bus; die laufende Sitzung bleibt unberührt.
#
#   bash werkzeug/gui-pruefung.sh [BREITExHÖHE] [Binary] [Szenario]
#
# Standard: 1024x768, das Release-Binary target/release/querencia-software
# (sonst Debug), werkzeug/szenario.sh. Ergebnis: nachweis/gui/<Auflösung>/
# ergebnis.txt, Bilder, software.log (Debug-Ausgabe), lauf-*.log.
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
R=${1:-1024x768}
case $R in *x*) ;; *) echo "Auflösung als BREITExHÖHE" >&2; exit 2 ;; esac
BIN=${2:-}
[ -n "$BIN" ] && BIN=$(readlink -f "$BIN")
if [ -z "$BIN" ]; then
    if [ -x "$E/target/release/querencia-software" ]; then BIN=$E/target/release/querencia-software
    else BIN=$E/target/debug/querencia-software; fi
fi
[ -x "$BIN" ] || { echo "kein Binary: $BIN" >&2; exit 2; }
[ -x "$E/werkzeug/bin/qeingabe" ] || bash "$E/werkzeug/bauen.sh"
SZ=${3:-szenario.sh}
PROFIL=$E/gui-test/profil-$R
[ -d "$E/gui-test" ] && [ "$(dirname "$PROFIL")" = "$E/gui-test" ] && rm -rf "$PROFIL"
OUT=$E/nachweis/gui/$R
[ "$SZ" = szenario.sh ] || OUT=$E/nachweis/gui/$R-${SZ%.sh}
rm -rf "$OUT"; mkdir -p "$PROFIL/data" "$PROFIL/config/labwc" "$OUT"
# Serverdekoration wie live; das Fenster liegt fest bei (20, 20).
cat >"$PROFIL/config/labwc/rc.xml" <<R
<?xml version="1.0"?>
<labwc_config>
  <core><decoration>server</decoration></core>
  <windowRules>
    <windowRule identifier="org.querencia.Software" title="Querencia Software">
      <action name="MoveTo" x="20" y="20" />
    </windowRule>
  </windowRules>
</labwc_config>
R
cat >"$PROFIL/env.sh" <<R
BIN=$BIN
Q=$E/werkzeug/bin/qeingabe
QA=$E/werkzeug/bin/qausgabe
OUT=$OUT
R=$R
R
sha256sum "$BIN" >"$OUT/binary.sha256"
python3 -I "$E/werkzeug/isoliert.py" "$PROFIL" "$E/werkzeug/$SZ" 420 || true
cp "$PROFIL"/logs/*.log "$OUT/" 2>/dev/null || true
cat "$OUT/ergebnis.txt"
