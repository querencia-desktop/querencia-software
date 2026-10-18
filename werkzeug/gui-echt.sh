#!/bin/bash
# Isolierte GUI-Prüfung des integrierten Programms (SW-6) gegen echte Backends.
# Eigener headless labwc, eigener Sitzungsbus, Wegwerfprofil (isoliert.py von
# SW-3); die laufende Sitzung bleibt unberührt. Nichts wird installiert.
#
#   bash werkzeug/gui-echt.sh <Modus> [BREITExHÖHE] [Binary]
#
# Modus (Szenario werkzeug/szenario-<Modus>.sh):
#   echt           System-PackageKit NUR LESEND (QS_NUR_LESEN=1 sperrt schreibende
#                  Vorgänge im Programm), AppStream des Systems, MetaInfo der
#                  Querencia-Programme im Profil ($XDG_DATA_HOME/metainfo)
#   ohne-metainfo  wie echt, aber ohne MetaInfo im Profil (Gegenprobe zur Suche)
#   pruefbus       SW-1-Prüfgerüst: qs-pk-attrappe auf privatem Bus (/tmp/qs-pk-sw6),
#                  schreibende Vorgänge, Benachrichtigungsdienst auf dem isolierten Bus
#   verweigert     wie pruefbus, polkit-Attrappe verweigert (KeinAgent/NichtBerechtigt)
#   nurlesen       wie pruefbus, aber mit QS_NUR_LESEN=1 (Schreibsperre greift)
#   testfail       SW-1-Prüfgerüst: echter packagekitd --backend=test_fail (Fehlerfall)
#   leerlauf       Start, Speicher nach 10 min Leerlauf (System-PackageKit nur lesend)
#   startzeit      fünf Starts, Zeiten bis Fenster/Katalog/Paketstand
#
# Ergebnis: nachweis/gui/<Modus>-<Auflösung>/ergebnis.txt, Bilder, software.log
# (QS_DEBUG=1), software.err, protokoll.log (qs_transaktion, QS_LOG).
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
P=$E/programm/werkzeug
SW5=$HOME/projects/querencia-software-paket
SW1=$HOME/projects/querencia-software-paketkit
MODUS=${1:?Modus fehlt (echt|ohne-metainfo|pruefbus|verweigert|nurlesen|testfail|leerlauf|startzeit)}
R=${2:-1024x768}
case $R in *x*) ;; *) echo "Auflösung als BREITExHÖHE" >&2; exit 2 ;; esac
BIN=$(readlink -f "${3:-$E/target/release/querencia-software}")
[ -x "$BIN" ] || { echo "kein Binary: $BIN" >&2; exit 2; }
SZ=$E/werkzeug/szenario-$MODUS.sh
case $MODUS in verweigert | nurlesen) SZ=$E/werkzeug/szenario-pruefbus-kurz.sh ;; esac
[ -f "$SZ" ] || { echo "kein Szenario: $SZ" >&2; exit 2; }
[ -x "$P/bin/qeingabe" ] || bash "$P/bauen.sh" >/dev/null

PROFIL=$E/gui-test/$MODUS-$R
OUT=$E/nachweis/gui/$MODUS-$R
case $PROFIL in "$E"/gui-test/?*) rm -rf -- "$PROFIL" ;; *) exit 2 ;; esac
case $OUT in "$E"/nachweis/gui/?*) rm -rf -- "$OUT" ;; *) exit 2 ;; esac
mkdir -p "$PROFIL/data/applications" "$PROFIL/data/icons/hicolor/scalable/apps" "$PROFIL/config/labwc" "$OUT"

# Starter und Symbol wie nach `aktivieren`, aber im Profil.
sed "s|^Exec=.*|Exec=$BIN|; s|^TryExec=.*|TryExec=$BIN|" "$SW5/data/org.querencia.Software.desktop" \
    >"$PROFIL/data/applications/org.querencia.Software.desktop"
cp "$SW5/data/icons/hicolor/scalable/apps/org.querencia.Software.svg" "$PROFIL/data/icons/hicolor/scalable/apps/"
# MetaInfo: SW-5s Kopierskript, Ziel ins Profil umgelenkt (nie ~/.local).
if [ "$MODUS" != ohne-metainfo ]; then
    QS_METAINFO_ZIEL=$PROFIL/data/metainfo bash "$SW5/metainfo/metainfo-benutzer-kopieren.sh" --wirklich \
        >"$OUT/metainfo-kopie.txt"
fi
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

# Prüfbus (privat, nie der Systembus).
export QS_PK_DIR=/tmp/qs-pk-sw6
export QS_PK_ATTRAPPE=$E/target/pruefgeruest/qs-pk-attrappe
unset QS_PK_BUS QS_NUR_LESEN QS_ATTRAPPE
PRUEFBUS=0
case $MODUS in
pruefbus | nurlesen) PRUEFBUS="attrappe" ;;
verweigert) PRUEFBUS="attrappe --polkit verweigern" ;;
testfail) PRUEFBUS="test_fail" ;;
esac
case $MODUS in echt | ohne-metainfo | leerlauf | startzeit | nurlesen | rauch) export QS_NUR_LESEN=1 ;; esac
if [ "$PRUEFBUS" != 0 ]; then
    if [ ! -x "$QS_PK_ATTRAPPE" ]; then
        mkdir -p "$(dirname "$QS_PK_ATTRAPPE")"
        cp "$SW1/target/debug/qs-pk-attrappe" "$QS_PK_ATTRAPPE"
    fi
    # shellcheck disable=SC2086
    bash "$SW1/pruefgeruest/pk-pruefbus.sh" start $PRUEFBUS >"$OUT/pruefbus.txt" 2>&1
    eval "$(bash "$SW1/pruefgeruest/pk-pruefbus.sh" env)"
    case $QS_PK_BUS in unix:path=/tmp/qs-pk-sw6/*) ;; *) echo "Prüfbus-Adresse unerwartet: $QS_PK_BUS" >&2; exit 2 ;; esac
    trap 'bash "$SW1/pruefgeruest/pk-pruefbus.sh" stop >>"$OUT/pruefbus.txt" 2>&1; cp "$QS_PK_DIR"/*.log "$OUT/" 2>/dev/null || true' EXIT
fi
export QS_LOG=$OUT/protokoll.log

cat >"$PROFIL/env.sh" <<X
BIN=$BIN
Q=$P/bin/qeingabe
QA=$P/bin/qausgabe
OUT=$OUT
R=$R
MODUS=$MODUS
E=$E
P=$P
export GSK_RENDERER=cairo
X
sha256sum "$BIN" >"$OUT/binary.sha256"
ZEIT=420
case $MODUS in leerlauf) ZEIT=800 ;; esac
python3 -I "$P/isoliert.py" "$PROFIL" "$SZ" "$ZEIT" || true
cp "$PROFIL"/logs/*.log "$OUT/" 2>/dev/null || true
cat "$OUT/ergebnis.txt"
