# mcommand

Rust と `macroquad` で実装した `Missile Command` 風ゲームです。

現在は以下の 2 形態で動作します。

- ローカルアプリとしてのネイティブ実行
- WebAssembly + HTML5 によるブラウザ実行

公開中の Web 版:

- https://densuke.github.io/mcommand/

## 現在の主な機能

- フルスクリーン対応
- 照準移動
  - 矢印キー
  - マウス
- 発射
  - `Z`: 左基地
  - `X`: 中央基地
  - `C`: 右基地
- 一時停止
  - `Space` または `P`
- タイトル画面
- 設定画面
  - 基地ミサイル数
  - 爆風半径
  - 難易度
- 敵種
  - 通常弾
  - 分裂弾
  - スマートボム
  - 爆撃機
  - 衛星

## 必要環境

### ネイティブ実行

- Rust
- Cargo

macOS / Linux / Windows での移植性は意識していますが、現時点では macOS を中心に確認しています。

### WebAssembly ビルド

- Rust
- Cargo
- `wasm32-unknown-unknown` ターゲット

追加していない場合は以下を実行してください。

```bash
rustup target add wasm32-unknown-unknown
```

## ローカルアプリとして実行する

開発用にそのまま起動する場合:

```bash
cargo run
```

リリースビルドを作る場合:

```bash
cargo build --release
```

生成物の例:

```text
target/release/mcommand
```

## WebAssembly 版をビルドする

静的配信用のファイル一式を作るには以下を実行します。

```bash
./scripts/build-web.sh
```

生成先:

```text
dist/web/
```

生成される主なファイル:

- `dist/web/index.html`
- `dist/web/mq_js_bundle.js`
- `dist/web/mcommand.wasm`

## WebAssembly 版をローカルで確認する

`file://` で `index.html` を直接開いても動きません。
必ず HTTP サーバー経由で開いてください。

ローカル確認用サーバー:

```bash
./scripts/serve-web.sh 8000
```

ブラウザで以下を開きます。

```text
http://127.0.0.1:8000
```

## Web サーバーへ配置する

静的ホスティングでは `dist/web/` の中身をそのまま配置してください。

配置対象:

- `index.html`
- `mq_js_bundle.js`
- `mcommand.wasm`

GitHub Pages、Netlify、Cloudflare Pages、任意の静的 Web サーバーなどで配信できます。

注意:

- `mcommand.wasm` が HTTP 経由で配信されること
- `index.html` と `mcommand.wasm` と `mq_js_bundle.js` が同じ公開ディレクトリにあること
- ブラウザの自動再生制限により、音は初回入力後に有効になる場合があること

## GitHub Pages で自動デプロイする

GitHub Pages 用のワークフローを同梱しています。

- ワークフロー: [.github/workflows/deploy-pages.yml](/Users/densuke/Documents/2026/03/06/mcommand/.github/workflows/deploy-pages.yml)

このワークフローは以下を行います。

- `main` ブランチへの push を契機に実行
- `wasm32-unknown-unknown` ターゲットで Web 版をビルド
- `dist/web/` を GitHub Pages へデプロイ

利用前に GitHub 側で以下を確認してください。

- リポジトリの `Settings > Pages` を開く
- `Build and deployment` の `Source` を `GitHub Actions` に設定する

現在のワークフローは `actions/configure-pages@v5` の `enablement: true` を使っているため、
初回実行時に Pages が未有効でも、そのまま有効化される前提です。
ただし、GitHub 側の権限やリポジトリ設定によっては手動確認が必要です。

設定後は `main` へ push するだけで Pages へ公開されます。

手動実行したい場合は GitHub の `Actions` タブから `Deploy Pages` を `Run workflow` してください。

## 操作方法

### 共通

- 照準移動: `矢印キー` または `マウス`
- 発射: `Z` `X` `C`
- 一時停止: `Space` または `P`
- フルスクリーン: `F`

### ネイティブ版

- 終了: `Q`

### Web 版

- `Q` は終了ではなく戻る操作になります
  - 設定画面では前の画面へ戻る
  - プレイ中やゲームオーバーではタイトルへ戻る

## 保存データ

### ネイティブ版

以下のファイルに簡易保存します。

```text
~/.mcommand-save
```

保存内容:

- 設定
- ハイスコア

### Web 版

ブラウザの `localStorage` に保存します。

保存内容:

- 設定
- ハイスコア

## よく使うコマンド

ネイティブ実行:

```bash
cargo run
```

ネイティブ確認:

```bash
cargo check
```

Web ビルド:

```bash
./scripts/build-web.sh
```

Web ローカル配信:

```bash
./scripts/serve-web.sh 8000
```

## 備考

- 現在の Web 版は静的配信前提です
- オンラインランキング機能はまだ実装していません
- 今後は Web API を追加することで、ネイティブ版 / Web 版の両方から同じランキングへ参加できる構成を想定しています
- 開発者向けのコードマップは [docs/code-structure.md](/Users/densuke/Documents/2026/03/06/mcommand/docs/code-structure.md) にまとめています
