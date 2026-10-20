#!/bin/bash
# Software 0.1.3 (QD-067): isolierte GUI-Prüfung der Erreichbarkeitsprüfung mit
# Attrappen (QS_ATTRAPPE=1 und QS_NETZ_ATTRAPPE), eigener headless labwc und
# Sitzungsbus (programm/werkzeug/isoliert.py). Nichts wird installiert.
#
#   bash werkzeug/gui-netz.sh <Fall> [BREITExHÖHE] [Binary]
#
# Fall (QS_NETZ_ATTRAPPE = PackageKit, Netzwerkmonitor, Archiv):
#   online       online,online,online     Stufe 1 genügt (wie entwicklungs-vm)
#   nm-archiv    offline,offline,online   Pflichtprobe B2: NetworkManager verwaltet
#                                          eth0 nicht, das Archiv ist erreichbar
#   offline      offline,offline,offline  alle Stufen offline: Sperre, 30-s-Neuprüfung
#
# Ergebnis: nachweis/gui/netz-<Fall>-<Auflösung>/ (ergebnis.txt, Bilder,
# software.log); Bilder zusätzlich nach .
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
P=$E/programm/werkzeug
FALL=${1:?Fall fehlt (online|nm-archiv|offline)}
case $FALL in
online) NETZ=online,online,online ;;
nm-archiv) NETZ=offline,offline,online ;;
offline) NETZ=offline,offline,offline ;;
*) echo "unbekannter Fall: $FALL" >&2; exit 2 ;;
esac
R=${2:-1024x768}
case $R in *x*) ;; *) echo "Auflösung als BREITExHÖHE" >&2; exit 2 ;; esac
BIN=$(readlink -f "${3:-$E/target/release/querencia-software}")
[ -x "$BIN" ] || { echo "kein Binary: $BIN" >&2; exit 2; }
[ -x "$P/bin/qeingabe" ] || bash "$P/bauen.sh" >/dev/null
ZUSATZ=${QS_GPU:+-gpu}
PROFIL=$E/gui-test/netz-$FALL-$R$ZUSATZ
OUT=$E/nachweis/gui/netz-$FALL-$R$ZUSATZ
BILDER=$HOME/review/software-013-bilder/$FALL-$R$ZUSATZ
case $PROFIL in "$E"/gui-test/?*) rm -rf -- "$PROFIL" ;; *) exit 2 ;; esac
case $OUT in "$E"/nachweis/gui/?*) rm -rf -- "$OUT" ;; *) exit 2 ;; esac
mkdir -p "$PROFIL/data" "$PROFIL/config/labwc" "$OUT"
if [ ! -d "$(dirname "$BIN")/share/locale" ]; then
    (cd "$E/programm" && python3 -I werkzeug/i18n.py mo "$PROFIL/locale" >/dev/null)
    export QUERENCIA_SOFTWARE_LOCALEDIR=$PROFIL/locale
fi
export LANG=en_US.UTF-8 LANGUAGE=${LANGUAGE:-de}
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
unset QS_PK_BUS QS_NUR_LESEN QS_APT_QUELLE
export QS_ATTRAPPE=1 QS_NETZ_ATTRAPPE=$NETZ
export QS_LOG=$OUT/protokoll.log
cat >"$PROFIL/env.sh" <<X
BIN=$BIN
Q=$P/bin/qeingabe
QA=$P/bin/qausgabe
OUT=$OUT
R=$R
MODUS=netz-$FALL
FALL=$FALL
E=$E
P=$P
X
if [ "${QS_GPU:-}" = 1 ]; then echo "unset GSK_RENDERER" >>"$PROFIL/env.sh"; else echo "export GSK_RENDERER=cairo" >>"$PROFIL/env.sh"; fi
sha256sum "$BIN" >"$OUT/binary.sha256"
python3 -I "$P/isoliert.py" "$PROFIL" "$E/werkzeug/szenario-netz.sh" 300 || true
cp "$PROFIL"/logs/*.log "$OUT/" 2>/dev/null || true
case $BILDER in "$HOME"/review/software-013-bilder/?*) rm -rf -- "$BILDER" ;; *) exit 2 ;; esac
mkdir -p "$BILDER"
cp "$OUT"/*.png "$OUT/ergebnis.txt" "$BILDER/" 2>/dev/null || true
cat "$OUT/ergebnis.txt"
