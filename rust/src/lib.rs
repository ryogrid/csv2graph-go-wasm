// rust/src/lib.rs (完全版: use 文再修正)

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use plotters::prelude::*;
use plotters_bitmap::BitMapBackend;
use image::{ImageFormat, ImageOutputFormat, load_from_memory_with_format};
use std::io::Cursor;
use base64::{Engine as _, engine::general_purpose::STANDARD as base64_standard};
use once_cell::sync::Lazy;
// --- 修正: register_font の use パスを style 直下にする ---
use plotters::style::font::register_font;
// use plotters::style::font::register_font; // ← もし残っていたら削除
// --- 修正 ここまで ---
use std::sync::atomic::{AtomicBool, Ordering};
use plotters::series::PointSeries;


// --- カスタムフォント名定義 ---
const CUSTOM_FONT_NAME: &str = "NotoSansJP";

// --- フォントデータの静的読み込み ---
static FONT_DATA: Lazy<Vec<u8>> = Lazy::new(|| {
    include_bytes!("../assets/NotoSansJP-Regular.ttf").to_vec()
});

// --- フォント登録フラグ ---
static FONT_REGISTERED: AtomicBool = AtomicBool::new(false);

// --- フォント登録関数 ---
fn ensure_font_registered() -> Result<(), String> {
    match FONT_REGISTERED.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(false) => {
            // register_font は use plotters::style::register_font; で解決されるはず
            register_font(CUSTOM_FONT_NAME, &FONT_DATA)
                .map_err(|e| format!("Failed to register custom font '{}': {}", CUSTOM_FONT_NAME, e))?;
            Ok(())
        }
        Ok(true) => Ok(()),
        Err(_) => Err("Failed to check font registration status due to atomic operation failure.".to_string())
    }
}


// --- JavaScript から受け取るオプション ---
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PlotOptions {
    columns: Vec<String>,
    title: String,
    size: String,
    max_range: Option<f64>,
    skip: usize,
    xdata: bool,
    xscale: Option<String>,
}

// --- JavaScript に返す結果 ---
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PlotResult {
    base64_image: Option<String>,
    error: Option<String>,
}

// --- ユーティリティ: サイズ文字列をパース ---
fn parse_size(size_str: &str) -> Result<(u32, u32), String> {
    let parts: Vec<&str> = size_str.split('x').collect();
    if parts.len() != 2 { return Err(format!("Invalid size format: '{}'. Expected 'WIDTHxHEIGHT'.", size_str)); }
    let width = parts[0].parse::<u32>().map_err(|e| format!("Invalid width: {}", e))?;
    let height = parts[1].parse::<u32>().map_err(|e| format!("Invalid height: {}", e))?;
    Ok((width, height))
}

// --- ユーティリティ: スケール文字列をパース ---
fn parse_scale(scale_str: &Option<String>) -> Result<Option<(f64, f64)>, String> {
    match scale_str {
        Some(s) if !s.is_empty() => {
            let parts: Vec<&str> = s.split(',').collect();
            if parts.len() != 2 { return Err(format!("Invalid xscale format: '{}'. Expected 'START,END'.", s)); }
            let start = parts[0].trim().parse::<f64>().map_err(|e| format!("Invalid xscale start value: {}", e))?;
            let end = parts[1].trim().parse::<f64>().map_err(|e| format!("Invalid xscale end value: {}", e))?;
            if start >= end { return Err("Invalid xscale range: start value must be less than end value.".to_string()); }
            Ok(Some((start, end)))
        }
        _ => Ok(None),
    }
}


// --- JavaScript から呼び出されるメイン関数 ---
#[wasm_bindgen]
pub fn generate_plot_rust(csv_content: &str, options_json: &str) -> Result<JsValue, JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    ensure_font_registered().map_err(|e| JsValue::from_str(&e))?;

    let options: PlotOptions = serde_json::from_str(options_json).map_err(|e| JsValue::from_str(&format!("Failed to parse options JSON: {}", e)))?;
    let (width, height) = parse_size(&options.size).map_err(JsValue::from)?;
    let result = process_csv_and_plot(csv_content, options, width, height).unwrap_or_else(|err_msg| PlotResult { base64_image: None, error: Some(err_msg) });
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}


