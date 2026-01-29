use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use wasm_bindgen::JsValue;
use crate::api::api_base;
use crate::types::User;
use crate::components::terminal::TerminalView;

#[component]
pub fn CreatePage() -> impl IntoView {
    let query_params = use_query_map();
    let pre_filled_name = move || {
        query_params.with(|params| {
            params.get("name").cloned().unwrap_or_default()
        })
    };

    let (container_id, set_container_id) = create_signal("".to_string());
    let (markdown, set_markdown) = create_signal("# My Awesome Tool\n\nRun the install command...".to_string());
    let (slug, set_slug) = create_signal(pre_filled_name());
    let (slug_error, set_slug_error) = create_signal(None::<String>);
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
    let navigate = leptos_router::use_navigate();
    view! {
       <div class="nav">
            <div class="brand" style="cursor: pointer;" on:click=move |_| {
                if user.get().is_some() {
                    navigate("/dashboard", Default::default());
                } else {
                    navigate("/", Default::default());
                }
            }>
                "TryCli Studio"
            </div>
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
                            on:input=move |ev| {
                                let value = event_target_value(&ev);
                                // Only allow alphanumeric characters and hyphens (Docker-safe)
                                if value.is_empty() || value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                                    set_slug.set(value);
                                    set_slug_error.set(None);
                                } else {
                                    set_slug_error.set(Some("Only letters, numbers, and hyphens allowed".to_string()));
                                }
                            }
                            prop:value=slug />
                     {move || slug_error.get().map(|err| view! {
                         <span style="color: #ef4444; font-size: 0.75rem; margin-left: 8px;">{err}</span>
                     })}
                     <button class="btn-primary" on:click=on_publish 
                             prop:disabled=move || container_id.get().is_empty() || slug_error.get().is_some()>"Publish"</button>
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
                    <h2 style="color: var(--text-main);">"Welcome to TryCli Studio"</h2>
                    <p style="color: var(--text-muted);">"Please sign in to start creating interactive demos."</p>
                </div>
            }.into_view()
        }}
    }
}
