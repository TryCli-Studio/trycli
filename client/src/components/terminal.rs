use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, ErrorEvent};
use crate::api::ws_base;

// BINDING 1: FitAddon 
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = FitAddon)]
    type XtermFitAddon;
    #[wasm_bindgen(constructor, js_namespace = FitAddon, js_class = "FitAddon")]
    fn new() -> XtermFitAddon;
    #[wasm_bindgen(method)]
    fn fit(this: &XtermFitAddon);
}

//  BINDING 2: Terminal 
#[wasm_bindgen]
extern "C" {
    type Terminal;
    #[wasm_bindgen(constructor, js_namespace = window)]
    fn new() -> Terminal;
    #[wasm_bindgen(method)]
    fn open(this: &Terminal, parent: &web_sys::HtmlDivElement);
    #[wasm_bindgen(method)]
    fn write(this: &Terminal, data: &str);
    #[wasm_bindgen(method, js_name = onData)]
    fn on_data(this: &Terminal, callback: &Closure<dyn FnMut(String)>);
    #[wasm_bindgen(method, js_name = loadAddon)]
    fn load_addon(this: &Terminal, addon: &XtermFitAddon);
}

#[component]
pub fn TerminalView(container_id: String) -> impl IntoView {
    let terminal_div_ref = create_node_ref::<leptos::html::Div>();
    let id_for_effect = container_id.clone();

    create_effect(move |_| {
        if let Some(div) = terminal_div_ref.get() {
            let term = Terminal::new();
            
            let fit_addon = XtermFitAddon::new();
            term.load_addon(&fit_addon);
            term.open(&div);
            
            fit_addon.fit(); 
            let fit_addon_clone = fit_addon.clone().unchecked_into::<XtermFitAddon>();
            let on_resize = Closure::<dyn FnMut()>::new(move || {
                fit_addon_clone.fit();
            });
            window().set_onresize(Some(on_resize.as_ref().unchecked_ref()));
            on_resize.forget();
            on_cleanup(move || {
                window().set_onresize(None);
            });
            term.write(&format!("Connecting to session {}...\r\n", id_for_effect));
            
            let term_clone: Terminal = term.clone().unchecked_into();
            let ws_url = format!("{}/ws/{}", ws_base(), id_for_effect);
            
            // FIX: Removed unwrap() on WebSocket::new
            match WebSocket::new(&ws_url) {
                Ok(ws) => {
                    let ws_cleanup = ws.clone();
                    
                    on_cleanup(move || {
                        let _ = ws_cleanup.close();
                    });
                    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            term_clone.write(&String::from(txt));
                        }
                    });
                    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                    onmessage.forget();

                    let ws_clone = ws.clone();
                    let on_data_callback = Closure::<dyn FnMut(String)>::new(move |data: String| {
                        let _ = ws_clone.send_with_str(&data);
                    });
                    term.on_data(&on_data_callback);
                    on_data_callback.forget();

                    let term_err = term.clone().unchecked_into::<Terminal>();
                    let onerror = Closure::<dyn FnMut(ErrorEvent)>::new(move |_| {
                         term_err.write("\r\n\x1b[31m[!] Connection Error.\x1b[0m\r\n");
                    });
                    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                    onerror.forget();

                    let term_close = term.clone().unchecked_into::<Terminal>();
                    let onclose = Closure::<dyn FnMut()>::new(move || {
                         term_close.write("\r\n\x1b[33m[!] Connection Closed.\x1b[0m\r\n");
                    });
                    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
                    onclose.forget();
                },
                Err(_) => {
                    term.write("\r\n\x1b[31m[!] Failed to initialize WebSocket connection.\x1b[0m\r\n");
                }
            }
        }
    });

    view! { <div _ref=terminal_div_ref class="terminal" style="height: 100%; width: 100%; padding: 8px"></div> }
}
