#!/usr/bin/env bash
# Deploy the self-driving clientless-albion orchestrator to a box.
# Usage: bootstrap.sh <ssh_host> <ssh_port>
#
# Reinstall note: the clean reprovision flow is Vultr OS reinstall, then rerun
# this script. The Vultr API path is POST /v2/instances/<instance-id>/reinstall
# with the host-side VULTR_API_KEY; if the API key is IP-allowlisted, use the
# Vultr dashboard reinstall action instead, then remove the old ssh host key:
#   ssh-keygen -R <host-or-ip>
# and rerun bootstrap so StrictHostKeyChecking=accept-new records the new key.
#
# Provisions as root (apt/useradd) BUT deploys + RUNS the loop as a NON-ROOT user ($NRU).
# We never run the orchestrator instance as root.
set -euo pipefail
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
ORCH=/home/sdancer/orchestrator
HARNESS_SRC=$ORCH/harness-rs
DASHBOARD_SRC=/home/sdancer/orch-rust-dash/dashboard
POOL_INVENTORY=$ORCH/analysis/pool_inventory.json
ENV_FILE=$ORCH/.env
SPACETIME_VERSION=2.1.0
SPACETIME_RELEASE_ROOT="https://github.com/clockworklabs/SpacetimeDB/releases/download/v$SPACETIME_VERSION"
HARNESS_SERVER_LOCAL=http://127.0.0.1:3001
HARNESS_DATABASE_LOCAL=orchestrator-box
# Operator pubkey trusted for root (provisioning) AND $NRU (run + ssh-in) on every (re)deploy.
PUBKEY="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJS7/xAmJAmhhjZ7PlxEpgu8k86cyiETPY5uMf382Kq1 sdancer@navi"
log(){ echo "[$(date -u +%H:%M:%SZ)] $*"; }

