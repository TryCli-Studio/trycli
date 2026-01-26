use leptos::*;
use leptos_router::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent};
use pulldown_cmark::{Parser, Options, html};

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
    view! {
        <Router>
            <Routes>
                <Route path="/new" view=CreatePage />
                <Route path="/project/:slug" view=ViewPage />
            </Routes>
        </Router>
    }
}

// PAGE 1: CREATE (/new)
#[component]
fn CreatePage() -> impl IntoView {
    // 1. Define State
    let (container_id, set_container_id) = create_signal("".to_string());
    let (markdown, set_markdown) = create_signal("# My Awesome Tool\n\nRun the install command to get started...".to_string());
    let (slug, set_slug) = create_signal("demo-project".to_string());

    // 2. On Load: Call Server to get a fresh container
    create_resource(|| (), move |_| async move {
        let client = reqwest::Client::new();
        let res = client.post("http://localhost:3000/api/spawn").send().await.unwrap();
        let id = res.json::<String>().await.unwrap();
        set_container_id.set(id);
    });

    // 3. Define the Publish Action
    let on_publish = move |_| {
        spawn_local(async move {
            let client = reqwest::Client::new();
            let _ = client.post("http://localhost:3000/api/publish")
                .json(&serde_json::json!({
                    "container_id": container_id.get(),
                    "slug": slug.get(),
                    "markdown": markdown.get()
                }))
                .send().await;
            window().alert_with_message("Published!").unwrap();
        });
    };

    view! {
        <div class="nav">
            <div class="brand">"TryCLI Studio"</div>
            <div class="controls">
                <span style="color: var(--text-muted); font-size: 0.9rem;">"trycli.com /"</span>
                <input type="text" class="input-slug" 
                       on:input=move |ev| set_slug.set(event_target_value(&ev)) 
                       prop:value=slug />
                <button class="btn-primary" on:click=on_publish prop:disabled=move || container_id.get().is_empty()>"Publish Demo"</button>
            </div>
        </div>

        <div class="workspace">
            // Left Pane: Terminal
            <div class="pane">
                <div class="terminal-header">
                    <div class="dot red"></div>
                    <div class="dot yellow"></div>
                    <div class="dot green"></div>
                    <span class="terminal-title">"bash — interactive"</span>
                </div>
                <div class="terminal-body">
                    {move || match container_id.get().as_str() {
                        "" => view! { <div style="padding: 20px; color: #666;">"Initializing Environment..."</div> }.into_view(),
                        id => view! { <TerminalView container_id=id.to_string() /> }.into_view()
                    }}
                </div>
            </div>
            
            // Right Pane: Editor
            <div class="pane">
                 <textarea class="editor-textarea"
                    spellcheck="false"
                    on:input=move |ev| set_markdown.set(event_target_value(&ev))
                 >{markdown}</textarea>
            </div>
        </div>
    }
}

// PAGE 2: VIEW (/project/:slug)
#[component]
fn ViewPage() -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
    
    // 1. Fetch project data
    let project_data = create_resource(slug, |s| async move {
        let url = format!("http://localhost:3000/api/project/{}", s);
        reqwest::get(&url).await.unwrap()
            .json::<serde_json::Value>().await.unwrap()
    });

    view! {
        {move || match project_data.get() {
            Some(data) => {
                let cid = data["container_id"].as_str().unwrap().to_string();
                let md_raw = data["markdown"].as_str().unwrap().to_string();
                let html_output = render_markdown(&md_raw);

                view! {
                     <div class="nav">
                        <div class="brand">"TryCLI"</div>
                        <div class="controls">
                            <button class="btn-primary" style="background: #27272a;">"Clone Repo"</button>
                        </div>
                     </div>

                     <div class="workspace">
                        // Left: Markdown Read-Mode
                        <div class="pane" style="background: var(--bg-dark);">
                            <div class="markdown-body" inner_html=html_output />
                        </div>

                        // Right: Terminal
                        <div class="pane">
                            <div class="terminal-header">
                                <div class="dot red"></div>
                                <div class="dot yellow"></div>
                                <div class="dot green"></div>
                                <span class="terminal-title">"Live Demo"</span>
                            </div>
                            <div class="terminal-body">
                                <TerminalView container_id=cid />
                            </div>
                        </div>
                     </div>
                }.into_view()
            },
            None => view! { <div style="padding: 50px; text-align: center;">"Loading Project..."</div> }.into_view()
        }}
    }
}

// HELPER: Markdown Parser
fn render_markdown(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser); 
    
    html_output
}

// SHARED COMPONENT: TERMINAL
#[component]
fn TerminalView(container_id: String) -> impl IntoView {
    // Avoid name collision with pulldown_cmark::html
    let terminal_div_ref = create_node_ref::<leptos::html::Div>();
    
    let id_for_effect = container_id.clone();

    create_effect(move |_| {
        if let Some(div) = terminal_div_ref.get() {
            let term = Terminal::new();
            term.open(&div);
            term.write(&format!("Connecting to session {}...\r\n", id_for_effect));

            let term_clone: Terminal = term.clone().unchecked_into();

            let ws_url = format!("ws://localhost:3000/ws/{}", id_for_effect);
            let ws = WebSocket::new(&ws_url).unwrap();
            
            let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                    term_clone.write(&String::from(text));
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
        }
    });

    view! { <div _ref=terminal_div_ref class="terminal" style="height: 100%; width: 100%;"></div> }
}

#[wasm_bindgen(start)] 
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App/> })
}