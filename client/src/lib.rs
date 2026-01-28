use leptos::*;
use leptos_router::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast; // Required for unchecked_into
use web_sys::{WebSocket, MessageEvent, ErrorEvent};
use pulldown_cmark::{Parser, Options, html};
use gloo_net::http::Request;
use web_sys::RequestCredentials;

// --- CONFIGURATION HELPERS ---
fn api_base() -> &'static str {
    option_env!("API_URL").unwrap_or("http://localhost:3000")
}

fn ws_base() -> &'static str {
    option_env!("WS_URL").unwrap_or("ws://localhost:3000")
}

mod dashboard;
use dashboard::DashboardPage;

#[component]
fn LandingPage() -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    let (checked, set_checked) = create_signal(false);

    {
        let navigate = navigate.clone();
        create_resource(|| (), move |_| {
            let navigate = navigate.clone();
            async move {
                // FIX: Use dynamic API URL
                let url = format!("{}/api/me", api_base());
                let auth_req = Request::get(&url)
                    .credentials(RequestCredentials::Include)
                    .send()
                    .await;

                match auth_req {
                    Ok(resp) => {
                        if resp.ok() {
                            if let Ok(_user) = resp.json::<dashboard::User>().await {
                                // User is authenticated, redirect to dashboard
                                navigate("/dashboard", Default::default());
                            }
                        }
                    }
                    Err(_) => {}
                }
                set_checked.set(true);
            }
        });
    }

    view! {
        {move || {
            if !checked.get() {
                return view! {
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center; background: var(--bg-dark);">
                        <div class="spinner"></div>
                    </div>
                }.into_view();
            }

            view! {
                <div style="display: flex; flex-direction: column; height: 100vh; justify-content: center; align-items: center; gap: 40px; background: var(--bg-dark);">
                    <div style="text-align: center;">
                        <h1 style="font-size: 4rem; font-weight: 800; color: var(--text-main); margin: 0 0 16px 0; letter-spacing: -1px;">
                            "TryCLI Studio"
                        </h1>
                        <p style="font-size: 1.2rem; color: var(--text-muted); margin: 0 0 8px 0;">
                            "Create and share interactive CLI experiences"
                        </p>
                        <p style="font-size: 1rem; color: var(--text-muted); margin: 0;">
                            "Build, demo, and deploy your tools instantly"
                        </p>
                    </div>

                    // FIX: Use dynamic API URL
                    <a href=format!("{}/auth/github", api_base()) 
                       class="btn-primary"
                       style="padding: 14px 32px; font-size: 1.1rem; text-decoration: none;">
                        "Sign in with GitHub"
                    </a>
                </div>
            }.into_view()
        }}
    }
}

#[component]
fn ProtectedRoute(children: Children) -> impl IntoView {
    let (user, set_user) = create_signal(None::<dashboard::User>);
    let (checked, set_checked) = create_signal(false);

    create_resource(|| (), move |_| async move {
        // FIX: Use dynamic API URL
        let url = format!("{}/api/me", api_base());
        let auth_req = Request::get(&url)
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        match auth_req {
            Ok(resp) => {
                if resp.ok() {
                    if let Ok(u) = resp.json::<dashboard::User>().await {
                        set_user.set(Some(u));
                    }
                }
            }
            Err(_) => {}
        }
        set_checked.set(true);
    });

    let children_view = children();

    view! {
        {move || {
            if !checked.get() {
                return view! {
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center;">
                        <div class="spinner"></div>
                    </div>
                }.into_view();
            }

            if user.get().is_some() {
                children_view.clone().into_view()
            } else {
                view! {
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center; flex-direction: column; gap: 20px; background: var(--bg-dark);">
                        <h2 style="color: var(--text-main);">"Access Denied"</h2>
                        <p style="color: var(--text-muted);">"Please log in to access this page."</p>
                        // FIX: Use dynamic API URL
                        <a href=format!("{}/auth/github", api_base()) class="btn-primary" style="text-decoration: none;">
                            "Sign in with GitHub"
                        </a>
                    </div>
                }.into_view()
            }
        }}
    }
}

