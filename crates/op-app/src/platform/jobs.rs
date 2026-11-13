//! Task lifecycle, cancellation and platform transport. No editor types here.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub type Compute = fn(String, Arc<AtomicBool>) -> Result<String, String>;

pub struct Job {
    cancelled: Arc<AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: std::sync::mpsc::Receiver<Result<String, String>>,
    #[cfg(target_arch = "wasm32")]
    worker: wasm_bindgen::JsValue,
    #[cfg(target_arch = "wasm32")]
    result: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    #[cfg(target_arch = "wasm32")]
    _callback: wasm_bindgen::closure::Closure<dyn FnMut(String)>,
}

impl Job {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn(input: String, compute: Compute, ctx: &egui::Context) -> Result<Self, String> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let stop = cancelled.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("open-process-analysis".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(|| compute(input, stop.clone())).unwrap_or_else(|_| Err("Analysis task failed".into()));
                if !stop.load(Ordering::Relaxed) {
                    let _ = sender.send(result);
                    ctx.request_repaint();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { cancelled, receiver })
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll(&mut self) -> Option<Result<String, String>> {
        match self.receiver.try_recv() {
            Ok(value) => Some(value),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(_) => Some(Err("Analysis task stopped".into())),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn spawn(input: String, _compute: Compute, ctx: &egui::Context) -> Result<Self, String> {
        use wasm_bindgen::JsCast;
        let cancelled = Arc::new(AtomicBool::new(false));
        let result = Arc::new(std::sync::Mutex::new(None));
        let inbox = result.clone();
        let ctx = ctx.clone();
        let callback = wasm_bindgen::closure::Closure::<dyn FnMut(String)>::new(move |value: String| {
            let parsed = serde_json::from_str::<serde_json::Value>(&value)
                .map_err(|e| e.to_string())
                .and_then(|v| {
                    v.get("ok")
                        .and_then(|x| x.as_str())
                        .map(String::from)
                        .ok_or_else(|| v["error"].as_str().unwrap_or("Analysis worker failed").into())
                });
            *inbox.lock().unwrap() = Some(parsed);
            ctx.request_repaint();
        });
        let worker = start_analysis_job(&input, callback.as_ref().unchecked_ref()).map_err(|e| format!("{e:?}"))?;
        Ok(Self {
            cancelled,
            worker,
            result,
            _callback: callback,
        })
    }
    #[cfg(target_arch = "wasm32")]
    pub fn poll(&mut self) -> Option<Result<String, String>> {
        self.result.lock().unwrap().take()
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        #[cfg(target_arch = "wasm32")]
        cancel_analysis_job(&self.worker);
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(inline_js = r#"
export function start_analysis_job(input, done) {
    const glue = [...document.querySelectorAll('link[rel="modulepreload"]')].find(x => /open[-_]process.*\.js$/.test(x.href));
    const wasm = document.querySelector('link[rel="preload"][type="application/wasm"]');
    if (!glue || !wasm) throw new Error('Analysis worker requires Trunk module and Wasm preload URLs');
    const worker = new Worker(new URL('analysis-worker.js', document.baseURI), {type:'module'});
    worker.onmessage = e => { done(JSON.stringify(e.data)); worker.terminate(); };
    worker.onerror = e => { done(JSON.stringify({error:e.message || 'Analysis worker failed'})); worker.terminate(); };
    worker.postMessage({glue:glue.href, wasm:wasm.href, input});
    return worker;
}
export function cancel_analysis_job(worker) {
    worker.onmessage = null; worker.onerror = null; worker.terminate();
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn start_analysis_job(input: &str, done: &js_sys::Function) -> Result<JsValue, JsValue>;
    fn cancel_analysis_job(worker: &JsValue);
}
