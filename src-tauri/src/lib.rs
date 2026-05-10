use std::path::{Path, PathBuf};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri::path::BaseDirectory;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "tiff", "tif", "heic", "heif", "bmp",
];

#[derive(Default)]
struct UpscalerState {
    current_child: Mutex<Option<CommandChild>>,
}

#[derive(Serialize, Clone)]
struct ProgressPayload {
    #[serde(rename = "itemId")]
    item_id: String,
    progress: f64,
}

fn is_image_file(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTS.iter().any(|x| x.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

/// 폴더는 안의 이미지 파일들로 펼치고, 파일은 이미지 확장자만 통과시킨다.
#[tauri::command]
fn expand_paths(paths: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for p in paths {
        let pb = PathBuf::from(&p);
        if pb.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&pb) {
                let mut images: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| is_image_file(p))
                    .collect();
                images.sort();
                for img in images {
                    if let Some(s) = img.to_str() {
                        out.push(s.to_string());
                    }
                }
            }
        } else if is_image_file(&pb) {
            out.push(p);
        }
    }
    out
}

fn build_output_path(
    input: &Path,
    output_dir: Option<&Path>,
    scale: u32,
    model_tag: &str,
) -> PathBuf {
    let dir: PathBuf = output_dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| input.parent().unwrap_or(Path::new(".")).to_path_buf());
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let candidate = dir.join(format!("{stem}_{scale}x_{model_tag}.png"));
    if !candidate.exists() {
        return candidate;
    }
    let mut i = 2;
    loop {
        let p = dir.join(format!("{stem}_{scale}x_{model_tag}_{i}.png"));
        if !p.exists() { return p; }
        i += 1;
    }
}

fn random_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

/// Tauri의 app.path().resolve()가 Windows에서 `\\?\` extended-length 접두어를
/// 붙여 반환할 수 있는데, ncnn-vulkan은 이 접두어를 처리하지 못해 _wfopen이 실패한다.
/// (실측: `\\?\C:\Users\...` → 0xC0000409 STACK_BUFFER_OVERRUN)
/// 접두어를 제거한 일반 절대 경로로 변환.
fn strip_unc_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped.to_string())
    } else {
        path.to_path_buf()
    }
}

/// ncnn-vulkan은 Windows에서 한글/유니코드 경로 처리에 버그가 있어
/// stack overrun을 일으킨다. 모든 OS에서 ASCII 보장된 임시 폴더에서 작업한다.
fn ascii_workdir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // C:\Users\Public\UpScale4K_temp — 사용자명이 한글이어도 ASCII 보장
        if let Ok(public) = std::env::var("PUBLIC") {
            let dir = PathBuf::from(public).join("UpScale4K_temp");
            if std::fs::create_dir_all(&dir).is_ok() {
                return dir;
            }
        }
        let fallback = PathBuf::from(r"C:\Users\Public\UpScale4K_temp");
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::temp_dir()
    }
}

