//! lib.rs – Rust/WASM PNG scatter-plot with line, grid, legend (imageproc + ab_glyph)

use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};
use serde_json;
use csv;

use image::{
    codecs::png::PngEncoder,
    ImageEncoder,
    ColorType,
};
use imageproc::image::{ImageBuffer, Rgba};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_line_segment_mut,
    draw_text_mut, text_size,
};
use ab_glyph::{FontArc, PxScale};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use std::io::Cursor;

// ───── panic → console ──────────────────────────────────────────
#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn set_panic_hook() { console_error_panic_hook::set_once(); }

// ───── JS ↔ Rust データ型 ───────────────────────────────────────
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlotOptions {
    columns:   Vec<String>,
    title:     String,
    size:      String,
    max_range: Option<f64>,
    skip:      usize,
    xdata:     bool,
    xscale:    Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlotResult {
    base64_image: Option<String>,
    error:        Option<String>,
}

#[wasm_bindgen]
pub fn generate_plot_rust(csv_content:&str,
                          options_json:&str)->Result<JsValue,JsValue>{
    let opt:PlotOptions = serde_json::from_str(options_json)
        .map_err(|e|JsValue::from_str(&format!("options parse: {e}")))?;

    let res = match scatter_png(csv_content,&opt){
        Ok(b64)=>PlotResult{base64_image:Some(b64),error:None},
        Err(e)=>PlotResult{base64_image:None,error:Some(e)},
    };
    serde_wasm_bindgen::to_value(&res)
        .map_err(|e|JsValue::from_str(&format!("serialize: {e}")))
}

// ───── コア処理 ────────────────────────────────────────────────
fn scatter_png(csv_text:&str,opt:&PlotOptions)->Result<String,String>{
    // 0) サイズ
    let (w,h)=parse_size(&opt.size)?;

    // 1) CSV
    let mut rdr=csv::ReaderBuilder::new().has_headers(true)
        .from_reader(csv_text.as_bytes());
    let headers=rdr.headers().map_err(|e|e.to_string())?.clone();

    let x_idx=if opt.xdata{Some(0)}else{None};
    let mut y_cols=Vec::<(usize,String)>::new();
    for name in &opt.columns{
        match headers.iter().position(|h|h==name){
            Some(i) if Some(i)!=x_idx => y_cols.push((i,name.clone())),
            None => return Err(format!("column '{name}' not found")),
            _=>{}
        }
    }
    if y_cols.is_empty(){return Err("no Y columns specified".into());}

    let mut series=vec![Vec::<(f64,f64)>::new(); y_cols.len()];
    let (mut min_x,mut max_x)=(f64::MAX,f64::MIN);
    let (mut min_y,mut max_y)=(f64::MAX,f64::MIN);
    for (row,rec_r) in rdr.records().enumerate(){
        let rec=rec_r.map_err(|e|format!("CSV row {row}: {e}"))?;
        if opt.skip>1 && row%opt.skip!=0 {continue;}

        let x_val=match x_idx{
            Some(i)=>parse_f64(rec.get(i),row,"X")?,
            None=>row as f64,
        };
        if opt.max_range.map_or(false,|m|x_val>m){continue;}

        for (si,(ci,_)) in y_cols.iter().enumerate(){
            let y_val=parse_f64(rec.get(*ci),row,"Y")?;
            series[si].push((x_val,y_val));
            min_y=min_y.min(y_val); max_y=max_y.max(y_val);
        }
        min_x=min_x.min(x_val); max_x=max_x.max(x_val);
    }
    if series.iter().all(|v|v.is_empty()){return Err("no data".into());}
    if let Some(s)=&opt.xscale{
        let(a,b)=parse_scale(s)?; min_x=a; max_x=b;
    }
    if min_x>=max_x {max_x=min_x+1.0;}
    if min_y>=max_y {max_y=min_y+1.0;}

    // 2) 描画バッファ
    let mut img:ImageBuffer<Rgba<u8>,Vec<u8>>=
        ImageBuffer::from_pixel(w,h,Rgba([255,255,255,255]));

    // フォント
    static FONT_BYTES:&[u8]=include_bytes!("../assets/NotoSansJP-Regular.ttf");
    let font=FontArc::try_from_slice(FONT_BYTES).map_err(|_|"font")?;
    let title_scale=PxScale::from(20.0);
    let label_scale=PxScale::from(13.0);

    // レイアウト
    let m=50.0;                                 // マージン
    let plot_w=(w as f64)-m*2.0;
    let plot_h=(h as f64)-m*2.0;
    let sx=plot_w/(max_x-min_x);
    let sy=plot_h/(max_y-min_y);
    let to_px=|x:f64| m+(x-min_x)*sx;
    let to_py=|y:f64| h as f64 - m - (y-min_y)*sy;

    let black=Rgba([0,0,0,255]);
    let gray =Rgba([220,220,220,255]);

    // 3) グリッド & ラベル ------------------------------------------------
    let grid_n=5;
    for i in 0..=grid_n{
        // X グリッド
        let vx=min_x+(max_x-min_x)*i as f64/grid_n as f64;
        let px=to_px(vx) as f32;
        draw_line_segment_mut(&mut img,(px,m as f32),(px,(h as f64-m) as f32),gray);
        let txt=format!("{:.2}",vx);
        let (tw,_)=text_size(label_scale,&font,&txt);
        draw_text_mut(&mut img,black,
            px as i32 - tw as i32/2, h as i32 - m as i32 + 4,
            label_scale,&font,&txt);

        // Y グリッド
        let vy=min_y+(max_y-min_y)*i as f64/grid_n as f64;
        let py=to_py(vy) as f32;
        draw_line_segment_mut(&mut img,(m as f32,py),((w as f64-m) as f32,py),gray);
        let txty=format!("{:.2}",vy);
        let (tyw,tyh)=text_size(label_scale,&font,&txty);
        draw_text_mut(&mut img,black,
            m as i32 - tyw as i32 - 4, py as i32 - tyh as i32/2,
            label_scale,&font,&txty);
    }

    // 軸（上書きでくっきり黒に）
    draw_line_segment_mut(&mut img,(m as f32,m as f32),
                                   (m as f32,(h as f64-m) as f32),black);
    draw_line_segment_mut(&mut img,(m as f32,(h as f64-m) as f32),
                                   ((w as f64-m) as f32,(h as f64-m) as f32),black);

    // タイトル
    let (tw,_)=text_size(title_scale,&font,&opt.title);
    draw_text_mut(&mut img,black,
        (w as i32 - tw as i32)/2, 10,
        title_scale,&font,&opt.title);

    // カラーパレット
    const PAL:&[Rgba<u8>]=&[
        Rgba([0x00,0x70,0xC0,255]), Rgba([0xC0,0x00,0x70,255]),
        Rgba([0x00,0xA0,0x40,255]), Rgba([0xFF,0x80,0x00,255]),
        Rgba([0x80,0x00,0xC0,255]), Rgba([0x00,0x90,0x90,255]),
    ];

    // 4) データ描画（線→点） --------------------------------------------
    for (si,pts) in series.iter().enumerate(){
        if pts.len()<2 {continue;}
        let col=PAL[si%PAL.len()];
        // 線
        for wdw in pts.windows(2){
            let (x1,y1)=wdw[0]; let (x2,y2)=wdw[1];
            draw_line_segment_mut(&mut img,
                (to_px(x1) as f32, to_py(y1) as f32),
                (to_px(x2) as f32, to_py(y2) as f32),
                col);
        }
        // 点
        for &(x,y) in pts{
            draw_filled_circle_mut(&mut img,
                (to_px(x) as i32, to_py(y) as i32), 4, col);
        }
    }

    // 5) 凡例 ------------------------------------------------------------
    let legend_x = (w as f64 - m + 10.0) as i32;
    let legend_y0 = m as i32;
    let line_h = 18;
    for (si,(_,name)) in y_cols.iter().enumerate(){
        let y = legend_y0 + (si as i32)*line_h;
        let col = PAL[si%PAL.len()];
        // シンボル
        draw_filled_circle_mut(&mut img,(legend_x, y+5),5,col);
        // テキスト
        draw_text_mut(&mut img, black,
            legend_x + 12, y,
            label_scale, &font, name);
    }

    // 6) PNG encode → base64 -------------------------------------------
    let mut buf=Vec::<u8>::new();
    {
        let mut cur=Cursor::new(&mut buf);
        PngEncoder::new(&mut cur).write_image(
            img.as_raw(),
            img.width(), img.height(),
            ColorType::Rgba8.into()
        ).map_err(|e|e.to_string())?;
    }
    Ok(B64.encode(buf))
}

// ───── ヘルパ ────────────────────────────────────────────
fn parse_size(s:&str)->Result<(u32,u32),String>{
    let(w,h)=s.split_once('x').ok_or("size format")?;
    let w:u32=w.trim().parse().map_err(|_|"width")?;
    let h:u32=h.trim().parse().map_err(|_|"height")?;
    if w==0||h==0 {Err("size>0".into())} else {Ok((w,h))}
}
fn parse_scale(s:&str)->Result<(f64,f64),String>{
    let(a,b)=s.split_once(',').ok_or("xscale format")?;
    let a:f64=a.trim().parse().map_err(|_|"start")?;
    let b:f64=b.trim().parse().map_err(|_|"end")?;
    if a>=b {Err("start<end".into())} else {Ok((a,b))}
}
fn parse_f64(v:Option<&str>,row:usize,label:&str)->Result<f64,String>{
    v.ok_or_else(||format!("{label} missing row {row}"))?
     .trim()
     .parse::<f64>()
     .map_err(|e|format!("{label} parse row {row}: {e}"))
}
