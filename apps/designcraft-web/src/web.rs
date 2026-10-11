//! The browser shell: web `Services`, drag-and-drop, and the eframe web runner.

use designcraft_engine::Session;
use designcraft_ui_egui::{DesignApp, Inbox, Services};
#[cfg(not(feature = "simplehub"))]
use wasm_bindgen::JsCast as _;

#[cfg(not(feature = "simplehub"))]
const DOC_EXTS: &[&str] = &["designcraft", "idml"];
#[cfg(not(feature = "simplehub"))]
const IMAGE_EXTS: &[&str] = &[
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "tif",
    "tiff",
    "bmp",
    "psd",
    "svg",
    "pdf",
    "ai",
    "txt",
    "docx",
    "rtf",
    "md",
    "xlsx",
    "idml",
    "designcraft",
];
#[cfg(not(feature = "simplehub"))]
const CANVAS_ID: &str = "designcraft_canvas";
#[cfg(not(feature = "simplehub"))]
const LOADING_ID: &str = "designcraft_loading";

#[cfg(not(feature = "simplehub"))]
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            log::error!("no document");
            return;
        };
        let Some(canvas) = document.get_element_by_id(CANVAS_ID).and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok()) else {
            log::error!("missing <canvas id=\"{CANVAS_ID}\">");
            return;
        };
        let mut options = eframe::WebOptions::default();
        if query().contains("webgl")
            && let eframe::egui_wgpu::WgpuSetup::CreateNew(create) = &mut options.wgpu_options.wgpu_setup
        {
            create.instance_descriptor.backends = eframe::wgpu::Backends::GL;
        }
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(move |cc| {
                    if let Some(rs) = &cc.wgpu_render_state {
                        log::info!("designcraft-web: wgpu backend {:?}", rs.adapter.get_info().backend);
                    }
                    let inbox: Inbox = Inbox::default();
                    let mut app = DesignApp::new(Session::new(), services(inbox.clone(), cc.egui_ctx.clone()));
                    if query().contains("sample") {
                        let _ = app.run("file.newSample", serde_json::json!({}));
                    }
                    Ok(Box::new(WebShell { app, inbox }))
                }),
            )
            .await;
        if let Some(el) = document.get_element_by_id(LOADING_ID) {
            match result {
                Ok(()) => el.remove(),
                Err(e) => el.set_inner_html(&format!("<p>{} failed to start: {e:?}</p><p>A browser with WebGPU or WebGL2 is required.</p>", designcraft_ui_egui::APP_NAME)),
            }
        }
    });
}

#[cfg(not(feature = "simplehub"))]
fn query() -> String {
    web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default()
}

/// Wraps the app to read dropped files asynchronously (browsers can't read them synchronously)
/// and feed them through the inbox.
struct WebShell {
    app: DesignApp,
    inbox: Inbox,
}

impl eframe::App for WebShell {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped = ctx.input_mut(|i| std::mem::take(&mut i.raw.dropped_files));
        for f in dropped {
            let inbox = self.inbox.clone();
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let name = f.path().file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "dropped".into());
                match f.bytes_async().await {
                    Ok(bytes) => {
                        inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, bytes));
                        ctx.request_repaint();
                    }
                    Err(e) => log::error!("couldn't read dropped file {name}: {e}"),
                }
            });
        }
        self.app.logic(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.ui(ui);
    }
}

#[cfg(not(feature = "simplehub"))]
fn services(inbox: Inbox, ctx: egui::Context) -> Services {
    let open_inbox = inbox.clone();
    Services {
        open_async: Some(Box::new(move |purpose: &str| {
            let inbox = open_inbox.clone();
            let ctx = ctx.clone();
            let dialog = if purpose == "swatches" {
                rfd::AsyncFileDialog::new().add_filter("Swatch Exchange (ASE)", &["ase"])
            } else if purpose == "place" {
                rfd::AsyncFileDialog::new().add_filter("Graphics", IMAGE_EXTS)
            } else {
                rfd::AsyncFileDialog::new().add_filter("DesignCraft", DOC_EXTS)
            };
            wasm_bindgen_futures::spawn_local(async move {
                let Some(file) = dialog.pick_file().await else {
                    return;
                };
                let bytes = file.read().await;
                inbox.lock().unwrap_or_else(|e| e.into_inner()).push((file.file_name(), bytes));
                ctx.request_repaint();
            });
        })),
        // Exports ask for a name; the browser decides where the download goes.
        pick_save: Some(Box::new(|name: &str| Some(name.to_string()))),
        write: Some(Box::new(|name: &str, bytes: &[u8]| download(name, bytes))),
        download: Some(Box::new(|name: &str, bytes: &[u8]| {
            if let Err(e) = download(name, bytes) {
                log::error!("download of {name} failed: {e}");
            }
        })),
        inbox: Some(inbox),
        ..Default::default()
    }
}

#[cfg(not(feature = "simplehub"))]
/// Trigger a browser download of `bytes` named after the last component of `path`.
fn download(path: &str, bytes: &[u8]) -> Result<(), String> {
    let js = |e: wasm_bindgen::JsValue| format!("{e:?}");
    let name = std::path::Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "designcraft".into());
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(mime_for(&name));
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(js)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js)?;
    let a: web_sys::HtmlAnchorElement = document.create_element("a").map_err(js)?.dyn_into().map_err(|_| "not an anchor")?;
    a.set_href(&url);
    a.set_download(&name);
    a.style().set_property("display", "none").map_err(js)?;
    let body = document.body().ok_or("no body")?;
    body.append_child(&a).map_err(js)?;
    a.click();
    a.remove();
    // Revoke after the click has been dispatched; the download keeps its own reference.
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || {
        web_sys::Url::revoke_object_url(&url).ok();
    });
    window.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 10_000).map_err(js)?;
    Ok(())
}