// --- CSV処理とグラフ描画のコアロジック ---
fn process_csv_and_plot(csv_content: &str, options: PlotOptions, width: u32, height: u32) -> Result<PlotResult, String> {

    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_reader(csv_content.as_bytes());
    let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
    let header_vec: Vec<&str> = headers.iter().collect();

    let mut plot_indices: Vec<(usize, String)> = Vec::new();
    let x_col_index: Option<usize> = if options.xdata { if header_vec.is_empty() { return Err("CSV no header with xdata".to_string()); } Some(0) } else { None };
    for col_name in &options.columns { match headers.iter().position(|h| h == col_name) { Some(index) => { if Some(index) == x_col_index && options.xdata { continue; } plot_indices.push((index, col_name.clone())); }, None => return Err(format!("Column '{}' not found", col_name)), } }
    if plot_indices.is_empty() && !options.xdata { if headers.len() > 0 { plot_indices.push((0, headers.get(0).unwrap_or("Col1").to_string())); } else { return Err("No columns to plot".to_string()); } } else if plot_indices.is_empty() && options.xdata { return Err("No Y columns specified".to_string()); }

    let mut data_series: Vec<Vec<(f64, f64)>> = vec![Vec::new(); plot_indices.len()];
    let mut record_count: usize = 0; let mut skipped_count: usize = 0;
    let mut min_x = f64::MAX; let mut max_x = f64::MIN; let mut min_y = f64::MAX; let mut max_y = f64::MIN;
    for (row_index, result) in rdr.records().enumerate() { let record = result.map_err(|e| format!("CSV read error r{}: {}", row_index + 1, e))?; record_count += 1; if options.skip > 1 && (skipped_count % options.skip != 0) { skipped_count += 1; continue; } skipped_count += 1; let x_val: f64 = match x_col_index { Some(idx) => record.get(idx).ok_or_else(|| format!("No X data r{}", row_index + 1))?.trim().parse::<f64>().map_err(|e| format!("X parse error r{}: {}", row_index + 1, e))?, None => skipped_count as f64, }; if let Some(max_r) = options.max_range { if x_val > max_r { continue; } } min_x = min_x.min(x_val); max_x = max_x.max(x_val); for (series_idx, (col_idx, _)) in plot_indices.iter().enumerate() { let y_val = record.get(*col_idx).ok_or_else(|| format!("No Y data c{} r{}", col_idx, row_index + 1))?.trim().parse::<f64>().map_err(|e| format!("Y parse error c{} r{}: {}", plot_indices[series_idx].1, row_index + 1, e))?; data_series[series_idx].push((x_val, y_val)); min_y = min_y.min(y_val); max_y = max_y.max(y_val); } }
    if record_count == 0 { return Err("No data records".to_string()); } if data_series.iter().all(|s| s.is_empty()) { return Err("No data to plot after filter/skip".to_string()); }

    if min_x.is_infinite() || min_x.is_nan() || max_x.is_infinite() || max_x.is_nan() || min_x >= max_x { min_x = 0.0; max_x = 1.0; } if min_y.is_infinite() || min_y.is_nan() || max_y.is_infinite() || max_y.is_nan() || min_y >= max_y { min_y = 0.0; max_y = 1.0; }
    let x_margin = (max_x - min_x) * 0.05; let y_margin = (max_y - min_y) * 0.05;
    let final_min_x = min_x - x_margin - f64::EPSILON; let final_max_x = max_x + x_margin + f64::EPSILON;
    let final_min_y = min_y - y_margin - f64::EPSILON; let final_max_y = max_y + y_margin + f64::EPSILON;
    let x_spec = match parse_scale(&options.xscale)? { Some((s, e)) => s..e, None => final_min_x..final_max_x };
    let y_spec = final_min_y..final_max_y;

    if width == 0 || height == 0 { return Err("Zero image dimensions".to_string()); }
    let expected_buffer_size = (width as usize).checked_mul(height as usize).and_then(|a| a.checked_mul(3)).ok_or_else(|| format!("Buffer size limit {}x{}", width, height))?;
    let mut buffer = vec![0u8; expected_buffer_size];

    {
        let root = BitMapBackend::with_buffer(&mut buffer, (width, height)).into_drawing_area();
        root.fill(&WHITE).map_err(|e| format!("Fill bg error: {}", e))?;
        let caption_font_size = (width as f64 * 0.04).max(12.0);
        let mut chart = ChartBuilder::on(&root)
            .caption(&options.title, (CUSTOM_FONT_NAME, caption_font_size).into_font())
            .margin(10).x_label_area_size((height as f64 * 0.1).max(30.0) as u32).y_label_area_size((width as f64 * 0.1).max(40.0) as u32)
            .build_cartesian_2d(x_spec.clone(), y_spec.clone()).map_err(|e| format!("Build chart error: {}", e))?;
        let x_axis_desc = if options.xdata { headers.get(0).unwrap_or(&"X Axis").to_string() } else { "Row Number".to_string() };
        let axis_desc_font_size = (width as f64 * 0.025).max(10.0);
        chart.configure_mesh()
            .x_desc(x_axis_desc).y_desc("Values")
            .axis_desc_style((CUSTOM_FONT_NAME, axis_desc_font_size).into_font())
            .label_style((CUSTOM_FONT_NAME, axis_desc_font_size * 0.8).into_font())
            .draw().map_err(|e| format!("Draw mesh error: {}", e))?;

        let colors = [RED, GREEN, BLUE, CYAN, MAGENTA, YELLOW, BLACK];
        for (i, series) in data_series.iter().enumerate() {
             if series.is_empty() { continue; }
             let color = colors[i % colors.len()].clone();
             let col_name = &plot_indices[i].1;
             chart.draw_series(
                 PointSeries::of_element(
                     series.iter().map(|(x, y)| (*x, *y)),
                     3, &color, &|c, s, st| { EmptyElement::at(c) + Circle::new((0, 0), s, st) },
                 )
             ).map_err(|e| format!("Draw series '{}' error: {}", col_name, e))?
              .label(col_name.clone())
              .legend(move |(x, y)| Circle::new((x, y), 3, color));
        }
        chart.configure_series_labels().border_style(BLACK).background_style(WHITE.mix(0.8)).position(SeriesLabelPosition::UpperRight)
             .draw().map_err(|e| format!("Draw legend error: {}", e))?;
        root.present().map_err(|e| format!("Present error: {}", e))?;
    }

    let mut png_buffer = Cursor::new(Vec::new());
    let dynamic_image = load_from_memory_with_format(&buffer, ImageFormat::Bmp).map_err(|e| format!("Load BMP error: {}", e))?;
    dynamic_image.write_to(&mut png_buffer, ImageOutputFormat::Png).map_err(|e| format!("Encode PNG error: {}", e))?;
    let base64_image = base64_standard.encode(png_buffer.get_ref());

    Ok(PlotResult { base64_image: Some(base64_image), error: None })
}