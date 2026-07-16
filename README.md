# hjkl-for-mac

`karabiner-configs/` にある Karabiner-Elements 設定のうち、次の操作を Rust の常駐 CLI アプリとして実装したものです。

| 入力 | 出力 |
| --- | --- |
| `;` を単独で押す | `;` |
| `;` を押しながら `h` | 左矢印 |
| `;` を押しながら `j` | 下矢印 |
| `;` を押しながら `k` | 上矢印 |
| `;` を押しながら `l` | 右矢印 |

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

## Karabiner 設定との差分

既存の `karabiner-configs/semicolon_to_right_command.json` は `;` を他キーと同時押しした場合に `right_command` として扱い、その上で `right_command + hjkl` を矢印キーにしています。

この Rust 実装は、まず requested behavior である `; + hjkl` の矢印キーレイヤーに絞っています。`;+a` のような `hjkl` 以外の同時押しでは、遅延していた `;` は出力せず、相手のキーだけをそのまま通します。`right_command` としての完全互換が必要なら、その挙動も追加できます。
# hjkl-for-mac
# hjkl-for-mac
