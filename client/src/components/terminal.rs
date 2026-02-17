use crate::api::ws_base;
use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // Required for unchecked_into
use web_sys::{MessageEvent, WebSocket}; // Removed unused ErrorEvent
use std::rc::Rc;
use std::cell::Cell;

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
    #[wasm_bindgen(method, getter)]
    fn cols(this: &Terminal) -> u16;
    #[wasm_bindgen(method, getter)]
    fn rows(this: &Terminal) -> u16;
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

            term.write(&format!("Connecting to session {}...\r\n", id_for_effect));

            let ws_url = format!("{}/ws/{}", ws_base(), id_for_effect);
            
            // 1. CAST: Ensure clones are treated as Terminal type, not generic JsValue
            let term_clone = term.clone().unchecked_into::<Terminal>(); 
            let term_resize = term.clone().unchecked_into::<Terminal>(); 
            let first_message = Rc::new(Cell::new(true));

            match WebSocket::new(&ws_url) {
                Ok(ws) => {
                    let ws_resize = ws.clone();
                    let fit_addon_resize = fit_addon.clone().unchecked_into::<XtermFitAddon>();
                    
                    let on_resize = Closure::<dyn FnMut()>::new(move || {
                        fit_addon_resize.fit();
                        // Now methods like cols() work because term_resize is typed
                        let cols = term_resize.cols();
                        let rows = term_resize.rows();
                        let _ = ws_resize.send_with_str(&format!("RESIZE:{}:{}", cols, rows));
                    });
                    
                    window().set_onresize(Some(on_resize.as_ref().unchecked_ref()));
                    on_resize.forget();

                    let ws_cleanup = ws.clone();
                    on_cleanup(move || {
                        let _ = ws_cleanup.close();
                        window().set_onresize(None); 
                    });

                    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            let text = String::from(txt);
                            if first_message.get() {
                                term_clone.write("\x1b[2J\x1b[H"); 
                                first_message.set(false);
                            }
                            term_clone.write(&text);
                        }
                    });
                    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                    onmessage.forget();

                    let ws_send = ws.clone();
                    let on_data_callback = Closure::<dyn FnMut(String)>::new(move |data: String| {
                        let _ = ws_send.send_with_str(&data);
                    });
                    term.on_data(&on_data_callback);
                    on_data_callback.forget();
                }
                Err(_) => {
                    term.write("\r\n\x1b[31m[!] WebSocket Error.\x1b[0m\r\n");
                }
            }
        }
    });

    view! { <div _ref=terminal_div_ref class="terminal" style="height: 100%; width: 100%; padding: 8px"></div> }
}