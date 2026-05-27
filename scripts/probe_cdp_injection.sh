#!/usr/bin/env bash
set -euo pipefail

SSH_HOST="${SSH_HOST:-root@ssh6.vast.ai}"
SSH_PORT="${SSH_PORT:-29576}"
SSH_OPTS=(-o BatchMode=yes -p "$SSH_PORT")

ssh_remote() {
  ssh "${SSH_OPTS[@]}" "$SSH_HOST" "$@"
}

launcher_bin="$(
  ssh_remote 'for candidate in \
    /home/albion/albion-launcher/data/launcher/Albion-Online \
    /home/albion/albion-online/Albion-Online; do
      if [ -x "$candidate" ]; then
        printf "%s\n" "$candidate"
        break
      fi
    done'
)"

if [[ -z "$launcher_bin" ]]; then
  echo "launcher binary not found under /home/albion" >&2
  exit 2
fi

launcher_dir="$(dirname "$launcher_bin")"

dir_hits="$(
  ssh_remote "ls -1 \"$launcher_dir\" | grep -iE 'QtWebEngine|electron|chrome-sandbox|app.asar|resources' || true"
)"

link_hits="$(
  ssh_remote "ldd \"$launcher_bin\" | grep -iE 'Qt5WebEngine|Qt5Core|Qt5Qml|electron|cef|chrome|nss|nspr' || true"
)"

proc_hits="$(
  ssh_remote "pgrep -a -f '^/home/albion/albion-launcher/data/launcher/QtWebEngineProcess' || true; pgrep -a -x electron || true"
)"

echo "launcher_bin=$launcher_bin"
echo
echo "[dir hits]"
printf '%s\n' "${dir_hits:-<none>}"
echo
echo "[ldd hits]"
printf '%s\n' "${link_hits:-<none>}"
echo
echo "[process hits]"
printf '%s\n' "${proc_hits:-<none>}"
echo

combined="$dir_hits
$link_hits
$proc_hits"

if grep -qiE 'electron|app\.asar|chrome-sandbox' <<<"$combined"; then
  outcome="electron-likely"
elif grep -qiE 'QtWebEngineProcess|Qt5WebEngine' <<<"$combined"; then
  outcome="not-electron"
else
  outcome="indeterminate"
fi

echo "outcome=$outcome"

case "$outcome" in
  electron-likely)
    echo "Next step: relaunch with --remote-debugging-port=9222 and probe /json/list." >&2
    ;;
  not-electron)
    echo "QtWebEngine detected; stop before any CDP injection attempt." >&2
    ;;
  *)
    echo "No decisive Electron or QtWebEngine markers found." >&2
    ;;
esac
