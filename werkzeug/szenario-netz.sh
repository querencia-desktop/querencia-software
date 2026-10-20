#!/bin/bash
# Software 0.1.3 (QD-067), läuft IN der isolierten labwc-Sitzung (gui-netz.sh):
# Netzwerkzustand in der Statuszeile, Kurzinfo mit den geprüften Stufen und der
# Quelle, Knopf „Erneut prüfen“, Entfernen nie gesperrt, Installieren je nach
# Fall (gelingt / Versuch und Hinweis „PackageKit meldet kein Netzwerk“ /
# Frage mit „Trotzdem versuchen“), automatische Neuprüfung alle 30 s nur
# offline, Wechsel offline → online entfernt die veraltete Fehlermeldung.
# Die Attrappen-Stufen stehen in $NETZDATEI (QS_NETZ_ATTRAPPE=@…).
. "$(dirname "$0")/gemeinsam.sh"
aufloesung
info "Fall $FALL, QS_NETZ_ATTRAPPE=${QS_NETZ_ATTRAPPE:-} ($(cat "$NETZDATEI" 2>/dev/null))"
[ "${QS_ATTRAPPE:-}" = 1 ] || { nok "Schutz: QS_ATTRAPPE fehlt – Abbruch"; abschluss; exit 0; }

# Installations-Bestätigung (SW-4: Zusatzpakete) annehmen, falls sie kommt.
bestaetige() { # bestaetige <vorher>
    local i=0
    while [ $i -lt 30 ] && [ "$(zahl '^dialog\tinstallieren')" -le "$1" ]; do sleep 0.1; i=$((i + 1)); done
    [ "$(zahl '^dialog\tinstallieren')" -gt "$1" ] && { sleep 0.4; q "key Return" "wait 150"; }
    return 0
}
statusbild() { # statusbild <Bildname> <Ziel>: Statuszeile ausschneiden
    local sx sy sb sh
    read -r sx sy sb sh < <(layout | awk -F'\t' '$1=="statuszeile"{print $2, $3, $4, $5; exit}')
    [ -n "${sb:-}" ] || return 0
    python3 -I "$E/werkzeug/ausschnitt.py" "$(ls "$OUT"/[0-9][0-9]-"$1".png | tail -1)" \
        $((OX + sx)) $((OY + sy)) "$sb" "$sh" "$OUT/$2" >/dev/null
}
dialog_zu() { # dialog_zu <kennung>
    local v; v=$(zahl "^dialog\tzu\t$1")
    q "key Escape" "wait 500"
    warte "^dialog\tzu\t$1" "$v" 3
}

start_programm
running && ok "N0 Start ($R, $FALL)" || { nok "N0 Start"; abschluss; exit 0; }
warte '^netz\t(online|offline)\t' 0 6
sleep 0.8; warte_layout >/dev/null
case $FALL in
online)
    fall "N1 Statuszeile „Netzwerk: online (PackageKit)“" passt_letzte '^netzstatus\t' '^netzstatus\tNetzwerk: online \(PackageKit\)$'
    fall "N2 Stufe 1 genügt: Netzwerkmonitor und Archiv nicht gefragt" passt_letzte '^netz\t(online|offline)\t' '^netz\tonline\tpackagekit\tOnline\t-\t-\t'
    ;;
nm-archiv)
    fall "N1 Statuszeile „Netzwerk: online (Archiv erreichbar), PackageKit: offline“" passt_letzte '^netzstatus\t' '^netzstatus\tNetzwerk: online \(Archiv erreichbar\), PackageKit: offline$'
    fall "N2 alle drei Stufen gefragt, Archiv entscheidet" passt_letzte '^netz\t(online|offline)\t' '^netz\tonline\tarchiv\tOffline\taus\terreichbar\t[^\t]+$'
    ;;
offline)
    fall "N1 Statuszeile „Netzwerk: offline: <Host> nicht erreichbar (keine Antwort in 3 s)“" passt_letzte '^netzstatus\t' '^netzstatus\tNetzwerk: offline: [^ ]+ nicht erreichbar \(keine Antwort in 3 s\)$'
    fall "N2 alle drei Stufen offline" passt_letzte '^netz\t(online|offline)\t' '^netz\toffline\t-\tOffline\taus\tnicht-erreichbar\t[^\t]+$'
    ;;
