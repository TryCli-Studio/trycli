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
use crate::types::User;
use serde::{Serialize, Deserialize};

pub fn render_markdown(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser); 
    html_output
}

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

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
enum ProjectState {
    Loading,
    NotFound,
    LimitReached,
    Ready(serde_json::Value),
}

#[component]
pub fn ViewPage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
    let (user, set_user) = create_signal(None::<User>);
    
    // Auth Check
    create_resource(|| (), move |_| async move {
        let auth_req = Request::get(&format!("{}/api/me", api_base()))
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        if let Ok(resp) = auth_req {
            if resp.ok() {
                 if let Ok(u) = resp.json::<User>().await {
                     set_user.set(Some(u));
                 }
            }
        }
    });

    let navigate = leptos_router::use_navigate();
    
    let copy_embed_code = move |u: String, s: String| {
        let origin = window().location().origin().unwrap_or("http://localhost:8080".to_string());
        let code = format!(
            "<iframe src=\"{}/embed/{}/{}\" width=\"100%\" height=\"500px\" frameborder=\"0\" allowtransparency=\"true\" loading=\"lazy\"></iframe>",
            origin, u, s
        );
        let _ = window().navigator().clipboard().write_text(&code);
        let _ = window().alert_with_message("Embed code copied to clipboard!");
    };

    let project_resource = create_resource(
        move || (username(), slug()), 
        |(u, s)| async move {
            let url = format!("{}/api/project/{}/{}", api_base(), u, s);
            let req = Request::get(&url).send().await;
            
            match req {
                Ok(resp) => {
                    if resp.status() == 429 {
                        ProjectState::LimitReached
                    } else if resp.ok() {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            ProjectState::Ready(json)
                        } else {
                            ProjectState::NotFound
                        }
                    } else {
                        ProjectState::NotFound
                    }
                },
                Err(_) => ProjectState::NotFound
            }
        }
    );

    // 3. Ownership Logic
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

    view! {
        <>
            <div class="nav">
                // ... Nav content same as before ...
                 <div class="nav-brand" style="cursor: pointer;" on:click=move |_| {
                    if user.get().is_some() { navigate("/dashboard", Default::default()); } 
                    else { navigate("/", Default::default()); }
                }>
                    <span class="logo-icon">">_"</span>
                    <span>"TryCli Studio"</span>
                </div>
                <div class="controls">
                    // ... User/Login controls same as before ...
                    {move || match user.get() {
                        Some(u) => view! {
                             <div style="display: flex; align-items: center; gap: 16px;">
                                <div style="display: flex; align-items: center; gap: 8px;">
                                    <img src=u.avatar_url style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                                    <span style="color: var(--text-main); font-weight: 500;">{u.login.clone()}</span>
                                </div>
                                <Show when=is_owner fallback=|| ()>
                                    <button class="btn-primary btn-success" on:click=move |_| {
                                        copy_embed_code(username(), slug());
                                    }>
                                        "Share / Embed"
                                    </button>
                                </Show>
                                <a href=format!("{}/auth/logout", api_base()) class="btn-primary btn-logout" rel="external" style="text-decoration: none; font-size: 0.9rem;">"Logout"</a>
                            </div>
                        }.into_view(),
                        None => view! {
                             <a href=format!("{}/auth/github", api_base()) class="btn-primary" style="text-decoration: none;">"Login"</a>
                        }.into_view()
                    }}
                </div>
            </div>

            // MAIN CONTENT SWITCH
            {move || match project_resource.get() {
                Some(ProjectState::Ready(data)) => {
                    let cid = data["container_id"].as_str().unwrap_or_default().to_string();
                    let md_raw = data["markdown"].as_str().unwrap_or_default().to_string();
                    let html_output = render_markdown(&md_raw);
                    
                    // Trigger resize logic
                    let mounted = create_signal(false);
                    create_effect(move |_| {
                        if !mounted.0.get() {
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
                    }.into_view()
                },
                Some(ProjectState::LimitReached) => view! { <LimitReached /> }.into_view(),
                Some(ProjectState::NotFound) => view! { 
                    <div style="color: var(--text-muted); text-align: center; margin-top: 50px;">"Project not found."</div> 
                }.into_view(),
                Some(ProjectState::Loading) | None => view! { 
                    <div style="padding: 50px; text-align: center;">
                         <div class="spinner" style="margin: 0 auto;"></div>
                         <p style="margin-top: 1rem; color: var(--text-muted);">"Loading Environment..."</p>
                    </div> 
                }.into_view()
            }}
        </>
    }
}