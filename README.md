# csv2graph SPA (Rust/WASM Version)

CSV ファイルから **グリッド・折れ線・凡例付きの散布図** を生成する Web アプリケーションです。  
描画ロジックは Rust で実装され、 WebAssembly (WASM) と `wasm‑bindgen` でブラウザに組み込みます。
- [hosted site](https://ryogrid.github.io/csv2graph-go-wasm/)

![SPA Screenshot](./screenshot.png)

## 概要

1. ユーザーが CSV をアップロード  
2. JavaScript から WASM 関数 `generate_plot_rust()` を呼び出し  
3. Rust 側で CSV 解析 → 画像バッファ生成 → PNG へエンコード  
4. Base64 文字列を JS に返し `<img>` に描画 ― というシンプルな構成です。  

最新実装では **点を折れ線で結び、グリッドと数値目盛を描画し、凡例を右上に表示** するようになりました。

## 機能

* **CSV ➜ 散布図** – 複数系列を同時に描画  
* **点 + 折れ線** – 各系列の点を並び順に細線で接続  
* **グリッド** – X/Y 方向 5 分割の補助線と数値ラベル  
* **自動凡例** – 右上に系列名をカラー付きで一覧表示  
* **各種オプション**  
  * グラフタイトル  
  * 画像サイズ (例 `768x512`)  
  * X 最大値でのデータフィルタ  
  * N 行おきの間引き  
  * X 軸を CSV 1 列目にする / 行番号にする  
  * X 軸を任意範囲にスケーリング  
* **PNG ダウンロード** – 生成画像をそのまま保存可能

## 技術スタック

| レイヤ      | ライブラリ等 |
|-------------|-------------|
| コア言語    | Rust 2021 |
| ブラウザ実行| WebAssembly + `wasm‑bindgen` |
| CSV 解析    | `csv` |
| 描画        | `image` 0.25 / `imageproc` 0.25 / `ab_glyph`  |
| Base64      | `base64` |
| フロントエンド| HTML + CSS + Vanilla JS (ES Modules) |
| ビルド      | `wasm‑pack` |

## セットアップ & ビルド

```bash
# 1. Rust + wasm32 ターゲット
rustup target add wasm32-unknown-unknown

# 2. wasm‑pack
cargo install wasm-pack
```

```bash
# 3. ビルド (static/ 以下に出力)
cd rust
wasm-pack build --target web --out-dir ../static --out-name csv2graph
```

## 使い方

1. `static/` を任意の HTTP サーバーで公開  
   ```bash
   python3 -m http.server --directory static 8080
   ```  
2. ブラウザで `http://localhost:8080` を開く  
3. CSV を選択しオプションを設定 → **グラフ生成**  
4. 画像が表示され、クリックで PNG をダウンロード

## CSV 例

```csv
time,temperature,humidity
0,22.4,45
1,22.6,46
2,23.1,47
...
```

## ライセンス

This software is released into the public domain under the [Unlicense](https://unlicense.org/).