fn resize_to_factor(input: &Path, output: &Path, factor: f64) -> Result<(), String> {
    use image::imageops::FilterType;
    let img = image::open(input).map_err(|e| format!("이미지 로드: {e}"))?;
    let new_w = ((img.width() as f64 * factor).round() as u32).max(1);
    let new_h = ((img.height() as f64 * factor).round() as u32).max(1);
    let resized = img.resize(new_w, new_h, FilterType::Lanczos3);
    resized.save(output).map_err(|e| format!("이미지 저장: {e}"))?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpscaleArgs {
    item_id: String,
    input_path: String,
    output_dir: Option<String>,
    model: String, // "realesrgan-x4plus" | "realesrgan-x4plus-anime"
    scale: u32,    // 2, 3, 4
}

#[tauri::command]
async fn upscale_image(
    app: AppHandle,
    state: State<'_, UpscalerState>,
    args: UpscaleArgs,
) -> Result<String, String> {
    let input = PathBuf::from(&args.input_path);
    let model_tag = if args.model.contains("anime") { "anime" } else { "general" };
    let output_dir_path = args.output_dir.as_deref().map(PathBuf::from);
    let final_output = build_output_path(
        &input,
        output_dir_path.as_deref(),
        args.scale,
        model_tag,
    );

    // ncnn은 항상 4×로 호출. 사용자가 2× / 3× 선택했으면 결과를 다운스케일.
    let needs_downscale = args.scale != 4;

    // ★ 한글/유니코드 경로에서 ncnn-vulkan이 stack overrun을 일으키는 버그를
    //   회피하기 위해, 입력·출력을 ASCII 보장된 임시 폴더로 복사·격리한다.
    let workdir = ascii_workdir();
    let id = random_id();
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let temp_input = workdir.join(format!("upscale4k_in_{id}.{ext}"));
    let temp_inference = workdir.join(format!("upscale4k_infer_{id}.png"));

    std::fs::copy(&input, &temp_input)
        .map_err(|e| format!("임시 입력 복사 실패: {e}"))?;

    let models_dir = app
        .path()
        .resolve("resources/models", BaseDirectory::Resource)
        .map_err(|e| format!("models 디렉토리 해석 실패: {e}"))?;
    let models_dir = strip_unc_prefix(&models_dir);
    let models_dir_str = models_dir
        .to_str()
        .ok_or_else(|| "models 경로가 UTF-8 아님".to_string())?;

    let sidecar = app
        .shell()
        .sidecar("realesrgan")
        .map_err(|e| format!("sidecar 찾지 못함: {e}"))?
        .args([
            "-i", temp_input.to_str().ok_or("temp input UTF-8 아님")?,
            "-o", temp_inference.to_str().ok_or("temp output UTF-8 아님")?,
            "-n", &args.model,
            "-s", "4",
            "-m", models_dir_str,
        ]);

    let (mut rx, child) = sidecar.spawn().map_err(|e| format!("ncnn 실행: {e}"))?;
    *state.current_child.lock().unwrap() = Some(child);

    let mut stderr_buf = String::new();
    let mut exit_code: Option<i32> = None;
    let mut signal_kill = false;

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stderr(line) => {
                let s = String::from_utf8_lossy(&line);
                stderr_buf.push_str(&s);
                for token in s.split_whitespace() {
                    if let Some(num) = token.strip_suffix('%') {
                        if let Ok(p) = num.parse::<f64>() {
                            let _ = app.emit("upscale-progress", ProgressPayload {
                                item_id: args.item_id.clone(),
                                progress: (p / 100.0).min(1.0),
                            });
                        }
                    }
                }
            }
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
                if payload.signal.is_some() && payload.code != Some(0) {
                    signal_kill = true;
                }
            }
            _ => {}
        }
    }

    *state.current_child.lock().unwrap() = None;

    if signal_kill {
        let _ = std::fs::remove_file(&temp_input);
        let _ = std::fs::remove_file(&temp_inference);
        return Err("CANCELLED".to_string());
    }

    if exit_code != Some(0) {
        let _ = std::fs::remove_file(&temp_input);
        let _ = std::fs::remove_file(&temp_inference);
        let snippet: String = stderr_buf.chars().take(1500).collect();
        return Err(format!("ncnn 실패 (exit {exit_code:?}): {snippet}"));
    }

    // 다운스케일 (필요시): temp_inference → temp_resized
    let temp_to_publish = if needs_downscale {
        let temp_resized = workdir.join(format!("upscale4k_resize_{id}.png"));
        if let Err(e) = resize_to_factor(&temp_inference, &temp_resized, args.scale as f64 / 4.0) {
            let _ = std::fs::remove_file(&temp_input);
            let _ = std::fs::remove_file(&temp_inference);
            return Err(e);
        }
        let _ = std::fs::remove_file(&temp_inference);
        temp_resized
    } else {
        temp_inference
    };

    // 최종 출력 폴더 보장
    if let Some(parent) = final_output.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            let _ = std::fs::remove_file(&temp_input);
            let _ = std::fs::remove_file(&temp_to_publish);
            return Err(format!("출력 폴더 생성: {e}"));
        }
    }

    // 임시 결과 → 사용자 지정/입력 옆 위치로 복사
    if let Err(e) = std::fs::copy(&temp_to_publish, &final_output) {
        let _ = std::fs::remove_file(&temp_input);
        let _ = std::fs::remove_file(&temp_to_publish);
        return Err(format!("최종 출력 복사: {e}"));
    }

    let _ = std::fs::remove_file(&temp_input);
    let _ = std::fs::remove_file(&temp_to_publish);

    Ok(final_output.to_string_lossy().into_owned())
}

#[tauri::command]
fn cancel_upscale(state: State<'_, UpscalerState>) -> Result<(), String> {
    let mut guard = state
        .current_child
        .lock()
        .map_err(|e| e.to_string())?;
    if let Some(child) = guard.take() {
        child.kill().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(UpscalerState::default())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            upscale_image,
            cancel_upscale,
            expand_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
