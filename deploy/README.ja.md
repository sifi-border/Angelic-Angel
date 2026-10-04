# Angelic Angel を systemd で運用する

[English](README.md)

このディレクトリには、Linux サーバーで `listen` を常駐させるための systemd ユニットとセットアップスクリプトがあります。サービスは専用ユーザー `angelic-angel` で動き、ユニットで権限を絞っています (`ProtectSystem=strict`、特権なし、システムコールの制限など)。

## セットアップ

1. サーバー上でビルドし、リポジトリのルートでセットアップスクリプトを実行します (`pkg-config`、`libssl-dev`、C コンパイラ、Rust 1.85 以降が必要)。

   ```sh
   cargo build --release
   sudo sh deploy/setup.sh
   ```

   `setup.sh` は、システムユーザー `angelic-angel` の作成、`/usr/local/bin` へのバイナリの配置、`/var/lib/angelic-angel` と空の `/etc/angelic-angel/discord.env` (`0600`、既存なら上書きしない) の作成、ユニットの配置と systemd の再読み込みを行います。サービスの起動はしません。

2. 登録済みの設定ファイルを配置します。`init` と `register` は別の環境 (またはサーバー上) で済ませてファイルをコピーしてください。サーバーのためだけに登録をやり直す必要はありません。

   ```sh
   sudo install -m 600 -o angelic-angel -g angelic-angel angelic-angel.toml /var/lib/angelic-angel/
   ```

3. `sudoedit /etc/angelic-angel/discord.env` で Webhook を設定します。

   ```sh
   WEBHOOK_FORMAT=discord
   WEBHOOK_ENDPOINT=https://discord.com/api/webhooks/ID/TOKEN
   ```

4. 同じ設定ファイルを使うほかの `listen` を止めてから (同じ UAID で二重に接続しないため)、サービスを起動します。

   ```sh
   sudo systemctl enable --now angelic-angel
   ```

| パス | 内容 |
|------|------|
| `/usr/local/bin/angelic-angel` | バイナリ |
| `/var/lib/angelic-angel/angelic-angel.toml` | 認証情報と鍵を含む設定。UAID が変わると `listen` が書き換えます |
| `/etc/angelic-angel/discord.env` | `WEBHOOK_FORMAT` と `WEBHOOK_ENDPOINT` (URL は認証情報です) |
| `/etc/systemd/system/angelic-angel.service` | ユニット |

## 運用

```sh
systemctl status angelic-angel              # 状態
journalctl -u angelic-angel -f              # ログを追う
sudo systemctl restart angelic-angel        # 再起動 (discord.env を変更したあとなど)
sudo systemctl disable --now angelic-angel  # 停止して自動起動を無効化
```

- ログに出るのは警告とエラーだけです。デバッグログが必要なときは、`ExecStart` の `-c` の前に `-v` を付けて `sudo systemctl daemon-reload && sudo systemctl restart angelic-angel` を実行します。デバッグログには通知ごとのペイロードとプッシュエンドポイントが含まれるため、確認が済んだら `-v` を外してください。
- 異常終了すると 30 秒後に再起動します。ただし終了コード 3 (X への再登録の失敗) のときは再起動しません。その場合は `register` をやり直してからサービスを起動します。

  ```sh
  sudo systemctl stop angelic-angel
  sudo -u angelic-angel angelic-angel -c /var/lib/angelic-angel/angelic-angel.toml register
  sudo systemctl start angelic-angel
  ```

- 更新するとき: `git pull && cargo build --release && sudo sh deploy/setup.sh && sudo systemctl restart angelic-angel`。`setup.sh` は既存の設定ファイルと `discord.env` を残します。
