# hjkl-for-mac

Karabiner-Elements で使っていた `;` ベースのキーバインドを、Rust の常駐 CLI アプリとして実装したものです。

| 入力 | 出力 |
| --- | --- |
| `;` を単独で押す | `;` |
| `;` を押しながら `h` | 左矢印 |
| `;` を押しながら `j` | 下矢印 |
| `;` を押しながら `k` | 上矢印 |
| `;` を押しながら `l` | 右矢印 |
| `;` を押しながらその他のキー | `Command` + そのキー |

実装は macOS の `CGEventTap` を使ってキーボードイベントを監視・書き換えます。外部 Rust crate には依存していません。

## Requirements

- macOS
- Rust toolchain
- macOS Accessibility permission

## Install as a LaunchAgent

ログイン中に常駐させるには、macOS の per-user LaunchAgent としてインストールします。

```sh
scripts/install-launch-agent.sh
```

このスクリプトは次を行います。

1. `cargo build --release`
2. バイナリを `~/.local/bin/hjkl-for-mac` にコピー
3. LaunchAgent plist を `~/Library/LaunchAgents/com.kazuki-hanai.hjkl-for-mac.plist` に生成
4. `launchctl bootstrap` / `kickstart` で起動

インストール後、System Settings → Privacy & Security → Accessibility で以下のバイナリを許可してください。

```text
~/.local/bin/hjkl-for-mac
```

権限がまだない状態で LaunchAgent が起動しても、`--daemon` モードではプロセスが終了せず、30秒ごとに低負荷でイベントタップ作成を再試行します。

## Manage the LaunchAgent

```sh
# 状態確認
launchctl print gui/$(id -u)/com.kazuki-hanai.hjkl-for-mac

# 再起動
launchctl kickstart -k gui/$(id -u)/com.kazuki-hanai.hjkl-for-mac

# アンインストール（plistのみ削除、バイナリは残す）
scripts/uninstall-launch-agent.sh

# バイナリも削除
REMOVE_BINARY=1 scripts/uninstall-launch-agent.sh
```

ログは以下に出力されます。

```text
~/Library/Logs/hjkl-for-mac.log
~/Library/Logs/hjkl-for-mac.err.log
```

## Run manually

```sh
cargo run --release
```

または:

```sh
target/release/hjkl-for-mac
```

実行中だけキーリマップが有効です。停止するにはターミナルで `Ctrl-C` を押してください。

## CLI

```sh
hjkl-for-mac --help
hjkl-for-mac --version
hjkl-for-mac --daemon
```

`--daemon` は LaunchAgent 用です。macOS 権限がまだ付与されていない場合でも終了せず、30秒ごとに再試行します。

## macOS permissions

初回実行時、macOS の権限がないとイベントタップを作成できません。その場合は次を許可してください。

1. System Settings
2. Privacy & Security
3. Accessibility
4. 必要に応じて Input Monitoring
5. 手動実行の場合は Terminal/iTerm、LaunchAgent 実行の場合は `~/.local/bin/hjkl-for-mac` を許可

## 元の Karabiner 設定との対応

元の Karabiner-Elements 設定では、`;` を他キーと同時押しした場合に `right_command` として扱い、その上で `right_command + hjkl` を矢印キーにしていました。

この Rust 実装では、`hjkl` は矢印キーへ変換し、それ以外のキーではイベントに `Command` フラグを付与します。そのため `; + c` のようなショートカットは `Command + c` として動作します。

## 消費電力について

ほとんど消費しません。理由は実装がポーリングではなく **イベント駆動** だからです。

- `CFRunLoopRun()` はイベントが届くまでスレッドをスリープさせます。入力していない間は CPU をほぼ使いません（アイドル時 CPU 使用率はほぼ 0%）。
- 購読しているのは `keyDown` / `keyUp` の 2 種類だけです（`mask = keyDown | keyUp`）。マウス移動などの高頻度イベントでは起こされません。
- 1 キーあたりの処理はフィールドの読み書き程度で、無視できるコストです。

Karabiner-Elements のような常駐リマッパーと同程度で、ノート PC で 24 時間動かしても電池への影響は実用上ほぼありません。

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
plutil -lint launchd/com.kazuki-hanai.hjkl-for-mac.plist.template
sh -n scripts/install-launch-agent.sh scripts/uninstall-launch-agent.sh
```

## OSS status

This repository is ready for public OSS use with:

- MIT license
- Cargo package metadata
- LaunchAgent installer/uninstaller scripts
- GitHub Actions CI
- contribution and security notes

## License

MIT
