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

## ビルド

```sh
cargo build --release
```

## 実行

```sh
cargo run --release
```

または:

```sh
target/release/hjkl-for-mac
```

実行中だけキーリマップが有効です。停止するにはターミナルで `Ctrl-C` を押してください。

## macOS 権限

初回実行時、macOS の権限がないとイベントタップを作成できません。その場合は次を許可してから、ターミナルを再起動して再実行してください。

1. System Settings
2. Privacy & Security
3. Accessibility
4. 必要に応じて Input Monitoring
5. 実行している Terminal/iTerm、またはビルドした `hjkl-for-mac` バイナリを許可

## 元の Karabiner 設定との対応

元の Karabiner-Elements 設定では、`;` を他キーと同時押しした場合に `right_command` として扱い、その上で `right_command + hjkl` を矢印キーにしていました。

この Rust 実装では、`hjkl` は矢印キーへ変換し、それ以外のキーではイベントに `Command` フラグを付与します。そのため `; + c` のようなショートカットは `Command + c` として動作します。

## 消費電力について

ほとんど消費しません。理由は実装がポーリングではなく **イベント駆動** だからです。

- `CFRunLoopRun()` はイベントが届くまでスレッドをスリープさせます。入力していない間は CPU をほぼ使いません（アイドル時 CPU 使用率はほぼ 0%）。
- 購読しているのは `keyDown` / `keyUp` の 2 種類だけです（`mask = keyDown | keyUp`）。マウス移動などの高頻度イベントでは起こされません。
- 1 キーあたりの処理はフィールドの読み書き程度で、無視できるコストです。

Karabiner-Elements のような常駐リマッパーと同程度で、ノート PC で 24 時間動かしても電池への影響は実用上ほぼありません。

## デーモン（LaunchAgent）として常駐させる

macOS でログインセッション中に常駐させるには **LaunchDaemon ではなく LaunchAgent** を使います。イベントタップはユーザーの GUI ログインセッションとアクセシビリティ権限を必要とするため、root で動く LaunchDaemon では正しく動作しません。

`launchd/com.kazuki-hanai.hjkl-for-mac.plist` を用意しています。バイナリの絶対パスは環境に合わせて書き換えてください。

### インストール

```sh
# 先にリリースビルドしておく
cargo build --release

# plist を配置
cp launchd/com.kazuki-hanai.hjkl-for-mac.plist ~/Library/LaunchAgents/

# 読み込み（RunAtLoad=true なのでそのまま起動します）
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.kazuki-hanai.hjkl-for-mac.plist
```

### 状態確認・再起動・停止

```sh
# 状態
launchctl print gui/$(id -u)/com.kazuki-hanai.hjkl-for-mac

# 手動で再起動
launchctl kickstart -k gui/$(id -u)/com.kazuki-hanai.hjkl-for-mac

# 停止・アンロード
launchctl bootout gui/$(id -u)/com.kazuki-hanai.hjkl-for-mac
```

### 注意点

- launchd 経由で起動する場合、アクセシビリティ権限は **ターミナルではなくバイナリ本体** (`target/release/hjkl-for-mac`) に付与する必要があります。初回は System Settings → Privacy & Security → Accessibility で許可してください。
- `ProcessType` を `Interactive` にしているのは、macOS のスロットリングを避けるためです。処理が遅延するとイベントタップがタイムアウトで一時無効化されることがあります（無効化された場合は自動で再有効化する実装にしています）。
- `KeepAlive=true` なのでクラッシュ時や終了時に自動で再起動します。
- ビルド成果物のパスを変えた場合は plist の `ProgramArguments` を合わせて更新してください。
