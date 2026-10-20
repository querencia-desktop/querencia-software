#!/bin/bash
# SW-3s isolierte GUI-Prüfung (programm/werkzeug/szenario.sh, 67 Fälle,
# QS_ATTRAPPE=1) gegen das integrierte Binary. Ergebnis unter
# nachweis/gui/attrappe-<Auflösung>/ (SW-3s eigene Nachweise in
# programm/nachweis bleiben unverändert).
#   bash werkzeug/gui-attrappe.sh [BREITExHÖHE] [Binary]
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
P=$E/programm/werkzeug
R=${1:-1024x768}
BIN=$(readlink -f "${2:-$E/target/release/querencia-software}")
[ -x "$BIN" ] || { echo "kein Binary: $BIN" >&2; exit 2; }
[ -x "$P/bin/qeingabe" ] || bash "$P/bauen.sh" >/dev/null
PROFIL=$E/gui-test/attrappe-$R
OUT=$E/nachweis/gui/attrappe-$R
case $PROFIL in "$E"/gui-test/?*) rm -rf -- "$PROFIL" ;; *) exit 2 ;; esac
case $OUT in "$E"/nachweis/gui/?*) rm -rf -- "$OUT" ;; *) exit 2 ;; esac
mkdir -p "$PROFIL/data" "$PROFIL/config/labwc" "$OUT"
# Kataloge (ab 0.1.1): neben der Binary oder frisch aus programm/po/; Sprache
# wie live (SW-3s Fälle erwarten die deutschen Texte).
if [ ! -d "$(dirname "$BIN")/share/locale" ]; then
    (cd "$E/programm" && python3 -I werkzeug/i18n.py mo "$PROFIL/locale" >/dev/null)
    export QUERENCIA_SOFTWARE_LOCALEDIR=$PROFIL/locale
fi
export LANG=en_US.UTF-8 LANGUAGE=de
cat >"$PROFIL/config/labwc/rc.xml" <<X
<?xml version="1.0"?>
<labwc_config>
  <core><decoration>server</decoration></core>
  <windowRules>
    <windowRule identifier="org.querencia.Software" title="Querencia Software">
      <action name="MoveTo" x="20" y="20" />
    </windowRule>
  </windowRules>
</labwc_config>
X
cat >"$PROFIL/env.sh" <<X
BIN=$BIN
Q=$P/bin/qeingabe
QA=$P/bin/qausgabe
OUT=$OUT
R=$R
X
unset QS_PK_BUS QS_NUR_LESEN
export QS_LOG=$OUT/protokoll.log
sha256sum "$BIN" >"$OUT/binary.sha256"
python3 -I "$P/isoliert.py" "$PROFIL" "$P/szenario.sh" 420 || true
cp "$PROFIL"/logs/*.log "$OUT/" 2>/dev/null || true
cat "$OUT/ergebnis.txt"