// --- BINDING 1: FitAddon ---
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
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/" view=LandingPage />
                <Route path="/dashboard" view=move || view! {
                    <ProtectedRoute>
                        <DashboardPage />
                    </ProtectedRoute>
                } />
                <Route path="/new" view=move || view! {
                    <ProtectedRoute>
                        <CreatePage />
                    </ProtectedRoute>
                } />
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
        let url = format!("{}/api/me", api_base());
        let auth_req = Request::get(&url)
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        match auth_req {
            Ok(resp) => {
                if resp.ok() {
                    if let Ok(u) = resp.json::<User>().await {
                        set_user.set(Some(u));
                        
                        let spawn_url = format!("{}/api/spawn", api_base());
                        let spawn_req = Request::post(&spawn_url)
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
            let body_data = serde_json::json!({
                "container_id": container_id.get(),
                "slug": slug.get(),
                "markdown": markdown.get()
            });

            // FIX: Safe serialization instead of unwrap()
            let body_str = match serde_json::to_string(&body_data) {
                Ok(s) => s,
                Err(_) => {
                    let _ = window().alert_with_message("Failed to serialize request");
                    return;
                }
            };

            let url = format!("{}/api/publish", api_base());
            
            // FIX: Safe Request building
            let req = Request::post(&url)
                .header("Content-Type", "application/json")
                .credentials(RequestCredentials::Include)
                .body(body_str);

            if let Ok(r) = req {
                match r.send().await {
                    Ok(resp) => {
                        if resp.ok() {
                            let _ = window().alert_with_message("Published!");
                        } else {
                            let _ = window().alert_with_message("Publish Failed: Server rejected request");
                        }
                    },
                    Err(_) => {
                        let _ = window().alert_with_message("Publish Failed: Network Error");
                    }
                }
            } else {
                let _ = window().alert_with_message("Failed to build request");
            }
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
                     <a href=format!("{}/auth/logout", api_base()) 
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
                    <a href=format!("{}/auth/github", api_base()) class="btn-primary" style="text-decoration: none;">
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
            let url = format!("{}/api/project/{}/{}", api_base(), u, s);
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
    
    // FIX: Using window.location.origin for the embed code
    let copy_embed_code = move |u: String, s: String| {
        let origin = window().location().origin().unwrap_or("http://localhost:8080".to_string());
        let code = format!(
            "<iframe src=\"{}/embed/{}/{}\" width=\"100%\" height=\"500px\" frameborder=\"0\" allowtransparency=\"true\" loading=\"lazy\"></iframe>",
            origin, u, s
        );
        let _ = window().navigator().clipboard().write_text(&code);
        let _ = window().alert_with_message("Embed code copied to clipboard!");
    };

    let project_data = create_resource(
        move || (username(), slug()), 
        |(u, s)| async move {
            let url = format!("{}/api/project/{}/{}", api_base(), u, s);
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

            term.write(&format!("Connecting to session {}...\r\n", id_for_effect));
            
            let term_clone: Terminal = term.clone().unchecked_into();
            let ws_url = format!("{}/ws/{}", ws_base(), id_for_effect);
            
            // FIX: Removed unwrap() on WebSocket::new
            match WebSocket::new(&ws_url) {
                Ok(ws) => {
                    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
                        if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                            term_clone.write(&String::from(text));
                        }
                    });
                    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                    onmessage.forget();

                    let ws_clone = ws.clone();
                    let on_data_callback = Closure::<dyn FnMut(String)>::new(move |data: String| {
                        if ws_clone.ready_state() == WebSocket::OPEN {
                            let _ = ws_clone.send_with_str(&data);
                        }
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

#[wasm_bindgen(start)] 
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App/> })
}