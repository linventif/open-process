//! Cross-platform file open/save.
//!
//! * Native: blocking system dialogs (rfd).
//! * Web: async file picker (rfd) and downloads through a Blob URL.

use op_core::i18n::{key, tr, trf};
use std::sync::{Arc, Mutex};

/// A file picked by the user: (file name, content).
pub type Loaded = (String, Vec<u8>);

const FILTER_NAME: &str = key("file.supported_files");

#[derive(Default, Clone)]
pub struct FileIo {
    inbox: Arc<Mutex<Vec<Loaded>>>,
}

impl FileIo {
    /// Files that finished loading since the last call.
    pub fn poll(&self) -> Vec<Loaded> {
        std::mem::take(&mut *self.inbox.lock().unwrap())
    }

    pub fn push(&self, f: Loaded) {
        self.inbox.lock().unwrap().push(f);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_dialog(&self, _ctx: &egui::Context, extensions: Vec<&'static str>) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(tr(FILTER_NAME), &extensions)
            .add_filter(tr("file.all_files"), &["*"])
            .pick_file()
        {
            match std::fs::read(&path) {
                Ok(bytes) => self.push((path.display().to_string(), bytes)),
                Err(e) => log::error!("Cannot read {}: {e}", path.display()),
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open_dialog(&self, ctx: &egui::Context, extensions: Vec<&'static str>) {
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter(tr(FILTER_NAME), &extensions)
                .pick_file()
                .await
            {
                let data = file.read().await;
                inbox.lock().unwrap().push((file.file_name(), data));
                ctx.request_repaint();
            }
        });
    }
}

/// Asks where to save (native) or triggers a download (web).
/// Returns the chosen path / name, or `None` if cancelled.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_file(suggested_name: &str, data: &[u8]) -> Result<Option<String>, String> {
    let ext = suggested_name.rsplit('.').next().unwrap_or("");
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(suggested_name)
        .add_filter(ext, &[ext])
        .save_file()
    else {
        return Ok(None);
    };
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

/// Writes directly to a known path (native "Save" after a first "Save as").
#[cfg(not(target_arch = "wasm32"))]
pub fn write_path(path: &str, data: &[u8]) -> Result<(), String> {
    std::fs::write(path, data).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn save_file(suggested_name: &str, data: &[u8]) -> Result<Option<String>, String> {
    use eframe::wasm_bindgen::JsCast;
    let err = |e: eframe::wasm_bindgen::JsValue| format!("{e:?}");
    let array = js_sys::Uint8Array::from(data);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &opts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(err)?.dyn_into().map_err(|_| "anchor")?;
    a.set_href(&url);
    a.set_download(suggested_name);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(Some(suggested_name.to_string()))
}

#[cfg(target_arch = "wasm32")]
pub fn write_path(_path: &str, _data: &[u8]) -> Result<(), String> {
    Err("not supported on the web".into())
}

/// Base name of a path (works for both `/` and `\`).
pub fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

// ---------------------------------------------------------------------------
// PNG export of a screen region (matrices, diagrams) through an egui screenshot.

use std::sync::Mutex as StdMutex;

type Shot = (std::sync::Arc<egui::ColorImage>, egui::Rect, String);
static READY: StdMutex<Option<Shot>> = StdMutex::new(None);

/// Asks for a screenshot of the current frame; the region `rect` (in points)
/// will be saved as `file_name` by [`poll_png_export`].
pub fn request_png_export(ctx: &egui::Context, rect: egui::Rect, file_name: String) {
    let ctx2 = ctx.clone();
    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |img| {
        *READY.lock().unwrap() = Some((img, rect, file_name));
        ctx2.request_repaint();
    })));
}

/// Encodes a region of a screenshot as PNG.
pub fn encode_png(img: &egui::ColorImage, rect: egui::Rect, pixels_per_point: f32) -> Result<Vec<u8>, String> {
    let region = img.region(&rect, Some(pixels_per_point));
    let [w, h] = region.size;
    let raw: Vec<u8> = region.pixels.iter().flat_map(|c| c.to_array()).collect();
    let buf = image::RgbaImage::from_raw(w as u32, h as u32, raw).ok_or("invalid image")?;
    let mut out = std::io::Cursor::new(Vec::new());
    buf.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

/// Saves a finished screenshot request, if any. Returns a status message.
pub fn poll_png_export(ctx: &egui::Context) -> Option<Result<String, String>> {
    let (img, rect, name) = READY.lock().unwrap().take()?;
    let bytes = match encode_png(&img, rect, ctx.pixels_per_point()) {
        Ok(b) => b,
        Err(e) => return Some(Err(e)),
    };
    Some(match save_file(&name, &bytes) {
        Ok(Some(p)) => Ok(trf("app.exported", &[&(file_name(&p))])),
        Ok(None) => Ok(String::new()),
        Err(e) => Err(e),
    })
}
