#!/usr/bin/env bash
# Deploy the self-driving clientless-albion orchestrator to a box.
# Usage: bootstrap.sh <ssh_host> <ssh_port>
# Provisions as root (apt/useradd) BUT deploys + RUNS the loop as a NON-ROOT user ($NRU).
# We never run the orchestrator instance as root.
set -uo pipefail
HOST="${1:?ssh_host}"; PORT="${2:?ssh_port}"
R="root@$HOST"
NRU="${NONROOT_USER:-sdanced}"          # the non-root user the loop runs as (box canonical = sdanced)
H="/home/$NRU"
SU="sudo -u $NRU"                        # run-as-non-root helper
SSH="ssh -p $PORT -o StrictHostKeyChecking=accept-new -o ConnectTimeout=15"
DEPLOY=/home/sdancer/orchestrator/clientless-deploy
ALBION=/home/sdancer/albion
ALBION_WIKI=/home/sdancer/albion-wiki
ALBION_TOOLS=/home/sdancer/albion/tools
# Operator pubkey trusted for root (provisioning) AND $NRU (run + ssh-in) on every (re)deploy.
PUBKEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJS7/xAmJAmhhjZ7PlxEpgu8k86cyiETPY5uMf382Kq1 sdancer@navi"
log(){ echo "[$(date -u +%H:%M:%SZ)] $*"; }

log "1/6 wait for ssh ($R:$PORT)"
up=0; for i in $(seq 1 45); do $SSH $R true 2>/dev/null && { up=1; break; }; sleep 10; done
[ $up = 1 ] || { log "SSH never came up"; exit 1; }
log "ssh up"

log "2/6 base deps (apt + rust + claude cli) [root, provisioning only]"
$SSH $R 'set -e; export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq >/dev/null 2>&1 || true
  apt-get install -y -qq git curl python3 python3-pip python3-venv build-essential pkg-config libssl-dev rsync ca-certificates sudo >/dev/null 2>&1 || true
  command -v cargo >/dev/null 2>&1 || { curl -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1; }
  command -v claude >/dev/null 2>&1 || { curl -fsSL https://claude.ai/install.sh | bash >/dev/null 2>&1 || true; }
  command -v claude >/dev/null 2>&1 || { curl -fsSL https://deb.nodesource.com/setup_20.x | bash - >/dev/null 2>&1; apt-get install -y -qq nodejs >/dev/null 2>&1; npm i -g @anthropic-ai/claude-code >/dev/null 2>&1; }
  echo "claude=$(command -v claude || echo MISSING) cargo=$(command -v cargo || echo MISSING)"'

log "2.5/6 access self-heal + provision the NON-ROOT user ($NRU): trusted pubkey + passwordless sudo"
$SSH $R "set -e
  mkdir -p /root/.ssh; chmod 700 /root/.ssh
  grep -qF '$PUBKEY' /root/.ssh/authorized_keys 2>/dev/null || echo '$PUBKEY' >> /root/.ssh/authorized_keys
  chmod 600 /root/.ssh/authorized_keys
  id $NRU >/dev/null 2>&1 || useradd -m -s /bin/bash $NRU
  mkdir -p $H/.ssh $H/.claude/commands $H/clientless/analysis; chmod 700 $H/.ssh
  grep -qF '$PUBKEY' $H/.ssh/authorized_keys 2>/dev/null || echo '$PUBKEY' >> $H/.ssh/authorized_keys
  chmod 600 $H/.ssh/authorized_keys
  echo '$NRU ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/90-$NRU; chmod 440 /etc/sudoers.d/90-$NRU
  chown -R $NRU:$NRU $H/.ssh $H/.claude $H/clientless
  echo \"self-heal OK: root+$NRU trust the operator key; $NRU has sudo\""

log "3/6 copy claude login + lean skill -> $NRU home"
scp -P $PORT -o StrictHostKeyChecking=accept-new /home/sdancer/.claude/.credentials.json $R:$H/.claude/.credentials.json
scp -P $PORT -o StrictHostKeyChecking=accept-new /home/sdancer/.claude/commands/orchestrate-lean.md $R:$H/.claude/commands/orchestrate-lean.md
$SSH $R "chown -R $NRU:$NRU $H/.claude; chmod 600 $H/.claude/.credentials.json"

log "4/6 transfer clientless code + deploy files -> $H/clientless (owned by $NRU)"
rsync -az -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" \
  --exclude '.git' --exclude 'target' --exclude '*.so' --exclude '*.png' --exclude '*.jpg' \
  "$ALBION/clientless-bot" "$ALBION/crates" "$ALBION/gamestate" "$ALBION/bin" \
  "$ALBION/Cargo.toml" "$ALBION/Cargo.lock" $R:$H/clientless/ 2>&1 | tail -1
rsync -az -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" "$DEPLOY/analysis/" $R:$H/clientless/analysis/
rsync -az -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" "$DEPLOY/onbox/" $R:$H/clientless/onbox/
$SSH $R "chown -R $NRU:$NRU $H/clientless; chmod +x $H/clientless/onbox/*.sh"

log "4.5/6 mirror knowledge base + proven Albion tools -> $NRU home"
$SSH $R "mkdir -p $H/albion-wiki $H/albion/tools; chown -R $NRU:$NRU $H/albion-wiki $H/albion"
rsync -az --delete -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" \
  --exclude '.git' "$ALBION_WIKI/" $R:$H/albion-wiki/ 2>&1 | tail -1
rsync -az --delete -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" \
  --exclude '.git' --exclude 'target' --exclude '__pycache__' --exclude '*.pyc' \
  "$ALBION_TOOLS/" $R:$H/albion/tools/ 2>&1 | tail -1
$SSH $R "chown -R $NRU:$NRU $H/albion-wiki $H/albion; test -f $H/albion-wiki/WIKI.md; test -f $H/albion/tools/input/navi-e2e/albion_e2e.py; test -f $H/albion/tools/input/albion_ingame_register_login.py"

log "5/6 smoke: claude auth on box (as $NRU)"
$SSH $R "$SU bash -lc 'cd ~/clientless && timeout 120 claude --dangerously-skip-permissions -p \"Reply with exactly: AUTH_OK\" 2>&1 | tail -5'"

log "6/6 start self-driving loop AS $NRU (never root)"
$SSH $R "$SU bash -lc 'cd ~/clientless && setsid --fork bash onbox/loop.sh >/dev/null 2>&1 </dev/null'; sleep 3; $SU bash -lc 'pgrep -fu $NRU onbox/loop.sh | tr \"\n\" \" \"; tail -3 ~/clientless/analysis/loop.log 2>/dev/null'"
log "DONE — box self-driving on clientless_albion_bot as NON-ROOT $NRU. Tail: ssh -p $PORT $R 'sudo -u $NRU tail -f $H/clientless/analysis/loop.log'"
