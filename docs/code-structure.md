# コード構造メモ

今の実装はまずゲーム成立を優先しているため、中心ロジックの大半が `src/game.rs` にあります。
このファイルは大きいですが、役割の塊はかなりはっきりしています。後から触るときは以下の順で追うと速いです。

## ファイルごとの役割

- [src/main.rs](/Users/densuke/Documents/2026/03/06/mcommand/src/main.rs)
  - `macroquad` のウィンドウ設定
  - メインループ
  - `Game::new -> update -> draw` の接続
- [src/game.rs](/Users/densuke/Documents/2026/03/06/mcommand/src/game.rs)
  - ゲーム状態
  - 入力処理
  - 敵出現制御
  - 当たり判定
  - 描画
  - 保存
  - 音生成
- [web/index.html](/Users/densuke/Documents/2026/03/06/mcommand/web/index.html)
  - HTML5 用の外側 UI
  - `localStorage` ブリッジ
  - `file://` 直開き時の案内
- [scripts/build-web.sh](/Users/densuke/Documents/2026/03/06/mcommand/scripts/build-web.sh)
  - Web 配信用成果物の生成
- [scripts/serve-web.sh](/Users/densuke/Documents/2026/03/06/mcommand/scripts/serve-web.sh)
  - `dist/web/` の簡易ローカル配信
- [.github/workflows/deploy-pages.yml](/Users/densuke/Documents/2026/03/06/mcommand/.github/workflows/deploy-pages.yml)
  - GitHub Pages 自動デプロイ

## `src/game.rs` の読み方

大きくは次の順で並んでいます。

1. 定数、列挙型、設定型
2. レイアウト、基地、都市、ミサイルなどのゲーム内データ
3. `AudioBank`
4. `Game` 本体
5. 描画ヘルパー
6. 保存処理
7. 音声波形生成

実際に手を入れるときの起点は以下です。

- 画面遷移や入力を変える
  - `Game::update`
  - `update_title`
  - `update_settings`
  - `update_playing`
  - `update_game_over`
- 敵の出現や難易度を変える
  - `Difficulty`
  - `begin_wave`
  - `spawn_enemy_missile`
  - `maybe_spawn_air_support`
  - `spawn_air_dropped_missile`
- 当たり判定やスコアを変える
  - `handle_explosion_hits`
  - `handle_airborne_hits`
  - `award_points`
  - `enemy_score`
- HUD や各画面を変える
  - `draw_title_screen`
  - `draw_settings_screen`
  - `draw_hud`
  - `draw_play_overlay`
  - `draw_game_over_screen`
- 保存先や保存形式を変える
  - `load_save_data`
  - `store_save_data`
  - `load_save_blob`
  - `store_save_blob`

## ネイティブ版と Web 版の差分

差分はなるべく少なくしています。

- `src/main.rs`
  - ネイティブではフルスクリーン起動
  - Web ではリサイズ可能
- `src/game.rs`
  - `Q` の扱いが異なる
  - 保存先が異なる
- `web/index.html`
  - `localStorage` ブリッジ
  - HTML 外枠

ゲームルール本体は可能な限り共通です。

## 次のリファクタリング候補

今の規模なら全面分割より、責務ごとに小さく切り出す方が安全です。

### 第一候補

- `save.rs`
  - `SaveData`
  - `load/store` 系
- `audio.rs`
  - `AudioBank`
  - 波形生成関数
- `ui.rs`
  - タイトル / 設定 / ゲームオーバー描画

### 第二候補

- `entities.rs`
  - `Base`, `City`, `EnemyMissile`, `Bomber`, `Satellite`, `Explosion`
- `spawning.rs`
  - 敵出現ロジック
- `render.rs`
  - 描画ヘルパー群

## 変更時の注意

- まず `cargo check` を通す
- Web 変更時は `./scripts/build-web.sh` まで確認する
- ブラウザ版は `file://` 直開きではなく HTTP 配信で確認する
- ネイティブ版と Web 版で `Q` の意味が違う点を崩さない