if [ -f "$ENV_FILE" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

log "1/6 wait for ssh ($R:$PORT)"
up=0; for i in $(seq 1 45); do $SSH $R true 2>/dev/null && { up=1; break; }; sleep 10; done
[ $up = 1 ] || { log "SSH never came up"; exit 1; }
log "ssh up"

log "2/6 base deps (apt + rust + claude cli) [root, provisioning only]"
$SSH $R 'set -e; export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq >/dev/null 2>&1 || true
  apt-get install -y -qq git curl python3 python3-pip python3-venv build-essential pkg-config libssl-dev rsync ca-certificates sudo cron jq lsof >/dev/null 2>&1 || true
  command -v cargo >/dev/null 2>&1 || { curl -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1; }
  command -v claude >/dev/null 2>&1 || { curl -fsSL https://claude.ai/install.sh | bash >/dev/null 2>&1 || true; }
  command -v claude >/dev/null 2>&1 || { curl -fsSL https://deb.nodesource.com/setup_20.x | bash - >/dev/null 2>&1; apt-get install -y -qq nodejs >/dev/null 2>&1; npm i -g @anthropic-ai/claude-code >/dev/null 2>&1; }
  command -v cloudflared >/dev/null 2>&1 || { curl -fsSL -o /tmp/cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb && dpkg -i /tmp/cloudflared.deb >/dev/null 2>&1; rm -f /tmp/cloudflared.deb; }
  echo "claude=$(command -v claude || echo MISSING) cargo=$(command -v cargo || echo MISSING) cloudflared=$(command -v cloudflared || echo MISSING)"'

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
$SSH $R "$SU bash -lc 'command -v cargo >/dev/null 2>&1 || curl -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1; source $H/.cargo/env 2>/dev/null || true; command -v cargo'"

log "3/6 copy claude login + lean skill -> $NRU home"
scp -P $PORT -o StrictHostKeyChecking=accept-new /home/sdancer/.claude/.credentials.json $R:$H/.claude/.credentials.json
scp -P $PORT -o StrictHostKeyChecking=accept-new /home/sdancer/.claude/commands/orchestrate-lean.md $R:$H/.claude/commands/orchestrate-lean.md
$SSH $R "chown -R $NRU:$NRU $H/.claude; chmod 600 $H/.claude/.credentials.json"

log "3.1/6 seed pool ssh key + live pool inventory"
if [ -f /home/sdancer/.ssh/id_ed25519 ]; then
  scp -P $PORT -o StrictHostKeyChecking=accept-new /home/sdancer/.ssh/id_ed25519 $R:$H/.ssh/id_ed25519
fi
if [ -f /home/sdancer/.ssh/id_ed25519.pub ]; then
  scp -P $PORT -o StrictHostKeyChecking=accept-new /home/sdancer/.ssh/id_ed25519.pub $R:$H/.ssh/id_ed25519.pub
fi
if [ -f "$POOL_INVENTORY" ]; then
  scp -P $PORT -o StrictHostKeyChecking=accept-new "$POOL_INVENTORY" $R:$H/clientless/pool.json
fi
$SSH $R "chown $NRU:$NRU $H/.ssh/id_ed25519 $H/.ssh/id_ed25519.pub $H/clientless/pool.json 2>/dev/null || true; chmod 600 $H/.ssh/id_ed25519 2>/dev/null || true; chmod 644 $H/.ssh/id_ed25519.pub $H/clientless/pool.json 2>/dev/null || true"

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

log "4.6/6 install SpacetimeDB $SPACETIME_VERSION, harness module, and Rust dashboard"
$SSH $R "set -e
  mkdir -p $H/.local/bin $H/.local/share/spacetime/data $H/.config/spacetime $H/orchestrator /home/sdancer/orchestrator $H/orch-rust-dash
  chown -R $NRU:$NRU $H/.local $H/.config $H/orchestrator $H/orch-rust-dash /home/sdancer/orchestrator"
$SSH $R "$SU bash -lc 'if ! $H/.local/bin/spacetime --version 2>/dev/null | grep -q \"spacetimedb tool version $SPACETIME_VERSION\"; then curl -fsSL https://install.spacetimedb.com | SPACETIME_DOWNLOAD_ROOT=$SPACETIME_RELEASE_ROOT sh -s -- -y; fi; $H/.local/bin/spacetime --version | sed -n \"1,3p\"'"
rsync -az --delete -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" \
  --exclude 'target' --exclude '.git' "$HARNESS_SRC/" $R:$H/orchestrator/harness-rs/ 2>&1 | tail -1
rsync -az --delete -e "ssh -p $PORT -o StrictHostKeyChecking=accept-new" \
  --exclude 'target' --exclude '.git' "$DASHBOARD_SRC/" $R:$H/orch-rust-dash/dashboard/ 2>&1 | tail -1
$SSH $R "chown -R $NRU:$NRU $H/orchestrator $H/orch-rust-dash"
$SSH $R "$SU bash -lc 'source $H/.cargo/env 2>/dev/null || true; env -C $H/orchestrator/harness-rs cargo build --release --features cli; install -m 0755 $H/orchestrator/harness-rs/target/release/harness $H/orchestrator/harness'"
$SSH $R "$SU bash -lc 'source $H/.cargo/env 2>/dev/null || true; env -C $H/orch-rust-dash/dashboard cargo build --release'"
$SSH $R "set -e
  cat > $H/orchestrator/harness.toml <<'EOF'
# Harness configuration
database = \"$HARNESS_DATABASE_LOCAL\"
server = \"$HARNESS_SERVER_LOCAL\"
biome_api_key = \"welcome to my w0rld\"
EOF
  cat > /home/sdancer/orchestrator/harness.toml <<'EOF'
# Harness configuration
database = \"$HARNESS_DATABASE_LOCAL\"
server = \"$HARNESS_SERVER_LOCAL\"
biome_api_key = \"welcome to my w0rld\"
EOF
  cp $H/orchestrator/harness /home/sdancer/orchestrator/harness
  cat > $H/orch-rust-dash/dashboard/harness.toml <<'EOF'
server = \"$HARNESS_SERVER_LOCAL\"
database = \"$HARNESS_DATABASE_LOCAL\"
EOF
  chown -R $NRU:$NRU $H/orchestrator /home/sdancer/orchestrator $H/orch-rust-dash/dashboard/harness.toml"

log "4.7/6 install systemd units for SpacetimeDB, dashboard, cloudflared, and lean loop"
if [ -n "${CLOUDFLARE_API_TOKEN:-}" ]; then
  printf 'TUNNEL_TOKEN=%s\n' "$CLOUDFLARE_API_TOKEN" | $SSH $R "umask 077; mkdir -p /etc/clientless-orchestrator; cat > /etc/clientless-orchestrator/cloudflared.env"
else
  log "CLOUDFLARE_API_TOKEN not set locally; preserving any existing /etc/clientless-orchestrator/cloudflared.env"
fi
$SSH $R "cat > /etc/systemd/system/spacetimedb-box.service <<'EOF'
[Unit]
Description=SpacetimeDB local orchestrator node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$NRU
Group=$NRU
WorkingDirectory=$H
Environment=HOME=$H
Environment=PATH=$H/.local/bin:$H/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ExecStart=$H/.local/bin/spacetime start --listen-addr 127.0.0.1:3001 --data-dir $H/.local/share/spacetime/data --non-interactive
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
cat > /etc/systemd/system/orchestrator-dashboard.service <<'EOF'
[Unit]
Description=Clientless orchestrator Rust dashboard
After=network-online.target spacetimedb-box.service
Wants=network-online.target spacetimedb-box.service

[Service]
Type=simple
User=$NRU
Group=$NRU
WorkingDirectory=$H/orch-rust-dash/dashboard
Environment=HOME=$H
Environment=PATH=$H/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ExecStart=$H/orch-rust-dash/dashboard/target/release/orchestrator-dashboard
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
cat > /etc/systemd/system/cloudflared-sg2.service <<'EOF'
[Unit]
Description=Cloudflared sg2 tunnel to clientless dashboard
After=network-online.target orchestrator-dashboard.service
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=-/etc/clientless-orchestrator/cloudflared.env
ExecStart=/usr/local/bin/cloudflared tunnel run --token \${TUNNEL_TOKEN}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF
cat > /etc/systemd/system/clientless-lean-loop.service <<'EOF'
[Unit]
Description=Clientless Albion DB-wired lean loop
After=network-online.target spacetimedb-box.service
Wants=network-online.target spacetimedb-box.service

[Service]
Type=simple
User=$NRU
Group=$NRU
WorkingDirectory=$H/clientless
Environment=HOME=$H
Environment=IS_SANDBOX=1
Environment=PATH=$H/.local/bin:$H/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ExecStart=/usr/bin/bash $H/clientless/onbox/loop.sh
Restart=always
RestartSec=10
TimeoutStopSec=10

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable spacetimedb-box.service orchestrator-dashboard.service cloudflared-sg2.service clientless-lean-loop.service >/dev/null
if ! systemctl is-active --quiet spacetimedb-box.service; then
  if pgrep -u $NRU -f spacetimedb-standalone >/dev/null; then echo 'spacetimedb unmanaged process already running; enabled unit for reboot'; else systemctl start spacetimedb-box.service; fi
fi
if ! systemctl is-active --quiet orchestrator-dashboard.service; then
  if ss -ltn '( sport = :3030 )' | grep -q ':3030'; then echo 'dashboard already listening on :3030; enabled unit for reboot'; else systemctl start orchestrator-dashboard.service; fi
fi
if ! systemctl is-active --quiet cloudflared-sg2.service; then
  if pgrep -f 'cloudflared tunnel run' >/dev/null; then echo 'cloudflared unmanaged process already running; enabled unit for reboot'; elif [ -s /etc/clientless-orchestrator/cloudflared.env ]; then systemctl start cloudflared-sg2.service; else echo 'cloudflared token env missing; unit enabled but not started'; fi
fi
if ! systemctl is-active --quiet clientless-lean-loop.service; then
  if pgrep -u $NRU -f '$H/clientless/onbox/loop.sh' >/dev/null; then echo 'lean loop unmanaged process already running; enabled unit for reboot'; else systemctl start clientless-lean-loop.service; fi
fi"

log "4.8/6 publish harness module if orchestrator-box is absent"
$SSH $R "$SU bash -lc 'for i in \$(seq 1 30); do $H/orchestrator/harness --server $HARNESS_SERVER_LOCAL --database $HARNESS_DATABASE_LOCAL facts >/dev/null 2>&1 && exit 0; sleep 1; done; env -C $H/orchestrator/harness-rs $H/.local/bin/spacetime publish --server $HARNESS_SERVER_LOCAL -y --delete-data=never $HARNESS_DATABASE_LOCAL; $H/orchestrator/harness --server $HARNESS_SERVER_LOCAL --database $HARNESS_DATABASE_LOCAL facts >/dev/null'"

log "4.9/6 keep legacy paths and cron-compatible projections consistent"
$SSH $R "set -e
  (crontab -u $NRU -l 2>/dev/null | grep -v 'clientless/onbox/gt2paths.py' || true; echo '*/2 * * * * python3 $H/clientless/onbox/gt2paths.py >/dev/null 2>&1') | crontab -u $NRU -
  $SU python3 $H/clientless/onbox/gt2paths.py >/dev/null 2>&1 || true"

log "5/6 smoke: claude auth on box (as $NRU)"
$SSH $R "$SU bash -lc 'env -C ~/clientless timeout 120 claude --dangerously-skip-permissions -p \"Reply with exactly: AUTH_OK\" 2>&1 | tail -5'"

log "6/6 health check: DB, dashboard, tunnel, loop"
$SSH $R "set -e
  $SU $H/orchestrator/harness --server $HARNESS_SERVER_LOCAL --database $HARNESS_DATABASE_LOCAL facts >/dev/null
  curl -fsSI http://127.0.0.1:3030/ | sed -n '1,3p'
  systemctl is-enabled spacetimedb-box.service orchestrator-dashboard.service cloudflared-sg2.service clientless-lean-loop.service
  pgrep -fu $NRU '$H/clientless/onbox/loop.sh' | tr '\n' ' '; echo
  tail -3 $H/clientless/analysis/loop.log 2>/dev/null || true"
log "DONE - box reprovisioning converged without intentionally disrupting live unmanaged processes."
