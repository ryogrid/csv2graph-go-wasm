// static/script.js (Rust/WASM版)

// WASMモジュールを非同期でインポート
import init, { generate_plot_rust } from './csv2graph.js'; // wasm-pack が生成するファイル

document.addEventListener('DOMContentLoaded', () => {
    // DOM要素の取得 (変更なし)
    const csvFileInput = document.getElementById('csvFile');
    const columnsInput = document.getElementById('columns');
    const titleInput = document.getElementById('title');
    const sizeInput = document.getElementById('size');
    const rangeInput = document.getElementById('range');
    const skipInput = document.getElementById('skip');
    const xdataCheckbox = document.getElementById('xdata');
    const xscaleInput = document.getElementById('xscale');
    const generateBtn = document.getElementById('generateBtn');
    const statusDiv = document.getElementById('status');
    const errorDiv = document.getElementById('error');
    const plotImage = document.getElementById('plotImage');
    const plotPlaceholder = document.getElementById('plotPlaceholder');
    const downloadLink = document.getElementById('downloadLink');

    let csvContent = null;
    let wasmReady = false;

    statusDiv.textContent = "WASMモジュールを初期化中...";

    // --- WASM モジュールの初期化 ---
    async function initializeWasm() {
        try {
            await init(); // WASMモジュールとJSグルーコードを初期化
            console.log("WASM Module Initialized");
            statusDiv.textContent = "WASM初期化完了。";
            wasmReady = true;
            statusDiv.textContent = "準備完了。CSVファイルを選択してください。";
            checkEnableButton();
        } catch (err) {
            console.error("WASM Initialization Error:", err);
            errorDiv.textContent = `エラー: WebAssemblyモジュールの初期化に失敗しました。 (${err})`;
            statusDiv.textContent = "WASM初期化エラー";
            generateBtn.disabled = true;
            wasmReady = false;
        }
    }

    initializeWasm(); // 初期化を実行

    // --- イベントリスナー (変更なし) ---
    csvFileInput.addEventListener('change', handleFileSelect);
    generateBtn.addEventListener('click', handleGenerateClick);

    function checkEnableButton() {
        generateBtn.disabled = !(wasmReady && csvContent);
    }

    function handleFileSelect(event) {
        const file = event.target.files[0];
        if (!file) {
            csvContent = null;
            checkEnableButton();
            return;
        }

        const reader = new FileReader();
        reader.onload = (e) => {
            csvContent = e.target.result;
            errorDiv.textContent = ''; // エラーメッセージをクリア
            statusDiv.textContent = `ファイル「${file.name}」を読み込みました。`; // ステータス更新
            checkEnableButton();
        };
        reader.onerror = (e) => {
            console.error("File Reading Error:", e);
            errorDiv.textContent = `エラー: ファイル「${file.name}」の読み込みに失敗しました。`;
            csvContent = null;
            checkEnableButton();
        };
        reader.readAsText(file); // テキストとして読み込む
    }

    // --- グラフ生成ボタンのクリックハンドラ (Rust関数呼び出しに変更) ---
    function handleGenerateClick() {
        if (!wasmReady || !csvContent) {
            errorDiv.textContent = 'WASMが準備できていないか、CSVファイルが選択されていません。';
            return;
        }
        // generate_plot_rust関数が利用可能かチェック (init完了後に利用可能になる)
        if (typeof generate_plot_rust !== 'function') {
            errorDiv.textContent = 'エラー: Rust側の関数(generate_plot_rust)が利用可能になっていません。';
            return;
        }

        errorDiv.textContent = '';
        plotImage.style.display = 'none';
        plotPlaceholder.style.display = 'block';
        downloadLink.style.display = 'none';
        statusDiv.textContent = 'グラフを生成中...';
        generateBtn.disabled = true;

        // オプションを組み立てる (Go版と同様)
        const options = {
            columns: columnsInput.value.split(',').map(s => s.trim()).filter(s => s),
            title: titleInput.value || "Scatter Plot from CSV",
            size: sizeInput.value || "768x512",
            // parseFloatの結果がNaNならnull、そうでなければ数値。Rust側はOption<f64>で受ける
            maxRange: !isNaN(parseFloat(rangeInput.value)) ? parseFloat(rangeInput.value) : null,
             // parseIntの結果がNaNなら1、そうでなければ数値 (1以上)
            skip: Math.max(1, parseInt(skipInput.value) || 1),
            xdata: xdataCheckbox.checked,
            // 空文字列ならnull、そうでなければ文字列。Rust側はOption<String>で受ける
            xscale: xscaleInput.value.trim() || null,
        };

        // 基本的なバリデーション
        if (options.columns.length === 0) {
            errorDiv.textContent = 'エラー: プロットする列を指定してください。';
            statusDiv.textContent = '';
            generateBtn.disabled = false; // 再度押せるように
            return;
        }

        // Rust関数を非同期で呼び出す可能性があるため、setTimeoutは不要かも
        // 直接呼び出す
        try {
            console.log("Calling generate_plot_rust with options:", options);
            // Rust関数を呼び出し、結果 (JsValue) を受け取る
            const resultJsValue = generate_plot_rust(csvContent, JSON.stringify(options));

            // JsValue を JavaScriptオブジェクトに変換する必要はない (serde-wasm-bindgen が自動で行う)
            // ただし、返り値が Result<JsValue, JsValue> のため、エラーチェックが必要
            // generate_plot_rust自体がエラーを投げた場合 (JSONパース失敗など) はcatchブロックへ
            // Rust内部のエラーは resultJsValue.error に入る想定

            const result = resultJsValue; // 直接代入 (JsValueがProxyのように振る舞う)
            console.log("Result from generate_plot_rust:", result);


            if (result && result.error) {
                console.error("WASM Error:", result.error);
                errorDiv.textContent = `生成エラー: ${result.error}`;
                statusDiv.textContent = '';
            } else if (result && result.base64Image) {
                const imageUrl = `data:image/png;base64,${result.base64Image}`;
                plotImage.src = imageUrl;
                plotImage.style.display = 'block';
                plotPlaceholder.style.display = 'none';
                statusDiv.textContent = 'グラフが生成されました。';
                downloadLink.href = imageUrl;
                downloadLink.style.display = 'inline-block';
            } else {
                // 予期しない成功応答 (画像もエラーもない)
                errorDiv.textContent = '予期しないエラー: WASMからの応答が不正です (画像もエラーもありません)。';
                console.log("Invalid result structure:", result);
                statusDiv.textContent = '';
            }
        } catch (err) {
            // generate_plot_rust呼び出し自体、またはserde_wasm_bindgenでのエラー
            console.error("Error calling WASM function or processing result:", err);
            // エラーオブジェクトが詳細情報を持つ場合がある
            const errorMessage = err.message || String(err);
            errorDiv.textContent = `実行時エラー: ${errorMessage}`;
            statusDiv.textContent = '';
        } finally {
            // 処理完了後、ボタンを再度有効化
            generateBtn.disabled = false;
        }

    } // handleGenerateClick end
}); // DOMContentLoaded end