esac
fall "N3 „wird geprüft …“ erscheint vor dem Ergebnis" grep -qP '^netzstatus\tNetzwerk: wird geprüft …$' "$LOG"
fall "N4 Knopf „Erneut prüfen“ in der Statuszeile" bash -c "tac '$LOG' | awk '/^layout\tende/{f=1;next} /^layout\tanfang/{if(f)exit} f' | grep -qP '^netzknopf\t'"
shot start
statusbild start status-start.png

# Kurzinfo: alle Stufen, geprüfte Quelle -----------------------------------
read -r nx ny < <(mitte_von '$1=="netzfeld"')
q "move $((OX + nx)) $((OY + ny))" "wait 300" "move $((OX + nx + 2)) $((OY + ny))" "wait 1800"
shot kurzinfo
q "move $((OX + 300)) $((OY + 200))" "wait 600"

# Erneut prüfen ----------------------------------------------------------------
vp=$(zahl '^netz\tpruefe'); ve=$(zahl '^netz\t(online|offline)\t')
klick_auf '$1=="netzknopf"' "Erneut prüfen"
warte '^netz\t(online|offline)\t' "$ve" 6
fall "N5 Knopf startet eine neue Prüfung mit Ergebnis" test "$(zahl '^netz\tpruefe')" -gt "$vp" -a "$(zahl '^netz\t(online|offline)\t')" -gt "$ve"
sleep 0.5; warte_layout >/dev/null

# Entfernen ist nie gesperrt (braucht kein Netz) -------------------------------------
suche zeichentabelle
vd=$(zahl '^dialog\tentfernen'); vf=$(zahl '^transaktion\tfertig')
knopf_von Zeichentabelle
warte '^dialog\tentfernen' "$vd" 4; sleep 0.6
q "key Return"
warte '^transaktion\tfertig' "$vf" 6
fall "N6 Entfernen gelingt ohne Rückfrage zum Netz ($FALL)" bash -c "grep -qP '^transaktion\tfertig\tgucharmap' '$LOG' && ! grep -qP '^netz\tvor-vorgang' '$LOG'"
sleep 0.8; warte_layout >/dev/null
shot nach-entfernen
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 400"

# Installieren -------------------------------------------------------------------
suche minen
vi=$(zahl '^dialog\tinstallieren'); vg=$(zahl '^transaktion\tgestartet'); vf=$(zahl '^transaktion\tfertig')
case $FALL in
online)
    knopf_von Minen; bestaetige "$vi"
    warte '^transaktion\tfertig' "$vf" 8
    fall "N7 online: Installieren gelingt" grep -qP '^transaktion\tfertig\tgnome-mines' "$LOG"
    sleep 0.6; shot installiert
    ;;
nm-archiv)
    vd=$(zahl '^dialog\tnetz-pk')
    knopf_von Minen; bestaetige "$vi"
    warte '^dialog\tnetz-pk' "$vd" 10; sleep 0.8
    fall "N7 nicht gesperrt, keine Rückfrage: Vorgang startet trotz PackageKit offline" bash -c "[ \$(grep -cP '^transaktion\tgestartet' '$LOG') -gt $vg ] && ! grep -qP '^dialog\tnetz-offline-frage' '$LOG'"
    fall "N8 PackageKit-Fehler no-network → Hinweis „PackageKit meldet kein Netzwerk“" bash -c "grep -qP '^transaktion\tfehler\tNetzFehlt$' '$LOG' && [ \$(grep -cP '^dialog\tnetz-pk' '$LOG') -gt $vd ]"
    fall "N9 Statuszeile meldet den Fehler" grep -qP '^status\t.*Minen: Keine Netzwerkverbindung\.' "$LOG"
    shot hinweis-packagekit
    dialog_zu netz-pk
    ;;
