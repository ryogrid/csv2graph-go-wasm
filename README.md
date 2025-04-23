# csv2graph SPA (Rust/WASM Version)

CSVファイルからインタラクティブに散布図を生成するWebアプリケーションです。Rust言語とWebAssembly (WASM) を利用して実装されています。

![SPA Screenshot](./screenshot.png)

## 概要

このWebアプリケーションは、ユーザーがアップロードしたCSVファイルの指定された列データを用いて散布図を描画します。グラフのタイトル、画像サイズ、データのフィルタリングや間引きなど、様々なオプションをWebインターフェース上で設定可能です。コアとなる描画処理はRust言語で実装され、WebAssemblyを介してブラウザ上で実行されます。

## 機能

*   **CSVファイルからの散布図描画:** ローカルのCSVファイルを選択してグラフを生成します。
*   **列指定:** グラフにプロットしたいデータ列名をカンマ区切りで指定できます。
*   **X軸データの自動/手動:** CSVファイルの最初の列をX軸データとして使用するか、行番号をX軸として自動生成するかを選択できます。
*   **カスタマイズ可能なオプション:**
    *   グラフタイトルの設定
    *   出力画像サイズの指定
    *   X軸の最大値によるデータフィルタリング
    *   データの間引き（N個ごとにプロット）
    *   X軸の値域を任意の値にマッピング
*   **インタラクティブな表示:** 生成されたグラフはWebページ上に直接表示されます。
*   **画像ダウンロード:** 生成されたグラフをPNGファイルとしてダウンロードできます。

## 技術スタック

*   **コアロジック:** Rust (2021 edition or later)
*   **ブラウザ実行:** WebAssembly (WASM)
*   **フロントエンド:** HTML, CSS, Vanilla JavaScript (ES Modules)
*   **WASM連携:** `wasm-bindgen`
*   **グラフ描画ライブラリ (Rust):** `plotters`, `plotters-bitmap`
*   **CSV解析ライブラリ (Rust):** `csv`
*   **画像処理ライブラリ (Rust):** `image`, `base64`
*   **ビルドツール:** `wasm-pack`

## セットアップ & ビルド

このアプリケーションをローカル環境で実行するには、以下の手順に従います。

1.  **Rustのインストール:** Rustツールチェイン (rustup) がインストールされていることを確認してください。
    ```bash
    # インストール (未導入の場合)
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    # WASMターゲットの追加
    rustup target add wasm32-unknown-unknown
    ```
2.  **wasm-packのインストール:** RustコードをWASMにコンパイルし、JavaScriptとの連携コードを生成するツールです。
    ```bash
    cargo install wasm-pack
    ```
3.  **リポジトリのクローン:**
    ```bash
    git clone <repository-url>
    cd csv2graph-spa-rust # ディレクトリ名が変わる可能性
    ```
4.  **Rust依存関係のビルド (任意):** `rust/` ディレクトリ内で依存関係をダウンロード・ビルドします（通常はwasm-packビルド時に自動で行われます）。
    ```bash
    cd rust
    cargo build --target wasm32-unknown-unknown # 動作確認用
    cd ..
    ```
5.  **WASMのビルド:** RustのコードをWebAssemblyモジュールとJavaScriptグルーコードにコンパイルします。
    ```bash
    cd rust
    wasm-pack build --target web --out-dir ../static --out-name csv2graph
    # または、リリースビルドの場合:
    # wasm-pack build --target web --out-dir ../static --out-name csv2graph --release
    cd ..
    ```
    これにより、`static/` ディレクトリに `csv2graph_bg.wasm` (WASM本体) と `csv2graph.js` (JavaScriptグルーコード) ファイルが生成されます。

## 使い方

1.  **ローカルWebサーバーの起動:** `static` ディレクトリをドキュメントルートとして、ローカルWebサーバーを起動します。
    ```bash
    # 例: Python 3 を使用する場合
    python3 -m http.server --directory static 8080

    # 例: Node.js の http-server を使用する場合 (npm install -g http-server)
    # http-server static -p 8080
    ```
2.  **ブラウザでアクセス:** Webブラウザを開き、`http://localhost:8080` (またはサーバーが使用するポート番号) にアクセスします。
3.  **CSVファイルの選択:** 「CSVファイル」セクションのファイル選択ボタンをクリックし、グラフ化したいCSVファイルを選びます。
4.  **オプションの設定:** (Go版と同様)
    *   **プロットする列:** グラフに表示したい列名をカンマ区切りで入力します (例: `temp,humidity`)。
    *   **グラフタイトル:** 表示したいグラフのタイトルを入力します。
    *   **画像サイズ:** 生成する画像のサイズを `幅x高さ` の形式で指定します (例: `800x600`)。
    *   **X軸最大値:** この値以下のX軸データのみをプロットする場合に指定します。
    *   **データ間引き:** データを間引いてプロットする場合、`N` を指定します (例: `2` なら1つおき)。
    *   **CSVの最初の列がX軸データ:** チェックを入れると、CSVの1列目をX軸の値として扱います。チェックしない場合は、行番号が自動的にX軸の値になります。
    *   **X軸の値域マッピング:** X軸の表示範囲を `開始値,終了値` の形式で指定します (例: `0,100`)。
5.  **グラフ生成:** 「グラフ生成」ボタンをクリックします。ボタンはWASMモジュールの準備が完了し、かつCSVファイルが選択されると有効になります。
6.  **結果の確認とダウンロード:** (Go版と同様)
    *   生成されたグラフが右側の「生成結果」エリアに表示されます。
    *   「グラフをダウンロード」リンクをクリックすると、表示されているグラフをPNGファイルとして保存できます。
    *   エラーが発生した場合は、設定エリア下部にメッセージが表示されます。

## CSVファイル形式の例

(Go版と同様)

## ライセンス

[Unlicense](http://unlicense.org/)