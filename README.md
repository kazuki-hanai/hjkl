# hjkl-for-mac

Karabiner-Elements で使っていた `;` ベースのキーバインドを、Rust の常駐 CLI アプリとして実装したものです。外部 Rust crate には依存せず、macOS の `CGEventTap` でキーボードイベントを監視・書き換えます。

| 入力 | 出力 |
| --- | --- |
| `;` を単独で押す | `;` |
| `;` を押しながら `h` | 左矢印 |
| `;` を押しながら `j` | 下矢印 |
| `;` を押しながら `k` | 上矢印 |
| `;` を押しながら `l` | 右矢印 |
| `;` を押しながらその他のキー | `Command` + そのキー |

## Requirements

- macOS
- Rust toolchain（ビルド時のみ）
- macOS Accessibility permission

## Quick start

```sh
# ビルドして ~/.local/bin にインストールし、ログイン時に自動起動するよう有効化
scripts/install.sh
```

`hjkl` コマンドをそのまま使うには `~/.local/bin` を `PATH` に入れてください。未設定の場合でも、`~/.local/bin/hjkl start` のように絶対パスで実行できます。

インストール後、System Settings → Privacy & Security → Accessibility で次のバイナリを許可してください。

```text
~/.local/bin/hjkl
```

許可したら再起動します。

```sh
hjkl restart
```

## Service commands（デーモン操作）

バイナリ自体が per-user の launchd LaunchAgent を管理します。シェルスクリプトは不要で、サブコマンドだけで完結します。

```sh
hjkl start      # 今すぐ裏で起動（ログイン時の自動起動はしない）
hjkl stop       # 停止
hjkl restart    # 再起動
hjkl enable     # ログイン時に自動起動するよう登録（今すぐ起動もする）
hjkl disable    # 自動起動を解除して停止
hjkl status     # 有効/起動状態と各種パスを表示
```

- `start` … その場で裏に立ち上げます。ログイン時の自動起動はしません。
- `enable` … `~/Library/LaunchAgents` に plist を置くことで、次回以降のログインでも勝手に立ち上がります。今すぐの起動も行います。
- `disable` … plist を削除し、自動起動を解除して停止します。
- `stop` … 今のプロセスを止めるだけで、`enable` 済みなら次回ログインでまた起動します。

launchctl を直接使う必要はありませんが、内部的には `gui/<uid>/com.kazuki-hanai.hjkl-for-mac` を操作しています。

## Run in foreground（お試し）

```sh
cargo run --release
# または
target/release/hjkl
```

実行中だけリマップが有効です。`Ctrl-C` で停止します。

## CLI

```sh
hjkl --help
hjkl --version
```

`start` / `enable` は LaunchAgent 経由でバックグラウンド起動します。内部的には launchd が foreground プロセスを管理するため、手動で `run --launchd` を実行する必要はありません。

## macOS permissions

Accessibility 権限がないとイベントタップを作成できません。

1. System Settings
2. Privacy & Security
3. Accessibility
4. LaunchAgent 実行時は `~/.local/bin/hjkl`、手動実行時は Terminal/iTerm を許可
5. 反映されないときは `hjkl restart`

## Uninstall

```sh
scripts/uninstall.sh                 # 自動起動を解除して停止（バイナリは残す）
scripts/uninstall.sh --remove-binary # バイナリも削除
```

## 元の Karabiner 設定との対応

元の Karabiner-Elements 設定では、`;` を他キーと同時押しした場合に `right_command` として扱い、その上で `right_command + hjkl` を矢印キーにしていました。

この Rust 実装では、`hjkl` は矢印キーへ変換し、それ以外のキーではイベントに `Command` フラグを付与します。そのため `; + c` のようなショートカットは `Command + c` として動作します。

## 消費電力について

ほとんど消費しません。実装がポーリングではなく **イベント駆動** だからです。

- `CFRunLoopRun()` はイベントが届くまでスレッドをスリープさせます（アイドル時 CPU 使用率はほぼ 0%）。
- 購読しているのは `keyDown` / `keyUp` の 2 種類だけです。マウス移動などの高頻度イベントでは起こされません。
- 1 キーあたりの処理はフィールドの読み書き程度で、無視できるコストです。

Karabiner-Elements のような常駐リマッパーと同程度で、ノート PC で 24 時間動かしても電池への影響は実用上ほぼありません。

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
sh -n scripts/install.sh scripts/uninstall.sh
```

LaunchAgent の plist はバイナリ（`src/main.rs` の `macos::service`）が生成します。`cargo test` が生成物を `plutil` で検証するため、別ファイルとして持たず単一の情報源に保っています。

## License

MIT