offline)
    vd=$(zahl '^dialog\tnetz-offline-frage'); vv=$(zahl '^netz\tvor-vorgang')
    knopf_von Minen
    warte '^dialog\tnetz-offline-frage' "$vd" 8; sleep 0.8
    fall "N7 vor der Frage frisch geprüft" test "$(zahl '^netz\tvor-vorgang')" -gt "$vv"
    fall "N8 Frage statt Sperre: „Keine Netzwerkverbindung“ mit „Trotzdem versuchen“, noch kein Vorgang" bash -c "grep -qP '^netz\tfrage-offline$' '$LOG' && [ \$(grep -cP '^transaktion\tgestartet' '$LOG') = $vg ]"
    shot frage-offline
    dialog_zu netz-offline-frage
    fall "N9 Abbrechen (Escape): kein Vorgang" test "$(zahl '^transaktion\tgestartet')" = "$vg"
    # Trotzdem versuchen (Alt+V): die Attrappe scheitert wie apt ohne Netz.
    vd=$(zahl '^dialog\tnetz-offline-frage'); vo=$(zahl '^dialog\tnetz-offline$')
    knopf_von Minen
    warte '^dialog\tnetz-offline-frage' "$vd" 8; sleep 0.6
    q "chord alt v" "wait 300"
    bestaetige "$vi"
    warte '^dialog\tnetz-offline$' "$vo" 10; sleep 0.8
    fall "N10 „Trotzdem versuchen“ startet den Vorgang" bash -c "grep -qP '^netz\ttrotzdem$' '$LOG' && [ \$(grep -cP '^transaktion\tgestartet' '$LOG') -gt $vg ]"
    fall "N11 no-network → Dialog „Keine Netzwerkverbindung“ (ohne Frage)" bash -c "grep -qP '^transaktion\tfehler\tNetzFehlt$' '$LOG' && [ \$(grep -cP '^dialog\tnetz-offline$' '$LOG') -gt $vo ]"
    shot nach-versuch
    dialog_zu netz-offline
    ;;
esac
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 400"

# Automatische Neuprüfung: nur offline, alle 30 s -------------------------------------
t0=$(date +%s.%N); vp=$(zahl '^netz\tpruefe')
if [ "$FALL" = offline ]; then
    warte '^netz\tpruefe' "$vp" 35
    t1=$(date +%s.%N)
    info "Neuprüfung nach $(awk -v a="$t0" -v b="$t1" 'BEGIN{printf "%.1f", b-a}') s (seit Messbeginn; Zeitgeber lief seit der letzten Prüfung)"
    fall "N12 offline: Neuprüfung ohne Eingabe innerhalb von 35 s" test "$(zahl '^netz\tpruefe')" -gt "$vp"
    warte '^netz\t(online|offline)\t' "$(zahl '^netz\t(online|offline)\t')" 6
    fall "N13 Fehlermeldung bleibt, solange offline" passt_letzte '^status\t' 'Minen: Keine Netzwerkverbindung\.$'
    sleep 0.5; shot nach-neupruefung
    # Netz kommt zurück (NM-b S3): Attrappe umschalten, „Erneut prüfen“.
    echo online,online,online >"$NETZDATEI"
    ve=$(zahl '^netz\t(online|offline)\t')
    klick_auf '$1=="netzknopf"' "Erneut prüfen"
    warte '^netz\t(online|offline)\t' "$ve" 6; sleep 0.6
    fall "N14 wieder online: „Netzwerk: online (PackageKit)“" passt_letzte '^netzstatus\t' '^netzstatus\tNetzwerk: online \(PackageKit\)$'
    fall "N15 veraltete Meldung „Keine Netzwerkverbindung“ entfernt (NM-b S3)" bash -c "grep -qP '^netz\tmeldung-entfernt$' '$LOG' && ! grep -P '^status\t' '$LOG' | tail -1 | grep -q 'Keine Netzwerkverbindung'"
    warte_layout >/dev/null; shot wieder-online
    statusbild wieder-online status-wieder-online.png
else
    sleep 33
    fall "N12 online: keine automatische Neuprüfung in 33 s" test "$(zahl '^netz\tpruefe')" = "$vp"
fi
beenden N16
abschluss
