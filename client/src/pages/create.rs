use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::api::api_base;
use crate::types::User;
use crate::components::terminal::TerminalView;

// Simple resize divider setup
fn setup_resize_divider() {
    if let Some(divider) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(".resize-divider").ok().flatten())
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let is_dragging = Rc::new(RefCell::new(false));
        
        let on_mousedown = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                *is_dragging.borrow_mut() = true;
            }) as Box<dyn Fn(web_sys::MouseEvent)>)
        };
        
        let on_mousemove = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                if !*is_dragging.borrow() {
                    return;
                }
                
                if let Some(workspace) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.query_selector(".workspace").ok().flatten())
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    let workspace_width = workspace.offset_width() as f64;
                    let workspace_left = workspace.offset_left() as f64;
                    let relative_x = e.client_x() as f64 - workspace_left;
                    let percentage = (relative_x / workspace_width * 100.0).max(20.0).min(80.0);
                    
                    if let Ok(panes) = workspace.query_selector_all(".pane") {
                        if panes.length() >= 2 {
                            if let Some(first_pane) = panes.get(0).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                                first_pane.style().set_property("flex", "0 1 auto").ok();
                                first_pane.style().set_property("width", &format!("{}%", percentage)).ok();
                            }
                            if let Some(second_pane) = panes.get(1).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                                second_pane.style().set_property("flex", "0 1 auto").ok();
                                second_pane.style().set_property("width", &format!("{}%", 100.0 - percentage)).ok();
                            }
                        }
                    }
                }
            }) as Box<dyn Fn(web_sys::MouseEvent)>)
        };
        
        let on_mouseup = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                *is_dragging.borrow_mut() = false;
            }) as Box<dyn Fn(web_sys::MouseEvent)>)
        };
        
        divider.add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref()).ok();
        on_mousedown.forget();
        
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document.add_event_listener_with_callback("mousemove", on_mousemove.as_ref().unchecked_ref()).ok();
            document.add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref()).ok();
            on_mousemove.forget();
            on_mouseup.forget();
        }
    }
}

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
                "container_id": container_id.get_untracked(),
                "slug": slug.get_untracked(),
                "markdown": markdown.get_untracked()
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
                            let status = resp.status();
                            let text = resp.text().await.unwrap_or_default();
                            let _ = window().alert_with_message(&format!("Publish Failed ({}: {})", status, text));
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
                        class="btn-primary btn-logout" 
                        style="margin-right: 12px; text-decoration: none; font-size: 0.8rem;">
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
                     <button class="btn-primary btn-success" on:click=on_publish 
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
            Some(_) => {
                let mounted = create_signal(false);
                
                create_effect(move |_| {
                    if !mounted.0.get() {
                        // Use requestAnimationFrame to ensure DOM is fully rendered
                        if let Some(window) = web_sys::window() {
                            let callback = wasm_bindgen::closure::Closure::once(move || {
                                setup_resize_divider();
                                mounted.1.set(true);
                            });
                            window.request_animation_frame(callback.as_ref().unchecked_ref()).ok();
                            callback.forget();
                        }
                    }
                });
                
                view! {
                    <div class="workspace">
                        <div class="pane" style="width: 50%">
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
                        <div class="resize-divider"></div>
                        <div class="pane" style="width: 50%">
                             <textarea class="editor-textarea"
                                spellcheck="false"
                                on:input=move |ev| set_markdown.set(event_target_value(&ev))
                             >{markdown}</textarea>
                        </div>
                    </div>
                }.into_view()
            },
            None => view! {
                <div style="display: flex; height: calc(100vh - 60px); justify-content: center; align-items: center; flex-direction: column; gap: 20px;">
                    <h2 style="color: var(--text-main);">"Welcome to TryCli Studio"</h2>
                    <p style="color: var(--text-muted);">"Please sign in to start creating interactive demos."</p>
                </div>
            }.into_view()
        }}
    }
}
