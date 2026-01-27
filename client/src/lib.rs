use leptos::*;
use leptos_router::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // Required for unchecked_into
use web_sys::{WebSocket, MessageEvent};
use pulldown_cmark::{Parser, Options, html};
use gloo_net::http::Request;
use web_sys::RequestCredentials;

// --- BINDING 1: FitAddon ---
// We define this in its own block to avoid macro conflicts.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = FitAddon)]
    type XtermFitAddon;

    #[wasm_bindgen(constructor, js_namespace = FitAddon, js_class = "FitAddon")]
    fn new() -> XtermFitAddon;

    #[wasm_bindgen(method)]
    fn fit(this: &XtermFitAddon);
}

// --- BINDING 2: Terminal ---
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

    // UPDATE: Use the renamed type here
    #[wasm_bindgen(method, js_name = loadAddon)]
    fn load_addon(this: &Terminal, addon: &XtermFitAddon);
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/new" view=CreatePage />
                <Route path="/:username/:slug" view=ViewPage />
                <Route path="/embed/:username/:slug" view=EmbedPage />
            </Routes>
        </Router>
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
struct User {
    login: String,
    avatar_url: String,
}

#[component]
fn CreatePage() -> impl IntoView {
    let (container_id, set_container_id) = create_signal("".to_string());
    let (markdown, set_markdown) = create_signal("# My Awesome Tool\n\nRun the install command...".to_string());
    let (slug, set_slug) = create_signal("demo-project".to_string());
    let (user, set_user) = create_signal(None::<User>);

    create_resource(|| (), move |_| async move {
        let auth_req = Request::get("http://localhost:3000/api/me")
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        match auth_req {
            Ok(resp) => {
                if resp.ok() {
                    if let Ok(u) = resp.json::<User>().await {
                        set_user.set(Some(u));

                        let spawn_req = Request::post("http://localhost:3000/api/spawn")
                            .credentials(RequestCredentials::Include)
                            .send()
                            .await;
                            
                        match spawn_req {
                            Ok(spawn_resp) => {
                                if spawn_resp.ok() {
                                    if let Ok(id) = spawn_resp.json::<String>().await {
                                        set_container_id.set(id);
                                    }
                                } else {
                                    let status = spawn_resp.status();
                                    let text = spawn_resp.text().await.unwrap_or_default();
                                    set_container_id.set(format!("ERROR {}: {}", status, text));
                                }
                            }
                            Err(e) => {
                                set_container_id.set(format!("NETWORK_FAIL: {}", e));
                            }
                        }
                    }
                }
            }
            Err(e) => web_sys::console::log_1(&JsValue::from_str(&format!("Auth Error: {}", e))),
        }
    });

    let on_publish = move |_| {
        spawn_local(async move {
            let body = serde_json::json!({
                "container_id": container_id.get(),
                "slug": slug.get(),
                "markdown": markdown.get()
            });

            let _ = Request::post("http://localhost:3000/api/publish")
                .header("Content-Type", "application/json")
                .credentials(RequestCredentials::Include)
                .body(body.to_string()).unwrap()
                .send()
                .await;

            window().alert_with_message("Published!").unwrap();
        });
    };

    view! {
       <div class="nav">
        <div class="brand">"TryCLI Studio"</div>
        <div class="controls">
            {move || match user.get() {
                Some(u) => view! {
                     <div style="display: flex; align-items: center; margin-right: 20px;">
                        <img src=u.avatar_url 
                             style="width: 24px; height: 24px; border-radius: 50%; margin-right: 8px; border: 1px solid var(--border);" />
                        <span style="color: var(--text-muted); font-size: 0.9rem;">
                            {u.login.clone()}
                        </span>
                     </div>

                     <a href="http://localhost:3000/auth/logout" 
                        class="btn-primary" 
                        style="background: #27272a; margin-right: 12px; text-decoration: none; font-size: 0.8rem; border: 1px solid var(--border);">
                        "Logout"
                     </a>

                     <span style="color: var(--text-muted); font-size: 0.9rem; margin-right: 8px;">"Project Slug:"</span>
                     <input type="text" class="input-slug" 
                            on:input=move |ev| set_slug.set(event_target_value(&ev)) 
                            prop:value=slug />
                     <button class="btn-primary" on:click=on_publish 
                             prop:disabled=move || container_id.get().is_empty()>"Publish"</button>
                }.into_view(),
                None => view! {
                    <a href="http://localhost:3000/auth/github" class="btn-primary" style="text-decoration: none;">
                        "Login with GitHub"
                    </a>
                }.into_view()
            }}
        </div>
       </div>

        {move || match user.get() {
            Some(_) => view! {
                <div class="workspace">
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
                    
                    <div class="pane">
                         <textarea class="editor-textarea"
                            spellcheck="false"
                            on:input=move |ev| set_markdown.set(event_target_value(&ev))
                         >{markdown}</textarea>
                    </div>
                </div>
            }.into_view(),
            None => view! {
                <div style="display: flex; height: calc(100vh - 60px); justify-content: center; align-items: center; flex-direction: column; gap: 20px;">
                    <h2 style="color: var(--text-main);">"Welcome to TryCLI"</h2>
                    <p style="color: var(--text-muted);">"Please sign in to start creating interactive demos."</p>
                </div>
            }.into_view()
        }}
    }
}

