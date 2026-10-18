#!/bin/bash
# SW-6, Modi „verweigert“ und „nurlesen“ gegen SW-1s Prüfgerüst (Attrappe):
#   verweigert  polkit-Attrappe verweigert → not-authorized; ohne Agent-Prozess
#               des Benutzers meldet qs_paketkit KeinAgent (Heuristik /proc),
#               sonst NichtBerechtigt. Dialog mit SW-4-Text, Fokus-Schließen.
#   nurlesen    QS_NUR_LESEN=1: das Programm verweigert Simulation/Ausführung,
#               bevor ein Aufruf den Bus erreicht (dbus-monitor zählt mit).
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
case ${QS_PK_BUS:-} in unix:path=/tmp/qs-pk-sw6/*) ;; *) nok "Schutz: kein Prüfbus"; abschluss; exit 0 ;; esac
dbus-monitor --address "$QS_PK_BUS" "type='method_call',interface='org.freedesktop.PackageKit.Transaction'" >"$OUT/bus-aufrufe.txt" 2>&1 &
MON=$!
sleep 0.5
start_programm
warte '^zeit\tkatalog' 0 30; warte '^zeit\tpaketstand' 0 30; sleep 1
titel_von() { tac "$LOG" | awk -F'\t' -v p="$1" '$1=="listenzeile" && $4==p {print $3; exit}'; }
suche gedit 5
T=$(titel_von gedit)
if [ "$MODUS" = verweigert ]; then
    if pgrep -u "$(id -u)" -f 'polkit-agent|lxpolkit|mate-polkit|polkit-gnome' >/dev/null; then ERW=nicht-berechtigt; else ERW=kein-agent; fi
    info "Agent-Prozess des Benutzers: $( [ $ERW = kein-agent ] && echo keiner || echo vorhanden ) → erwartet Dialog $ERW"
    vd=$(zahl "^dialog\t$ERW")
    knopf_von "$T"
    warte "^dialog\t$ERW" "$vd" 10; sleep 0.6
    fall "F26 Verweigerte Berechtigung: Dialog „$ERW“" test "$(zahl "^dialog\t$ERW")" -gt "$vd"
    if [ $ERW = kein-agent ]; then
        fall "F26 KeinAgent: Statuszeile nennt den polkit-Agenten" grep -qP '^status\t.*Es fehlt der Berechtigungsdialog der Sitzung \(polkit-Agent\)' "$LOG"
    fi
    shot "$ERW"
    q "key Return" "wait 700"
    fall "F26 Dialog per Eingabetaste (Tastaturfokus) geschlossen" grep -qP "^dialog\tzu\t$ERW" "$LOG"
    fall "Kein Absturz nach dem Dialog" running
else
    vd=$(zahl '^dialog\tfehler')
    knopf_von "$T"
    warte '^dialog\tfehler' "$vd" 6; sleep 0.6
    fall "Schreibsperre: Simulation im Programm verweigert" grep -qP '^nur-lesen\tsimulieren-verweigert' "$LOG"
    fall "Schreibsperre: Fehlerdialog statt Transaktion" bash -c "grep -qP '^transaktion\tfehler\tSonstiges.*nur-lesen' '$LOG' && [ $(zahl '^dialog\tfehler') -gt $vd ]"
    shot nurlesen
    q "key Return" "wait 600"
fi
beenden F19
kill $MON 2>/dev/null; sleep 0.3
NW=$(grep -c -E 'member=(InstallPackages|RemovePackages|UpdatePackages|RefreshCache)' "$OUT/bus-aufrufe.txt" || true)
NR=$(grep -c 'member=Resolve' "$OUT/bus-aufrufe.txt" || true)
info "Bus-Aufrufe (dbus-monitor): $NR × Resolve, $NW schreibende"
if [ "$MODUS" = nurlesen ]; then
    fall "Schreibsperre: kein Install/Remove/Update/Refresh hat den Bus erreicht (lesend: $NR Resolve)" test "$NW" = 0 -a "$NR" -ge 1
fi
abschluss
