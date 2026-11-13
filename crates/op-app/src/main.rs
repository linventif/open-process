//! Open Process — BPMN & Petri net modeller (native + web).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(test)]
mod test_sources;

mod app;
mod bpmn_editor;
mod brand;
mod canvas;
mod composition;
mod database_editor;
mod dmn_editor;
mod dock;
mod editor;
mod flow_editor;
mod history;
mod home;
mod i18n;
mod icons;
mod io;
mod mining_editor;
mod petri_editor;
mod platform;
mod theme;
mod uml_editor;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init();
    let files: Vec<String> = std::env::args().skip(1).collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Open Process")
            .with_app_id("open-process")
            .with_icon(eframe::icon_data::from_png_bytes(brand::ICON_PNG).expect("embedded application icon"))
            .with_inner_size([1360.0, 840.0])
            .with_min_inner_size([720.0, 480.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Open Process",
        options,
        Box::new(move |cc| Ok(Box::new(app::OpenProcessApp::new(cc, files, composition::registry())))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    if web_sys::window().is_none() {
        return;
    }
    use eframe::wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("missing #the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#the_canvas_id is not a canvas");
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(app::OpenProcessApp::new(cc, Vec::new(), composition::registry())))),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match result {
                Ok(_) => loading.remove(),
                Err(e) => loading.set_inner_html(&format!("<p>Open Process failed to start: {e:?}</p>")),
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_background_job(input: &str) -> Result<String, wasm_bindgen::JsValue> {
    composition::run_background_job(input).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}