#[component]
fn EmbedPage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
    
    let (started, set_started) = create_signal(false);

    let project_data = create_resource(
        move || (started.get(), username(), slug()), 
        |(is_started, u, s)| async move {
            if !is_started { return None; } 
            
            let url = format!("http://localhost:3000/api/project/{}/{}", u, s);
            let req = Request::get(&url).send().await;
            
            match req {
                Ok(resp) => resp.json::<serde_json::Value>().await.ok(),
                Err(_) => None
            }
        }
    );

    view! {
        <div class="embed-container" style="width: 100vw; height: 100vh; background: #000; overflow: hidden; position: relative;">
            {move || if !started.get() {
                view! {
                    <div class="embed-overlay" 
                         style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background: rgba(0,0,0,0.8); z-index: 10;">
                        <div style="text-align: center; color: white;">
                            <h3 style="margin-bottom: 1rem; font-family: var(--font-sans);">"TryCLI Demo"</h3>
                            <button class="btn-primary" 
                                    style="padding: 12px 24px; font-size: 1.1rem;"
                                    on:click=move |_| set_started.set(true)>
                                "▶ Start Terminal"
                            </button>
                            <p style="margin-top: 1rem; color: #666; font-size: 0.8rem;">"Powered by TryCLI"</p>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}

            {move || match project_data.get() {
                Some(Some(data)) => {
                    let cid = data["container_id"].as_str().unwrap_or_default().to_string();
                    view! { <TerminalView container_id=cid /> }.into_view()
                },
                Some(None) => view! { <div style="color:red; padding:20px;">"Project not found"</div> }.into_view(),
                None => {
                    if started.get() {
                        view! { <div style="color: #666; padding: 20px;">"Booting Container..."</div> }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }
            }}
        </div>
    }
}

#[component]
fn ViewPage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();    
    
    let copy_embed_code = move |u: String, s: String| {
        let code = format!(
            "<iframe src=\"http://localhost:8080/embed/{}/{}\" width=\"100%\" height=\"500px\" frameborder=\"0\" allowtransparency=\"true\" loading=\"lazy\"></iframe>",
            u, s
        );
        let _ = window().navigator().clipboard().write_text(&code);
        let _ = window().alert_with_message("Embed code copied to clipboard!");
    };

    let project_data = create_resource(
        move || (username(), slug()), 
        |(u, s)| async move {
            let url = format!("http://localhost:3000/api/project/{}/{}", u, s);
            let req = Request::get(&url).send().await;
            match req {
                Ok(resp) => resp.json::<serde_json::Value>().await.ok(),
                Err(_) => None
            }
        }
    );

    view! {
        {move || match project_data.get() {
            Some(Some(data)) => {
                let cid = data["container_id"].as_str().unwrap_or_default().to_string();
                let md_raw = data["markdown"].as_str().unwrap_or_default().to_string();
                let html_output = render_markdown(&md_raw);
                
                let u_clone = username();
                let s_clone = slug();

                view! {
                     <div class="nav">
                        <div class="brand">"TryCLI"</div>
                        <div class="controls">
                            <button class="btn-primary" 
                                    style="background: #27272a; border: 1px solid var(--border); margin-right: 10px;"
                                    on:click=move |_| copy_embed_code(u_clone.clone(), s_clone.clone())>
                                "Share / Embed"
                            </button>
                        </div>
                     </div>

                     <div class="workspace">
                        <div class="pane" style="background: var(--bg-dark);">
                            <div class="markdown-body" inner_html=html_output />
                        </div>
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
            Some(None) => view! { <div style="color: var(--text-muted); text-align: center; margin-top: 50px;">"Project not found."</div> }.into_view(),
            None => view! { <div style="padding: 50px; text-align: center;">"Loading Project..."</div> }.into_view()
        }}
    }
}

fn render_markdown(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser); 
    
    html_output
}

#[component]
fn TerminalView(container_id: String) -> impl IntoView {
    let terminal_div_ref = create_node_ref::<leptos::html::Div>();
    
    let id_for_effect = container_id.clone();

    create_effect(move |_| {
        if let Some(div) = terminal_div_ref.get() {
            let term = Terminal::new();
            
            // --- FitAddon Logic ---
            let fit_addon = XtermFitAddon::new();
            term.load_addon(&fit_addon);
            
            term.open(&div);
            
            // Initial fit
            fit_addon.fit(); 
            
            let fit_addon_clone = fit_addon.clone().unchecked_into::<XtermFitAddon>();
            
            let on_resize = Closure::<dyn FnMut()>::new(move || {
                fit_addon_clone.fit();
            });
            window().set_onresize(Some(on_resize.as_ref().unchecked_ref()));
            on_resize.forget();

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

    view! { <div _ref=terminal_div_ref class="terminal" style="height: 100%; width: 100%; padding: 8px"></div> }
}

#[wasm_bindgen(start)] 
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App/> })
}