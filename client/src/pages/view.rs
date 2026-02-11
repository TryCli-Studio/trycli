use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use wasm_bindgen::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use pulldown_cmark::{Parser, Options, html};
use crate::api::api_base;
use crate::components::terminal::TerminalView;
use crate::components::limit::LimitReached;
use crate::components::navbar::Navbar;
use crate::components::modal::EmbedModal;
use crate::types::User;
use serde::{Serialize, Deserialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
enum ProjectState {
    Loading,
    NotFound,
    LimitReached,
    Unauthorized, // Security block state
    Ready(serde_json::Value),
}

pub fn render_markdown(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser); 
    html_output
}

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
                if !*is_dragging.borrow() { return; }
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
                            if let Some(p1) = panes.get(0).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                                p1.style().set_property("flex", "0 1 auto").ok();
                                p1.style().set_property("width", &format!("{}%", percentage)).ok();
                            }
                            if let Some(p2) = panes.get(1).and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
                                p2.style().set_property("flex", "0 1 auto").ok();
                                p2.style().set_property("width", &format!("{}%", 100.0 - percentage)).ok();
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
pub fn ViewPage() -> impl IntoView {
    let params = use_params_map();
    let query_params = use_query_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
    
    let (user, set_user) = create_signal(None::<User>);
    let (embed_modal_open, set_embed_modal_open) = create_signal(false);
    let (iframe_code, set_iframe_code) = create_signal(String::new());
    let (smart_link, set_smart_link) = create_signal(String::new());
    let (vip_link, set_vip_link) = create_signal(String::new());
    
    let (whitelist, set_whitelist) = create_signal(Vec::<String>::new());

    // Auth Resource
    let auth_resource = create_resource(|| (), move |_| async move {
        let req = Request::get(&format!("{}/api/me", api_base()))
            .credentials(RequestCredentials::Include)
            .send().await;

        if let Ok(resp) = req {
            if resp.ok() {
                 if let Ok(u) = resp.json::<User>().await {
                     set_user.set(Some(u));
                 }
            }
        }
    });

    // Project Data Resource (with VIP key and Referer security)
    let project_resource = create_resource(
        move || (username(), slug(), auth_resource.get()), 
        move |(u, s, _)| async move {
            let key = query_params.get_untracked().get("key").cloned().unwrap_or_default();
            let url = if key.is_empty() {
                format!("{}/api/project/{}/{}", api_base(), u, s)
            } else {
                format!("{}/api/project/{}/{}?key={}", api_base(), u, s, key)
            };

            let req = Request::get(&url).credentials(RequestCredentials::Include).send().await;
            
            match req {
                Ok(resp) => {
                    if resp.status() == 403 {
                        ProjectState::Unauthorized
                    } else if resp.status() == 429 {
                        ProjectState::LimitReached
                    } else if resp.ok() {
                        resp.json::<serde_json::Value>().await
                            .map(ProjectState::Ready)
                            .unwrap_or(ProjectState::NotFound)
                    } else {
                        ProjectState::NotFound
                    }
                },
                Err(_) => ProjectState::NotFound
            }
        }
    );

    // Whitelist Resource for Owners
    let whitelist_resource = create_resource(
        move || (project_resource.get(), user.get(), slug()),
        move |(state, current_user, s)| async move {
            if let Some(ProjectState::Ready(p)) = state {
                // Only fetch whitelist if the current user is the owner
                let project_owner_id = p.get("owner_id").and_then(|id| id.as_i64());
                let is_owner = match current_user {
                    Some(u) => Some(u.id) == project_owner_id,
                    None => false
                };
                
                if is_owner {
                    let url = format!("{}/api/project/{}/whitelist", api_base(), s);
                    if let Ok(resp) = Request::get(&url).credentials(RequestCredentials::Include).send().await {
                        if let Ok(list) = resp.json::<Vec<String>>().await {
                            set_whitelist.set(list);
                        }
                    }
                }
            }
        }
    );

    let is_owner = move || {
        let current_user = user.get();
        if let Some(ProjectState::Ready(p)) = project_resource.get() {
             let project_owner_id = p.get("owner_id").and_then(|id| id.as_i64());
             match current_user {
                 Some(u) => Some(u.id) == project_owner_id,
                 None => false
             }
        } else {
            false
        }
    };

    let add_whitelist_item = create_action(move |url: &String| {
        let url = url.clone();
        let s = slug();
        async move {
            let req = Request::post(&format!("{}/api/project/{}/whitelist", api_base(), s))
                .credentials(RequestCredentials::Include)
                .json(&serde_json::json!({ "allowed_url": url }));

            if let Ok(builder) = req {
                match builder.send().await {
                    Ok(resp) => {
                        if resp.ok() {
                            whitelist_resource.refetch();
                        } else {
                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                "Failed to add URL to whitelist: HTTP {}", resp.status()
                            )));
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to add URL to whitelist: {:?}", e
                        )));
                    }
                }
            }
        }
    });

    let remove_whitelist_item = create_action(move |url: &String| {
        let url = url.clone();
        let s = slug();
        async move {
            let req = Request::delete(&format!("{}/api/project/{}/whitelist", api_base(), s))
                .credentials(RequestCredentials::Include)
                .json(&serde_json::json!({ "allowed_url": url }));

            if let Ok(builder) = req {
                match builder.send().await {
                    Ok(resp) => {
                        if resp.ok() {
                            whitelist_resource.refetch();
                        } else {
                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                "Failed to remove URL from whitelist: HTTP {}", resp.status()
                            )));
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "Failed to remove URL from whitelist: {:?}", e
                        )));
                    }
                }
            }
        }
    });

    view! {
        <>
            <EmbedModal 
                show=embed_modal_open.into() 
                title="Share Project".to_string().into() 
                iframe_code=iframe_code.into() 
                smart_link=smart_link.into()
                vip_link=vip_link.into()
                whitelist=whitelist.into()
                on_add_url=Callback::new(move |url: String| add_whitelist_item.dispatch(url))
                on_remove_url=Callback::new(move |url: String| remove_whitelist_item.dispatch(url))
                on_close=Callback::new(move |_| set_embed_modal_open.set(false)) 
            />
            
            <Navbar>
                <div class="controls">
                    {move || if is_owner() {
                        user.get().map(|u| view! {
                            <div style="display: flex; align-items: center; gap: 16px;">
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <img src=u.avatar_url style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                                    <span style="color: var(--text-main); font-weight: 500;">{u.login}</span>
                                </div>
                                <button class="btn-secondary btn-action btn-success" on:click=move |_| {
                                    let origin = window().location().origin().unwrap_or_else(|_| "http://localhost:8080".to_string());
                                    if let Some(ProjectState::Ready(data)) = project_resource.get() {
                                        let token = data.get("embed_token").and_then(|v| v.as_str()).unwrap_or_default();
                                        let key = data.get("embed_key").and_then(|v| v.as_str()).unwrap_or_default();
                                        
                                        // Public embed uses whitelist + domain check only
                                        let public_url = format!("{}/embed/{}/{}", origin, username(), slug());
                                        let smart_url = format!("{}/e/{}", api_base(), token);
                                        let vip = if key.is_empty() {
                                            String::new()
                                        } else {
                                            format!("{}/{}/{}?key={}", origin, username(), slug(), key)
                                        };

                                        set_iframe_code.set(format!(
                                            "<iframe src=\"{}\" width=\"100%\" height=\"500px\" frameborder=\"0\" allowtransparency=\"true\" loading=\"lazy\" allow=\"clipboard-read; clipboard-write\"></iframe>",
                                            public_url
                                        ));
                                        set_smart_link.set(smart_url);
                                        set_vip_link.set(vip);
                                        set_embed_modal_open.set(true);
                                    }
                                }>
                                    "Share / Embed"
                                </button>
                                <a href=format!("{}/auth/logout", api_base()) class="btn-secondary btn-action btn-logout" rel="external" style="text-decoration: none; font-size: 0.9rem;">"Logout"</a>
                            </div>
                        })
                    } else { None }}
                </div>
            </Navbar>

            {move || match project_resource.get() {
                Some(ProjectState::Ready(data)) => {
                    let cid = data["container_id"].as_str().unwrap_or_default().to_string();
                    let md_raw = data["markdown"].as_str().unwrap_or_default().to_string();
                    let html_output = render_markdown(&md_raw);
                    
                    let (is_mounted, set_mounted) = create_signal(false);
                    
                    create_effect(move |_| {
                        if !is_mounted.get() {
                            set_mounted.set(true);
                            if let Some(window) = web_sys::window() {
                                let callback = wasm_bindgen::closure::Closure::once(move || {
                                    setup_resize_divider();
                                });
                                window.request_animation_frame(callback.as_ref().unchecked_ref()).ok();
                                callback.forget();
                            }
                        }
                    });
                    
                    view! {
                        <div style="display: flex; flex-direction: column; height: calc(100vh - 60px);">
                            <div class="workspace" style="flex: 1;">
                                <div class="pane" style="width: 50%; background: var(--bg-dark); overflow-y: auto;">
                                    <div class="markdown-body" inner_html=html_output />
                                </div>
                                <div class="resize-divider"></div>
                                <div class="pane" style="width: 50%;">
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
                        </div>
                    }.into_view()
                },
                Some(ProjectState::Unauthorized) => view! {
                    <div style="display:flex; flex-direction:column; align-items:center; justify-content:center; height:80vh; text-align:center; padding:40px;">
                        <h2 style="color: #ef4444; font-size: 2rem;">"403: Access Denied"</h2>
                        <p style="color: var(--text-muted); margin-top: 1rem; max-width: 400px;">
                            "This terminal is restricted to authorized websites. Contact the owner to whitelist this domain."
                        </p>
                    </div>
                }.into_view(),
                Some(ProjectState::LimitReached) => view! { <LimitReached /> }.into_view(),
                Some(ProjectState::NotFound) => view! { 
                    <div style="color: var(--text-muted); text-align: center; margin-top: 100px;">"Project not found."</div> 
                }.into_view(),
                _ => view! { 
                    <div style="padding: 50px; text-align: center;">
                         <div class="spinner" style="margin: 0 auto;"></div>
                         <p style="margin-top: 1rem; color: var(--text-muted);">"PREPARING ENVIRONMENT..."</p>
                    </div> 
                }.into_view()
            }}
        </>
    }
}