#[cfg(not(feature = "simplehub"))]
fn mime_for(name: &str) -> &'static str {
    match name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("pdf") => "application/pdf",
        Some("designcraft") => "application/json",
        Some("idml") => "application/vnd.adobe.indesign-idml-package",
        _ => "application/octet-stream",
    }
}

/// Simple Layout inside SimpleHub. The page mounts the app on a canvas it owns and hands over a
/// bridge object (`open(): Promise<{name, bytes}|null>`, `save(name, bytes): Promise<void>`) that
/// moves documents to and from Simple Cloud; nothing here touches the browser's file picker.
/// `open` receives the purpose (`open`, `place`, `swatches`) as its only argument so the host may
/// filter file types; the inbox routes whatever comes back by extension (`.designcraft` / `.idml`
/// open a document, `.ase` loads swatches, anything else is placed).
#[cfg(feature = "simplehub")]
mod simplehub {
    use super::{DesignApp, Inbox, Services, Session, WebShell};
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::JsFuture;

    /// A running instance. SimpleHub calls `destroy` when the module unmounts.
    #[wasm_bindgen]
    pub struct LayoutHandle {
        runner: eframe::WebRunner,
    }

    #[wasm_bindgen]
    impl LayoutHandle {
        pub fn destroy(&self) {
            self.runner.destroy();
        }

        pub fn has_panicked(&self) -> bool {
            self.runner.has_panicked()
        }
    }

    /// Starts Simple Layout on `canvas`, with `bridge` providing open and save.
    #[wasm_bindgen]
    pub async fn start_simple_layout(canvas: web_sys::HtmlCanvasElement, bridge: JsValue) -> Result<LayoutHandle, JsValue> {
        eframe::WebLogger::init(log::LevelFilter::Info).ok();
        let runner = eframe::WebRunner::new();
        runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |cc| {
                    if let Some(rs) = &cc.wgpu_render_state {
                        log::info!("simple-layout: wgpu backend {:?}", rs.adapter.get_info().backend);
                    }
                    let inbox: Inbox = Inbox::default();
                    let app = DesignApp::new(Session::new(), services(bridge, inbox.clone(), cc.egui_ctx.clone()));
                    Ok(Box::new(WebShell { app, inbox }))
                }),
            )
            .await?;
        Ok(LayoutHandle { runner })
    }

    fn services(bridge: JsValue, inbox: Inbox, ctx: egui::Context) -> Services {
        let open_bridge = bridge.clone();
        let open_inbox = inbox.clone();
        let save_bridge = bridge.clone();
        Services {
            open_async: Some(Box::new(move |purpose: &str| {
                let bridge = open_bridge.clone();
                let inbox = open_inbox.clone();
                let ctx = ctx.clone();
                let purpose = purpose.to_string();
                wasm_bindgen_futures::spawn_local(async move {
                    match call_open(&bridge, &purpose).await {
                        Ok(Some((name, bytes))) => {
                            inbox.lock().unwrap_or_else(|e| e.into_inner()).push((name, bytes));
                            ctx.request_repaint();
                        }
                        Ok(None) => {}
                        Err(e) => log::error!("SimpleHub open failed: {e:?}"),
                    }
                });
            })),
            // Exports ask for a name; the bridge decides where it goes.
            pick_save: Some(Box::new(|name: &str| Some(name.to_string()))),
            write: Some(Box::new(move |name: &str, bytes: &[u8]| {
                save(&save_bridge, name, bytes);
                Ok(())
            })),
            download: Some(Box::new(move |name: &str, bytes: &[u8]| save(&bridge, name, bytes))),
            inbox: Some(inbox),
            ..Default::default()
        }
    }

    /// Hands `bytes` to the bridge as the file `name` (the last path component).
    fn save(bridge: &JsValue, name: &str, bytes: &[u8]) {
        let bridge = bridge.clone();
        let name = std::path::Path::new(name).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Document.designcraft".into());
        let bytes = js_sys::Uint8Array::from(bytes);
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = call_save(&bridge, &name, &bytes).await {
                log::error!("SimpleHub save of {name} failed: {e:?}");
            }
        });
    }

    fn method(bridge: &JsValue, name: &str) -> Result<js_sys::Function, JsValue> {
        js_sys::Reflect::get(bridge, &JsValue::from_str(name))?
            .dyn_into::<js_sys::Function>()
            .map_err(|_| JsValue::from_str(&format!("bridge.{name} is not a function")))
    }

    async fn call_open(bridge: &JsValue, purpose: &str) -> Result<Option<(String, Vec<u8>)>, JsValue> {
        let promise: js_sys::Promise = method(bridge, "open")?.call1(bridge, &JsValue::from_str(purpose))?.dyn_into()?;
        let result = JsFuture::from(promise).await?;
        if result.is_null() || result.is_undefined() {
            return Ok(None);
        }
        let name = js_sys::Reflect::get(&result, &JsValue::from_str("name"))?.as_string().unwrap_or_else(|| "Document.designcraft".into());
        let bytes: js_sys::Uint8Array = js_sys::Reflect::get(&result, &JsValue::from_str("bytes"))?.dyn_into()?;
        Ok(Some((name, bytes.to_vec())))
    }

    async fn call_save(bridge: &JsValue, name: &str, bytes: &js_sys::Uint8Array) -> Result<(), JsValue> {
        let ret = method(bridge, "save")?.call2(bridge, &JsValue::from_str(name), bytes)?;
        if let Ok(promise) = ret.dyn_into::<js_sys::Promise>() {
            JsFuture::from(promise).await?;
        }
        Ok(())
    }
}
