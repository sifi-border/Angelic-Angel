#!/bin/sh
# Installs Angelic Angel as a systemd service. Build first (cargo build --release), then:
#   sudo sh deploy/setup.sh
# Creates the service user, directories, binary and unit. Does not start anything.
# Afterwards: put the config at /var/lib/angelic-angel/angelic-angel.toml (600,
# owned by angelic-angel), fill /etc/angelic-angel/discord.env, then
#   sudo systemctl enable --now angelic-angel
set -eu

DEPLOY_DIR=$(cd "$(dirname "$0")" && pwd)
SRC_BIN="$DEPLOY_DIR/../target/release/angelic-angel"

# Dedicated system user without login or home directory.
if ! id angelic-angel >/dev/null 2>&1; then
    useradd --system --user-group --no-create-home \
        --home-dir /var/lib/angelic-angel --shell /usr/sbin/nologin angelic-angel
fi

install -m 0755 -o root -g root "$SRC_BIN" /usr/local/bin/angelic-angel

# State directory for the config (written by listen). StateDirectory= also manages it.
install -d -m 0700 -o angelic-angel -g angelic-angel /var/lib/angelic-angel

# Env file for the Discord webhook URL: created empty, never overwritten.
install -d -m 0700 -o root -g root /etc/angelic-angel
if [ ! -e /etc/angelic-angel/discord.env ]; then
    install -m 0600 -o root -g root /dev/null /etc/angelic-angel/discord.env
fi

install -m 0644 -o root -g root "$DEPLOY_DIR/angelic-angel.service" \
    /etc/systemd/system/angelic-angel.service
systemd-analyze verify /etc/systemd/system/angelic-angel.service
systemctl daemon-reload

echo "done. Not enabled or started."
