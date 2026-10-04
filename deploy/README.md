# Running Angelic Angel as a systemd service

[日本語版](README.ja.md)

This directory contains a systemd unit and a setup script for running `listen` permanently on a Linux server. The service runs as a dedicated `angelic-angel` user with a sandboxed unit (`ProtectSystem=strict`, no capabilities, restricted system calls).

## Setup

1. Build on the server and run the setup script from the repository root (requires `pkg-config`, `libssl-dev`, a C compiler and Rust 1.85+):

   ```sh
   cargo build --release
   sudo sh deploy/setup.sh
   ```

   `setup.sh` creates the `angelic-angel` system user, installs the binary to `/usr/local/bin`, creates `/var/lib/angelic-angel` and an empty `/etc/angelic-angel/discord.env` (`0600`, never overwritten), installs the unit and reloads systemd. It does not start anything.

2. Put an already registered config in place. Run `init` and `register` elsewhere (or on the server), then copy the file; do not register again just for the server:

   ```sh
   sudo install -m 600 -o angelic-angel -g angelic-angel angelic-angel.toml /var/lib/angelic-angel/
   ```

3. Set the webhook with `sudoedit /etc/angelic-angel/discord.env`:

   ```sh
   WEBHOOK_FORMAT=discord
   WEBHOOK_ENDPOINT=https://discord.com/api/webhooks/ID/TOKEN
   ```

4. Stop any other `listen` that uses the same config (the same UAID must not be connected twice), then start the service:

   ```sh
   sudo systemctl enable --now angelic-angel
   ```

| Path | Contents |
|------|----------|
| `/usr/local/bin/angelic-angel` | Binary |
| `/var/lib/angelic-angel/angelic-angel.toml` | Config with credentials and keys; rewritten by `listen` when the UAID changes |
| `/etc/angelic-angel/discord.env` | `WEBHOOK_FORMAT` and `WEBHOOK_ENDPOINT` (the URL is a credential) |
| `/etc/systemd/system/angelic-angel.service` | Unit |

## Operation

```sh
systemctl status angelic-angel              # state
journalctl -u angelic-angel -f              # follow logs
sudo systemctl restart angelic-angel        # restart (e.g. after editing discord.env)
sudo systemctl disable --now angelic-angel  # stop and disable
```

- Logs show only warnings and errors. For debug logs, add `-v` before `-c` in `ExecStart`, then `sudo systemctl daemon-reload && sudo systemctl restart angelic-angel`. Debug logs include each payload and the push endpoint, so remove `-v` again afterwards.
- The service restarts on failure after 30 seconds, except on exit status 3 (re-registration with X failed). In that case, run `register` again and start the service:

  ```sh
  sudo systemctl stop angelic-angel
  sudo -u angelic-angel angelic-angel -c /var/lib/angelic-angel/angelic-angel.toml register
  sudo systemctl start angelic-angel
  ```

- To update: `git pull && cargo build --release && sudo sh deploy/setup.sh && sudo systemctl restart angelic-angel`. `setup.sh` keeps the existing config and `discord.env`.
