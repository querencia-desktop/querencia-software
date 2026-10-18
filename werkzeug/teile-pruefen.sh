#!/bin/bash
# Prüft, dass die Teil-Repos, die der Workspace per Pfad einbindet, genau auf
# den getaggten Ständen stehen und sauber sind (nur lesend).
#   bash werkzeug/teile-pruefen.sh   → Exit 0 = alle passen
set -uo pipefail
P=$HOME/projects
fehler=0
while read -r ordner tag; do
    d=$P/$ordner
    kopf=$(git -C "$d" rev-parse HEAD 2>/dev/null)
    soll=$(git -C "$d" rev-parse "$tag^{commit}" 2>/dev/null)
    sauber=$(git -C "$d" status --porcelain --untracked-files=no 2>/dev/null | wc -l)
    if [ -n "$kopf" ] && [ "$kopf" = "$soll" ] && [ "$sauber" = 0 ]; then
        echo "PASS $ordner auf $tag (${kopf:0:7}), sauber"
    else
        echo "FAIL $ordner: HEAD ${kopf:0:7}, $tag = ${soll:0:7}, geänderte Dateien $sauber"; fehler=1
    fi
done <<'L'
querencia-software-paketkit paketkit/0.1.0
querencia-software-katalog katalog/0.1.0
querencia-software-transaktion transaktion/0.1.1
querencia-software-paket software-paket/0.1.1
querencia-software-ui software-ui/0.1.0
L
exit $fehler
