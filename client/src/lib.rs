use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // Essential for casting types
use web_sys::{WebSocket, MessageEvent};

// Bindings to the JS Xterm library
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
}

#[component]
pub fn App() -> impl IntoView {
    let terminal_div_ref = create_node_ref::<html::Div>();

    create_effect(move |_| {
        if let Some(div) = terminal_div_ref.get() {
            // 1. Initialize Terminal
            let term = Terminal::new();
            term.open(&div);
            term.write("Connecting to TryCLI environment...\r\n");

            // 2. Clone the terminal for the WebSocket closure
            // We explicitly say ": Terminal" so Rust knows exactly what type this is.
            let term_clone: Terminal = term.clone().unchecked_into();

            let container_id = option_env!("CONTAINER_ID").unwrap_or("demo");

            // 3. Connect WebSocket
            let ws_url = format!("ws://localhost:3000/ws/{}", container_id);
            let ws = WebSocket::new(&ws_url).unwrap();
            
            // 4. Handle Incoming Data (Server -> Browser)
            let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                    // term_clone is now guaranteed to be 'Terminal', so .write() will exist
                    term_clone.write(&String::from(text));
                }
            });
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget(); // Keep memory alive

            // 5. Handle Outgoing Data (Browser -> Server)
            let ws_clone = ws.clone();
            let on_data_callback = Closure::<dyn FnMut(String)>::new(move |data: String| {
                match ws_clone.send_with_str(&data) {
                    Ok(_) => {},
                    Err(err) => console_error(&err),
                }
            });
            term.on_data(&on_data_callback);
            on_data_callback.forget();
        }
    });

    view! {
        <div class="container" style="display: flex; height: 100vh;">
            <div class="editor" style="width: 50%;">
                <textarea style="width: 100%; height: 100%;">"# Instructions\n\nWrite your CLI tutorial here..."</textarea>
            </div>
            <div _ref=terminal_div_ref class="terminal" style="width: 50%; background: black;"></div>
        </div>
    }
}

// Helper to log errors to JS console
fn console_error(e: &JsValue) {
    web_sys::console::error_1(e);
}

#[wasm_bindgen(start)] 
pub fn main() {
    // Optional: better error messages in the console if it crashes
    console_error_panic_hook::set_once();
    
    leptos::mount_to_body(|| view! { <App/> })
}