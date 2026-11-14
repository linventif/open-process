//! Task lifecycle, cancellation and platform transport. No editor types here.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub type Compute = fn(String, Arc<AtomicBool>) -> Result<String, String>;

pub(crate) fn bounded_result(value: String) -> Result<String, String> {
    if value.len() > 8 * 1024 * 1024 {
        return Err(op_core::i18n::tr("analysis.result_limit").into());
    }
    Ok(value)
}

#[cfg(target_arch = "wasm32")]
static WEB_WORKERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(target_arch = "wasm32")]
struct WorkerPermit;
#[cfg(target_arch = "wasm32")]
impl Drop for WorkerPermit {
    fn drop(&mut self) {
        WEB_WORKERS.fetch_sub(1, Ordering::Relaxed);
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeTask {
    input: String,
    compute: Compute,
    stop: Arc<AtomicBool>,
    sender: std::sync::mpsc::Sender<Result<String, String>>,
    ctx: egui::Context,
}

#[cfg(not(target_arch = "wasm32"))]
fn native_queue() -> Result<&'static std::sync::mpsc::SyncSender<NativeTask>, String> {
    static QUEUE: std::sync::OnceLock<Result<std::sync::mpsc::SyncSender<NativeTask>, String>> = std::sync::OnceLock::new();
    QUEUE
        .get_or_init(|| {
            let (sender, receiver) = std::sync::mpsc::sync_channel::<NativeTask>(8);
            let receiver = Arc::new(std::sync::Mutex::new(receiver));
            for index in 0..2 {
                let receiver = receiver.clone();
                std::thread::Builder::new()
                    .name(format!("open-process-job-{index}"))
                    .spawn(move || {
                        loop {
                            let task = { receiver.lock().unwrap().recv() };
                            let Ok(task) = task else { break };
                            if task.stop.load(Ordering::Relaxed) {
                                continue;
                            }
                            let result = std::panic::catch_unwind(|| (task.compute)(task.input, task.stop.clone()))
                                .unwrap_or_else(|_| Err(op_core::i18n::tr("analysis.failed").into()))
                                .and_then(bounded_result);
                            if !task.stop.load(Ordering::Relaxed) {
                                let _ = task.sender.send(result);
                                task.ctx.request_repaint();
                            }
                        }
                    })
                    .map_err(|e| e.to_string())?;
            }
            Ok(sender)
        })
        .as_ref()
        .map_err(Clone::clone)
}

pub struct Job {
    cancelled: Arc<AtomicBool>,
    #[cfg(not(target_arch = "wasm32"))]
    receiver: std::sync::mpsc::Receiver<Result<String, String>>,
    #[cfg(target_arch = "wasm32")]
    worker: wasm_bindgen::JsValue,
    #[cfg(target_arch = "wasm32")]
    _permit: WorkerPermit,
    #[cfg(target_arch = "wasm32")]
    result: Arc<std::sync::Mutex<Option<Result<String, String>>>>,
    #[cfg(target_arch = "wasm32")]
    _callback: wasm_bindgen::closure::Closure<dyn FnMut(String)>,
}

impl Job {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn spawn(input: String, compute: Compute, ctx: &egui::Context) -> Result<Self, String> {
        if input.len() > 8 * 1024 * 1024 {
            return Err(op_core::i18n::tr("file.input_limit").into());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = std::sync::mpsc::channel();
        native_queue()?
            .try_send(NativeTask {
                input,
                compute,
                stop: cancelled.clone(),
                sender,
                ctx: ctx.clone(),
            })
            .map_err(|_| op_core::i18n::tr("analysis.jobs_busy").to_string())?;
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
        if input.len() > 8 * 1024 * 1024 {
            return Err(op_core::i18n::tr("file.input_limit").into());
        }
        use wasm_bindgen::JsCast;
        WEB_WORKERS
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| (count < 4).then_some(count + 1))
            .map_err(|_| op_core::i18n::tr("analysis.jobs_busy").to_string())?;
        let permit = WorkerPermit;
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
            _permit: permit,
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
    const finish = result => {
        clearTimeout(worker.openProcessDeadline);
        worker.onmessage = null; worker.onerror = null; worker.terminate();
        done(JSON.stringify(result));
    };
    worker.onmessage = e => finish(e.data);
    worker.onerror = e => finish({error:e.message || 'Analysis worker failed'});
    worker.openProcessDeadline = setTimeout(() => finish({error:'Background calculation timed out'}), 30000);
    worker.postMessage({glue:glue.href, wasm:wasm.href, input});
    return worker;
}
export function cancel_analysis_job(worker) {
    clearTimeout(worker.openProcessDeadline);
    worker.onmessage = null; worker.onerror = null; worker.terminate();
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn start_analysis_job(input: &str, done: &js_sys::Function) -> Result<JsValue, JsValue>;
    fn cancel_analysis_job(worker: &JsValue);
}
