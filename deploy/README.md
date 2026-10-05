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

- Logs show only warnings and errors. For more, set `RUST_LOG` with a drop-in (`info` shows connections and notifications, `debug` adds pings and decryption details):

  ```sh
  sudo systemctl edit angelic-angel     # add the two lines below, save
  #   [Service]
  #   Environment=RUST_LOG=angelic_angel=info
  sudo systemctl restart angelic-angel
  ```

  These logs include each payload and the push endpoint, so remove the drop-in afterwards with `sudo systemctl revert angelic-angel`.
- The service restarts on failure after 30 seconds, except on exit status 3 (re-registration with X failed). Check the cause first; `register` calls the X API, so run it only once the cause is fixed:

  ```sh
  journalctl -u angelic-angel | grep 're-registration'
  ```

  - **401 / 403** (e.g. `push subscription registration failed (401 Unauthorized)`): the `auth_token` / `ct0` cookies have expired. Run `init` first to enter new ones. `init` writes a new config without the registration, which `register` then creates again.
  - **Network or file errors**: fix the cause, then `register` as is.

  ```sh
  sudo systemctl stop angelic-angel
  sudo -u angelic-angel angelic-angel -c /var/lib/angelic-angel/angelic-angel.toml init      # only for 401/403
  sudo -u angelic-angel angelic-angel -c /var/lib/angelic-angel/angelic-angel.toml register
  sudo systemctl start angelic-angel
  ```

- To update: `git pull && cargo build --release && sudo sh deploy/setup.sh && sudo systemctl restart angelic-angel`. `setup.sh` keeps the existing config and `discord.env`.
