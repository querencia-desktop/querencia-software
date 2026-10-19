#!/bin/bash
# Software 0.1.1, Modus „i18n“: Oberfläche je Sprache (LANGUAGE=de wie live,
# LANGUAGE=en), System-PackageKit NUR LESEND. Prüft die Texte in der
# Debug-Ausgabe, ob glibc den deutschen Katalog einblendet (/proc/<pid>/maps)
# und macht die Belegbilder: Start, Suche, Detailseite, Installiert,
# Aktualisierungen, Menüs, Info-Dialog, Statuszeile nach Ansichtswechsel.
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
[ "${QS_NUR_LESEN:-}" = 1 ] || { nok "Schutz: QS_NUR_LESEN fehlt – Abbruch"; abschluss; exit 0; }
[ -z "${QS_PK_BUS:-}" ] || { nok "Schutz: QS_PK_BUS gesetzt – falscher Modus"; abschluss; exit 0; }
SPRACHE=${LANGUAGE:-de}
info "LANG=$LANG LANGUAGE=$SPRACHE QUERENCIA_SOFTWARE_LOCALEDIR=${QUERENCIA_SOFTWARE_LOCALEDIR:-neben der Binary}"
if [ "$SPRACHE" = de ]; then
    ST_LADEN='Katalog wird geladen …'
    ST_FERTIG='^status\tKatalog: \d+ Programme, \d+ installiert · letzte Aktualisierungsprüfung \d\d:\d\d$'
    ZUSTAENDE='(Verfügbar|Installiert)'; KNOEPFE='(Installieren|Entfernen)'
    INSTALLIERT='Installiert'; ENTFERNEN='Entfernen'; AKTUALISIEREN='Aktualisieren'
    GEPRUEFT='Aktualisierungen geprüft: '
    SCHLIESSEN='chord alt s'
else
    ST_LADEN='Loading catalog…'
    ST_FERTIG='^status\tCatalog: \d+ applications, \d+ installed · last check for updates \d\d:\d\d$'
    ZUSTAENDE='(Available|Installed)'; KNOEPFE='(Install|Remove)'
    INSTALLIERT='Installed'; ENTFERNEN='Remove'; AKTUALISIEREN='Update'
    GEPRUEFT='Checked for updates: '
    SCHLIESSEN='chord alt c'
fi

start_programm
running && ok "I0 Start ($R, $SPRACHE)" || { nok "I0 Start"; abschluss; exit 0; }
fall "I1 Statuszeile beim Laden: „$ST_LADEN“" grep -qP "^status\t.*$ST_LADEN" "$LOG"
warte '^zeit\tkatalog' 0 30
warte '^zeit\tpaketstand' 0 60
sleep 1.5; warte_layout >/dev/null
fall "I2 Statuszeile nach dem Laden ($SPRACHE)" passt_letzte '^status\t' "$ST_FERTIG"
MO=$(grep -c 'LC_MESSAGES/querencia-software.mo' "/proc/$PID/maps" || true)
MOPFAD=$(grep -o '/[^ ]*LC_MESSAGES/querencia-software.mo' "/proc/$PID/maps" | head -1)
if [ "$SPRACHE" = de ]; then
    fall "I3 glibc blendet den deutschen Katalog ein (${MOPFAD:-fehlt})" test "$MO" -ge 1
else
    fall "I3 kein deutscher Katalog eingeblendet (Englisch = msgid)" test "$MO" = 0
fi
fall "I4 Listenzeilen mit Zustand $ZUSTAENDE" bash -c "grep -P '^listenzeile\t' '$LOG' | tail -5 | cut -f5 | grep -qxP '$ZUSTAENDE'"
read -r SX SY SB SH < <(layout | awk -F'\t' '$1=="statuszeile"{print $2, $3, $4, $5; exit}')
statusbild() { # statusbild <Bildname> <Ziel>: Statuszeile ausschneiden
    python3 -I "$E/werkzeug/ausschnitt.py" "$(ls "$OUT"/[0-9][0-9]-"$1".png | tail -1)" \
        $((OX + SX)) $((OY + SY)) "$SB" "$SH" "$OUT/$2" >/dev/null
}
shot start
statusbild start status-start.png

# Menüs ----------------------------------------------------------------------
for kopf in 0 1 2 3; do
    read -r kx ky < <(mitte_von "\$1==\"menue\" && \$2==\"$kopf\"")
    q "move $((OX + kx)) $((OY + ky))" "click" "wait 900"
    shot "menue-$kopf"
    q "key Escape" "wait 400"
done
fall "I5 Menüleiste mit 4 Köpfen" test "$(layout | grep -c '^menue')" = 4

# Suche und Detailseite ----------------------------------------------------------
suche gimp 5
shot suche
v=$(zahl '^paketdetails')
zeile "GNU Image Manipulation Program"; q "key Return" "wait 600"
warte '^paketdetails' "$v" 20; warte_layout >/dev/null
fall "I6 Detailseite GIMP: Zustand und Knopf ($SPRACHE)" passt_letzte '^details' "^details\tGNU Image Manipulation Program\t$ZUSTAENDE\t$KNOEPFE\$"
sleep 0.8
shot details
q "key Escape" "wait 400" "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"

# Installiert, Aktualisierungen ---------------------------------------------------------
v=$(zahl '^ansicht'); seite installiert; warte '^ansicht' "$v" 3; warte_layout >/dev/null
fall "I7 Installiert: Knöpfe „$ENTFERNEN“ oder „$AKTUALISIEREN“" bash -c "layout() { tac '$LOG' | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | tac; }; layout | grep -P '^knopf\t' | cut -f3 | sort -u | grep -qxP '($ENTFERNEN|$AKTUALISIEREN)'"
sleep 0.8; shot installiert
v=$(zahl '^ansicht'); seite aktualisierungen; warte '^ansicht' "$v" 3; warte_layout >/dev/null
fall "I8 Aktualisierungen: Knöpfe „$AKTUALISIEREN“" bash -c "layout() { tac '$LOG' | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | tac; }; [ \"\$(layout | grep -P '^knopf\t' | cut -f3 | sort -u)\" = '$AKTUALISIEREN' ]"
sleep 0.8; shot aktualisierungen
v=$(zahl '^ansicht'); seite alle; warte '^ansicht' "$v" 3; warte_layout >/dev/null
sleep 1.2; shot statuszeile-nach-wechsel
statusbild statuszeile-nach-wechsel status-nach-wechsel.png

# Datei → Aktualisierungen prüfen (nur lesen: GetUpdates) -----------------------------------
v=$(zahl "^status\t.*$GEPRUEFT"); menue 0 0; warte "^status\t.*$GEPRUEFT" "$v" 30
fall "I9 Datei → Aktualisierungen prüfen: Meldung „$GEPRUEFT…“" grep -qP "^status\t.*$GEPRUEFT\d+" "$LOG"

# Info-Dialog, Schließen über die Zugriffstaste der Sprache ------------------------------------
vd=$(zahl '^dialog\tinfo'); vz=$(zahl '^dialog\tzu\tinfo')
menue 3 0; warte '^dialog\tinfo' "$vd" 3; sleep 0.6
shot info
q "$SCHLIESSEN" "wait 800"
fall "I10 Info-Dialog schließt mit ${SCHLIESSEN#chord }" warte '^dialog\tzu\tinfo' "$vz" 2

fall "I11 Nur lesend: keine Transaktion gestartet" test "$(zahl '^(transaktion\tgestartet|nur-lesen\t)')" = 0
beenden "I12 Strg+Q"
abschluss
