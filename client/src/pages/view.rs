use crate::api::api_base;
use crate::components::limit::LimitReached;
use crate::components::modal::EmbedModal;
use crate::components::navbar::Navbar;
use crate::components::terminal::TerminalView;
use crate::types::User;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::*;
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::RequestCredentials;

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

        let handle_move = {
            let is_dragging = is_dragging.clone();
            move |x: f64, y: f64| {
                if !*is_dragging.borrow() {
                    return;
                }

                if let Some(window) = web_sys::window() {
                    if let Some(workspace) = window
                        .document()
                        .and_then(|d| d.query_selector(".workspace").ok().flatten())
                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let width = window
                            .inner_width()
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1024.0);

                        if width <= 768.0 {
                            let workspace_height = workspace.offset_height() as f64;
                            let workspace_top = workspace.offset_top() as f64;
                            let relative_y = y - workspace_top;
                            let percentage = (relative_y / workspace_height * 100.0).max(20.0).min(80.0);

                            if let Ok(panes) = workspace.query_selector_all(".pane") {
                                if panes.length() >= 2 {
                                    if let Some(first_pane) = panes
                                        .get(0)
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        first_pane.style().set_property("flex", "0 1 auto").ok();
                                        first_pane.style().set_property("width", "100%").ok();
                                        first_pane
                                            .style()
                                            .set_property("height", &format!("{}%", percentage))
                                            .ok();
                                    }
                                    if let Some(second_pane) = panes
                                        .get(1)
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        second_pane.style().set_property("flex", "0 1 auto").ok();
                                        second_pane.style().set_property("width", "100%").ok();
                                        second_pane
                                            .style()
                                            .set_property("height", &format!("{}%", 100.0 - percentage))
                                            .ok();
                                    }
                                }
                            }
                        } else {
                            let workspace_width = workspace.offset_width() as f64;
                            let workspace_left = workspace.offset_left() as f64;
                            let relative_x = x - workspace_left;
                            let percentage = (relative_x / workspace_width * 100.0).max(20.0).min(80.0);

                            if let Ok(panes) = workspace.query_selector_all(".pane") {
                                if panes.length() >= 2 {
                                    if let Some(p1) = panes
                                        .get(0)
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        p1.style().set_property("flex", "0 1 auto").ok();
                                        p1.style()
                                            .set_property("width", &format!("{}%", percentage))
                                            .ok();
                                    }
                                    if let Some(p2) = panes
                                        .get(1)
                                        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                                    {
                                        p2.style().set_property("flex", "0 1 auto").ok();
                                        p2.style()
                                            .set_property("width", &format!("{}%", 100.0 - percentage))
                                            .ok();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        let on_mousedown = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                *is_dragging.borrow_mut() = true;
            }) as Box<dyn Fn(web_sys::MouseEvent)>)
        };

        let on_mousemove = {
            let handle_move = handle_move.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
                handle_move(e.client_x() as f64, e.client_y() as f64);
            }) as Box<dyn Fn(web_sys::MouseEvent)>)
        };

        let on_touchmove = {
            let handle_move = handle_move.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
                e.prevent_default();
                if let Ok(touches) = js_sys::Reflect::get(&e, &JsValue::from_str("touches")) {
                    if let Ok(touch) = js_sys::Reflect::get(&touches, &JsValue::from_f64(0.0)) {
                        let client_x = js_sys::Reflect::get(&touch, &JsValue::from_str("clientX"))
                            .ok()
                            .and_then(|v| v.as_f64());
                        let client_y = js_sys::Reflect::get(&touch, &JsValue::from_str("clientY"))
                            .ok()
                            .and_then(|v| v.as_f64());
                        if let (Some(x), Some(y)) = (client_x, client_y) {
                            handle_move(x, y);
                        }
                    }
                }
            }) as Box<dyn Fn(web_sys::TouchEvent)>)
        };

        let on_mouseup = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
                *is_dragging.borrow_mut() = false;
            }) as Box<dyn Fn(web_sys::MouseEvent)>)
        };

        let on_touchstart = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::TouchEvent| {
                e.prevent_default();
                *is_dragging.borrow_mut() = true;
            }) as Box<dyn Fn(web_sys::TouchEvent)>)
        };

        let on_touchend = {
            let is_dragging = is_dragging.clone();
            wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::TouchEvent| {
                *is_dragging.borrow_mut() = false;
            }) as Box<dyn Fn(web_sys::TouchEvent)>)
        };

        divider
            .add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref())
            .ok();
        on_mousedown.forget();

        divider
            .add_event_listener_with_callback("touchstart", on_touchstart.as_ref().unchecked_ref())
            .ok();
        on_touchstart.forget();

        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document
                .add_event_listener_with_callback(
                    "mousemove",
                    on_mousemove.as_ref().unchecked_ref(),
                )
                .ok();
            document
                .add_event_listener_with_callback("mouseup", on_mouseup.as_ref().unchecked_ref())
                .ok();
            document
                .add_event_listener_with_callback(
                    "touchmove",
                    on_touchmove.as_ref().unchecked_ref(),
                )
                .ok();
            document
                .add_event_listener_with_callback(
                    "touchend",
                    on_touchend.as_ref().unchecked_ref(),
                )
                .ok();
            on_mousemove.forget();
            on_mouseup.forget();
            on_touchmove.forget();
            on_touchend.forget();
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
    let (menu_open, set_menu_open) = create_signal(false);

    let (whitelist, set_whitelist) = create_signal(Vec::<String>::new());

    // Auth Resource
    let auth_resource = create_resource(
        || (),
        move |_| async move {
            let req = Request::get(&format!("{}/api/me", api_base()))
                .credentials(RequestCredentials::Include)
                .send()
                .await;

            if let Ok(resp) = req {
                if resp.ok() {
                    if let Ok(u) = resp.json::<User>().await {
                        set_user.set(Some(u));
                    }
                }
            }
        },
    );

    // Project Data Resource (with VIP key and Referer security)
    let project_resource = create_resource(
        move || (username(), slug(), auth_resource.get(), query_params.get()),
        move |(u, s, _, qp)| async move {
            let key = qp
                .get("key")
                .cloned()
                .unwrap_or_default();
            let url = if key.is_empty() {
                format!("{}/api/project/{}/{}", api_base(), u, s)
            } else {
                format!("{}/api/project/{}/{}?key={}", api_base(), u, s, key)
            };

            let req = Request::get(&url)
                .credentials(RequestCredentials::Include)
                .send()
                .await;

            match req {
                Ok(resp) => {
                    if resp.status() == 403 {
                        ProjectState::Unauthorized
                    } else if resp.status() == 429 {
                        ProjectState::LimitReached
                    } else if resp.ok() {
                        resp.json::<serde_json::Value>()
                            .await
                            .map(ProjectState::Ready)
                            .unwrap_or(ProjectState::NotFound)
                    } else {
                        ProjectState::NotFound
                    }
                }
                Err(_) => ProjectState::NotFound,
            }
        },
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
        },
    );

    let is_owner = move || {
        let current_user = user.get();
        if let Some(ProjectState::Ready(p)) = project_resource.get() {
            let project_owner_id = p.get("owner_id").and_then(|id| id.as_i64());
            match current_user {
                Some(u) => Some(u.id) == project_owner_id,
                None => false,
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
                        user.get().map(|u| {
                            let avatar_url = u.avatar_url.clone();
                            let avatar_url_mobile = u.avatar_url.clone();
                            let login = u.login.clone();
                            view! {
                            <div class="desktop-only" style="display: flex; align-items: center; gap: 16px;">
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <img src=avatar_url style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                                    <span style="color: var(--text-main); font-weight: 500;">{login}</span>
                                </div>
                                <button class="btn-secondary btn-action btn-success" on:click=move |_| {
                                    let origin = window().location().origin().unwrap_or_else(|_| "http://localhost:8080".to_string());
                                    if let Some(ProjectState::Ready(data)) = project_resource.get() {
                                        let token = data.get("embed_token").and_then(|v| v.as_str()).unwrap_or_default();
                                        
                                        // Public embed uses whitelist + domain check only
                                        let public_url = format!("{}/embed/{}/{}", origin, username(), slug());
                                        let smart_url = format!("{}/e/{}", api_base(), token);

                                        set_iframe_code.set(format!(
                                            "<iframe src=\"{}\" width=\"100%\" height=\"500px\" frameborder=\"0\" allowtransparency=\"true\" loading=\"lazy\" allow=\"clipboard-read; clipboard-write\"></iframe>",
                                            public_url
                                        ));
                                        set_smart_link.set(smart_url);
                                        
                                        // Fetch embed_key from dedicated endpoint to avoid exposure in main response
                                        let origin_clone = origin.clone();
                                        let slug_clone = slug();
                                        let username_clone = username();
                                        spawn_local(async move {
                                            let url = format!("{}/api/project/{}/embed-key", api_base(), slug_clone);
                                            match Request::get(&url).credentials(RequestCredentials::Include).send().await {
                                                Ok(resp) => {
                                                    match resp.json::<serde_json::Value>().await {
                                                        Ok(data) => {
                                                            let key = data.get("embed_key").and_then(|v| v.as_str()).unwrap_or_default();
                                                            let vip = if key.is_empty() {
                                                                String::new()
                                                            } else {
                                                                format!("{}/{}/{}?key={}", origin_clone, username_clone, slug_clone, key)
                                                            };
                                                            set_vip_link.set(vip);
                                                        }
                                                        Err(e) => {
                                                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                                                "Failed to parse embed_key response: {}", e
                                                            )));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    web_sys::console::error_1(&JsValue::from_str(&format!(
                                                        "Failed to fetch embed_key from server: {}", e
                                                    )));
                                                }
                                            }
                                        });
                                        
                                        set_embed_modal_open.set(true);
                                    }
                                }>
                                    "Share / Embed"
                                </button>
                                <a href=format!("{}/auth/logout", api_base()) class="btn-secondary btn-action btn-logout" rel="external" style="text-decoration: none; font-size: 0.9rem;">"Logout"</a>
                            </div>
                            <div class="mobile-only" style="display: flex; align-items: center; gap: 12px;">
                                <button class="btn-secondary btn-action btn-success" on:click=move |_| {
                                    let origin = window().location().origin().unwrap_or_else(|_| "http://localhost:8080".to_string());
                                    if let Some(ProjectState::Ready(data)) = project_resource.get() {
                                        let token = data.get("embed_token").and_then(|v| v.as_str()).unwrap_or_default();
                                        let public_url = format!("{}/embed/{}/{}", origin, username(), slug());
                                        let smart_url = format!("{}/e/{}", api_base(), token);

                                        set_iframe_code.set(format!(
                                            "<iframe src=\"{}\" width=\"100%\" height=\"500px\" frameborder=\"0\" allowtransparency=\"true\" loading=\"lazy\" allow=\"clipboard-read; clipboard-write\"></iframe>",
                                            public_url
                                        ));
                                        set_smart_link.set(smart_url);

                                        let origin_clone = origin.clone();
                                        let slug_clone = slug();
                                        let username_clone = username();
                                        spawn_local(async move {
                                            let url = format!("{}/api/project/{}/embed-key", api_base(), slug_clone);
                                            match Request::get(&url).credentials(RequestCredentials::Include).send().await {
                                                Ok(resp) => {
                                                    match resp.json::<serde_json::Value>().await {
                                                        Ok(data) => {
                                                            let key = data.get("embed_key").and_then(|v| v.as_str()).unwrap_or_default();
                                                            let vip = if key.is_empty() {
                                                                String::new()
                                                            } else {
                                                                format!("{}/{}/{}?key={}", origin_clone, username_clone, slug_clone, key)
                                                            };
                                                            set_vip_link.set(vip);
                                                        }
                                                        Err(e) => {
                                                            web_sys::console::error_1(&JsValue::from_str(&format!(
                                                                "Failed to parse embed_key response: {}", e
                                                            )));
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    web_sys::console::error_1(&JsValue::from_str(&format!(
                                                        "Failed to fetch embed_key from server: {}", e
                                                    )));
                                                }
                                            }
                                        });

                                        set_embed_modal_open.set(true);
                                    }
                                }>
                                    "Share"
                                </button>
                                <img src=avatar_url_mobile style="width: 28px; height: 28px; border-radius: 50%; border: 1px solid var(--border);" />
                                <div style="position: relative;">
                                    <button
                                        class="hamburger-menu view-hamburger"
                                        on:click=move |e: ev::MouseEvent| {
                                            e.stop_propagation();
                                            set_menu_open.set(!menu_open.get());
                                        }
                                        aria-label="Toggle menu"
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <line x1="3" y1="12" x2="21" y2="12"></line>
                                            <line x1="3" y1="6" x2="21" y2="6"></line>
                                            <line x1="3" y1="18" x2="21" y2="18"></line>
                                        </svg>
                                    </button>
                                    {move || if menu_open.get() {
                                        view! {
                                            <div class="dropdown-menu" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                                <a href="/docs" class="dropdown-item" style="text-decoration: none;">
                                                    "Docs"
                                                </a>
                                                <a href="/blogs" class="dropdown-item" style="text-decoration: none;">
                                                    "Blogs"
                                                </a>
                                                <a href="https://twitter.com" target="_blank" class="dropdown-item" style="text-decoration: none;">
                                                    "Twitter"
                                                </a>
                                                <a href="https://ko-fi.com/tryclistudio" target="_blank" class="dropdown-item" style="text-decoration: none;">
                                                    "Support Us"
                                                </a>
                                                <div style="border-top: 1px solid var(--border); margin: 4px 0;"></div>
                                                <a href=format!("{}/auth/logout", api_base()) class="dropdown-item dropdown-item-danger" rel="external" style="text-decoration: none;">"Logout"</a>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! { <></> }.into_view()
                                    }}
                                </div>
                            </div>
                        }
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
                    <div style="color: var(--text-muted); text-align: center; margin-top: 100px;">"Project not found. The link might be broken or the project was deleted."</div>
                   
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