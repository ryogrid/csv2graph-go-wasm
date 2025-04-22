// static/script.js (修正版)

document.addEventListener('DOMContentLoaded', () => {
    // === 修正箇所 ===
    // 最初にすべてのDOM要素を取得する
    const csvFileInput = document.getElementById('csvFile');
    const columnsInput = document.getElementById('columns');
    const titleInput = document.getElementById('title');
    const sizeInput = document.getElementById('size');
    const rangeInput = document.getElementById('range');
    const skipInput = document.getElementById('skip');
    const xdataCheckbox = document.getElementById('xdata');
    const xscaleInput = document.getElementById('xscale');
    const generateBtn = document.getElementById('generateBtn');
    const statusDiv = document.getElementById('status'); // <-- statusDivをここで取得
    const errorDiv = document.getElementById('error');
    const plotImage = document.getElementById('plotImage');
    const plotPlaceholder = document.getElementById('plotPlaceholder');
    const downloadLink = document.getElementById('downloadLink');
    // ===============

    let csvContent = null;
    const go = new Go();
    let wasmReady = false;

    // statusDivが取得された後なので、安全に使用できる
    statusDiv.textContent = "WASMモジュールを初期化中...";

    // --- Check WASM support and Initialize ---
    if (!WebAssembly.instantiateStreaming) {
        WebAssembly.instantiateStreaming = async (resp, importObject) => {
            const source = await (await resp).arrayBuffer();
            return await WebAssembly.instantiate(source, importObject);
        };
    }

    WebAssembly.instantiateStreaming(fetch("csv2graph.wasm"), go.importObject)
        .then((result) => {
            console.log("WASM Module Instantiated");
            statusDiv.textContent = "WASMインスタンス化完了。";

            wasmReady = true;
            console.log("WASM Ready Flag set to true.");

            statusDiv.textContent = "準備完了。CSVファイルを選択してください。";
            checkEnableButton();

            Promise.resolve(go.run(result.instance)).catch(err => {
                 console.error("Error during go.run():", err);
                 errorDiv.textContent = `エラー: Goランタイムの実行中にエラーが発生しました。 (${err})`;
                 wasmReady = false;
                 generateBtn.disabled = true;
                 statusDiv.textContent = "WASM実行エラー";
            });

            console.log("go.run() initiated.");

        })
        .catch((err) => {
            console.error("WASM Initialization Error during instantiation:", err);
            errorDiv.textContent = `エラー: WebAssemblyモジュールの初期化に失敗しました。 (${err})`;
            statusDiv.textContent = "";
            generateBtn.disabled = true;
            wasmReady = false;
        });

    // --- Event Listeners ---
    csvFileInput.addEventListener('change', handleFileSelect);
    generateBtn.addEventListener('click', handleGenerateClick);

    function checkEnableButton() {
        // この関数が呼ばれる時点で statusDiv と generateBtn は確実に存在する
        generateBtn.disabled = !(wasmReady && csvContent);
    }

    function handleFileSelect(event) {
        // この関数が呼ばれる時点で statusDiv, errorDiv などは確実に存在する
        const file = event.target.files[0];
        if (!file) {
            csvContent = null;
            checkEnableButton();
            return;
        }

        const reader = new FileReader();
        reader.onload = (e) => {
            csvContent = e.target.result;
            // statusDiv.textContent = `ファイル「${file.name}」を読み込みました。`; // メッセージは上書きしないようにコメントアウトのまま
            errorDiv.textContent = '';
            checkEnableButton();
        };
        reader.onerror = (e) => {
            console.error("File Reading Error:", e);
            errorDiv.textContent = `エラー: ファイル「${file.name}」の読み込みに失敗しました。`;
            csvContent = null;
            checkEnableButton();
        };
        reader.readAsText(file);
    }

    function handleGenerateClick() {
        // この関数が呼ばれる時点で必要な要素はすべて存在する
        if (!wasmReady || !csvContent) {
            errorDiv.textContent = 'WASMが準備できていないか、CSVファイルが選択されていません。';
            return;
        }
        if (typeof generatePlotGo !== 'function') {
             errorDiv.textContent = 'エラー: Go側の関数(generatePlotGo)がJavaScriptから利用可能になっていません。';
             return;
        }

        errorDiv.textContent = '';
        plotImage.style.display = 'none';
        plotPlaceholder.style.display = 'block';
        downloadLink.style.display = 'none';
        statusDiv.textContent = 'グラフを生成中...';
        generateBtn.disabled = true;

        const options = {
            columns: columnsInput.value.split(',').map(s => s.trim()).filter(s => s),
            title: titleInput.value || "Scatter Plot from CSV",
            size: sizeInput.value || "768x512",
            maxRange: parseFloat(rangeInput.value) || 0,
            skip: parseInt(skipInput.value) || 1,
            xdata: xdataCheckbox.checked,
            xscale: xscaleInput.value.trim() || "",
        };

        if (options.columns.length === 0) {
            errorDiv.textContent = 'エラー: プロットする列を指定してください。';
            statusDiv.textContent = '';
            generateBtn.disabled = false;
            return;
        }
        if (options.skip < 1) {
             errorDiv.textContent = 'エラー: データ間引きは1以上である必要があります。';
             statusDiv.textContent = '';
             generateBtn.disabled = false;
             return;
        }

        setTimeout(() => {
            try {
                console.log("Calling generatePlotGo with options:", options);
                const result = generatePlotGo(csvContent, JSON.stringify(options));
                console.log("Result from generatePlotGo:", result);

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
                    errorDiv.textContent = '予期しないエラー: WASMからの応答が不正です。';
                    console.log("Invalid result structure:", result);
                     statusDiv.textContent = '';
                }
            } catch (err) {
                 console.error("Error calling WASM function:", err);
                 errorDiv.textContent = `実行時エラー: ${err}`;
                 statusDiv.textContent = '';
            } finally {
                 generateBtn.disabled = false;
            }
        }, 10);

    } // handleGenerateClick end
}); // DOMContentLoaded end