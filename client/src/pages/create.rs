use crate::api::api_base;
use crate::components::modal::Modal;
use crate::components::navbar::Navbar;
use crate::components::terminal::TerminalView;
use crate::types::User;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::RequestCredentials;

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
                            if let Some(first_pane) = panes
                                .get(0)
                                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                            {
                                first_pane.style().set_property("flex", "0 1 auto").ok();
                                first_pane
                                    .style()
                                    .set_property("width", &format!("{}%", percentage))
                                    .ok();
                            }
                            if let Some(second_pane) = panes
                                .get(1)
                                .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                            {
                                second_pane.style().set_property("flex", "0 1 auto").ok();
                                second_pane
                                    .style()
                                    .set_property("width", &format!("{}%", 100.0 - percentage))
                                    .ok();
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

        divider
            .add_event_listener_with_callback("mousedown", on_mousedown.as_ref().unchecked_ref())
            .ok();
        on_mousedown.forget();

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
            on_mousemove.forget();
            on_mouseup.forget();
        }
    }
}

#[component]
pub fn CreatePage() -> impl IntoView {
    let query_params = use_query_map();
    let pre_filled_name =
        move || query_params.with(|params| params.get("name").cloned().unwrap_or_default());

    let (container_id, set_container_id) = create_signal("".to_string());
    let (spawn_error, set_spawn_error) = create_signal(None::<String>);
    let (markdown, set_markdown) = create_signal(r#"#  Welcome to Your TryCLI Environment

This interactive workspace is your project's staging area. On the left is your live terminal, and here on the right is your editable documentation panel.

---

##  Environment Configuration

### 1. Select Your Stack
Use the environment settings to choose your preferred **Linux Distribution** and **Shell** (e.g., Bash, Zsh) to ensure your project runs in its native environment.
### 2. Root Access
You are authenticated as the **root user** in this container. You can execute all commands directly; there is no need to use `sudo` for installations or system configurations.

### 3. Setup Your Project
Use the terminal to prepare your demo:
* Clone your repository or pull your source code directly.
* Install necessary dependencies using package managers like `npm`, `pip`, or `cargo`.
* Verify your CLI tool's functionality before publishing.

---

##  Guided Demo & Documentation

This Markdown panel is **fully editable**. You should use this space to write down the specific steps, descriptions, and commands that viewers need to follow to experience a demo of your project.


---

##  Publish & Embed

Once your environment is configured and your guide is written, you can make your project live via the **Publish** action in your dashboard.

After publishing, you can easily distribute your interactive terminal:
* **Direct Sharing:** Share the unique project URL with your community.
* **Embed Anywhere:** Copy the **Embed Code** from the project settings and paste it into any blog (e.g., Hashnode, Dev.to) or documentation site. Your viewers will be able to interact with your CLI directly within your post.

---

*For advanced tips on container optimization, visit the [TryCLI Publisher Guide](https://trycli.com/docs).*"#.to_string());
    let (slug, set_slug) = create_signal(pre_filled_name());
    let (slug_error, set_slug_error) = create_signal(None::<String>);
    let (user, set_user) = create_signal(None::<User>);
    let (is_publishing, set_is_publishing) = create_signal(false);
    let (modal_open, set_modal_open) = create_signal(false);
    let (modal_title, set_modal_title) = create_signal(String::new());
    let (modal_body, set_modal_body) = create_signal(String::new());
    let (modal_success, set_modal_success) = create_signal(false);

    create_resource(
        || (),
        move |_| async move {
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
                                            set_spawn_error.set(None);
                                        } else {
                                            set_spawn_error.set(Some(
                                                "Failed to parse container ID".to_string(),
                                            ));
                                        }
                                    } else {
                                        let status = spawn_resp.status();
                                        let text = spawn_resp.text().await.unwrap_or_default();
                                        set_spawn_error.set(Some(format!(
                                            "Spawn failed ({}): {}",
                                            status, text
                                        )));
                                    }
                                }
                                Err(e) => {
                                    set_spawn_error.set(Some(format!("Network error: {}", e)));
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!("Auth Error: {}", e)))
                }
            }
        },
    );
    let navigate_modal = use_navigate();
    let on_publish = Rc::new(move |_: ev::MouseEvent| {
        // Prevent concurrent publish requests
        if is_publishing.get() {
            return;
        }
        set_is_publishing.set(true);

        spawn_local(async move {
            let mut publish_success = false;
            let body_data = serde_json::json!({
                "container_id": container_id.get_untracked(),
                "slug": slug.get_untracked(),
                "markdown": markdown.get_untracked()
            });

            let body_str = match serde_json::to_string(&body_data) {
                Ok(s) => s,
                Err(_) => {
                    set_is_publishing.set(false);
                    set_modal_title.set("Publish failed".to_string());
                    set_modal_body.set("Failed to serialize request.".to_string());
                    set_modal_success.set(false);
                    set_modal_open.set(true);
                    return;
                }
            };

            let url = format!("{}/api/publish", api_base());
            let req = Request::post(&url)
                .header("Content-Type", "application/json")
                .credentials(RequestCredentials::Include)
                .body(body_str);

            if let Ok(r) = req {
                match r.send().await {
                    Ok(resp) => {
                        if resp.ok() {
                            publish_success = true;
                            set_modal_title.set("Published".to_string());
                            set_modal_body
                                .set("Your project has been published successfully.".to_string());
                        } else {
                            let status = resp.status();
                            let text = resp.text().await.unwrap_or_default();
                            set_modal_title.set("Publish failed".to_string());
                            set_modal_body.set(format!("{}: {}", status, text));
                        }
                    }
                    Err(_) => {
                        set_modal_title.set("Publish failed".to_string());
                        set_modal_body.set("Network error while publishing.".to_string());
                    }
                }
            } else {
                set_modal_title.set("Publish failed".to_string());
                set_modal_body.set("Failed to build request.".to_string());
            }

            set_is_publishing.set(false);
            set_modal_success.set(publish_success);
            set_modal_open.set(true);
        });
    });
    let on_modal_close = {
        let navigate = navigate_modal.clone();
        Callback::new(move |_| {
            set_modal_open.set(false);
            if modal_success.get() {
                navigate("/dashboard", Default::default());
            }
        })
    };
    view! {
        <Modal
            show=modal_open.into()
            title=modal_title.into()
            body=modal_body.into()
            button_label="Close".to_string().into()
            on_close=on_modal_close
        />
        <Navbar>
            <div class="controls">
                {move || {
                    let on_publish = on_publish.clone();
                    match user.get() {
                    Some(u) => view! {
                        <div style="display: flex; align-items: center; margin-right: 20px;">
                            <img src=u.avatar_url
                                 style="width: 24px; height: 24px; border-radius: 50%; margin-right: 8px; border: 1px solid var(--border);" />
                            <span style="color: var(--text-muted); font-size: 0.9rem;">
                                {u.login.clone()}
                            </span>
                        </div>
                        <a href=format!("{}/auth/logout", api_base())
                            class="btn-secondary btn-action btn-logout"
                            rel="external"
                            style="margin-right: 12px; text-decoration: none; font-size: 0.8rem;">
                            "Logout"
                        </a>
                        <span style="color: var(--text-muted); font-size: 0.9rem; margin-right: 8px;">"Project Slug:"</span>
                        <input type="text" class="input-slug"
                               on:input=move |ev| {
                                   let value = event_target_value(&ev);
                                   let value = value.to_lowercase();
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
                        <button class="btn-secondary btn-action btn-success" on:click=move |ev| (on_publish)(ev)
                                prop:disabled=move || container_id.get().is_empty() || slug_error.get().is_some() || is_publishing.get()
                                style=move || if is_publishing.get() { "opacity: 0.6; cursor: not-allowed;" } else { "" }>
                            {move || if is_publishing.get() { "Publishing..." } else { "Publish" }}
                        </button>
                    }.into_view(),
                    None => view! {
                        <a href=format!("{}/auth/github", api_base()) class="btn-secondary btn-action" style="text-decoration: none;">
                            "Login with GitHub"
                        </a>
                    }.into_view()
                    }
                }}
            </div>
        </Navbar>
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
                                {move || {
                                    if let Some(error) = spawn_error.get() {
                                        view! {
                                            <div style="padding: 20px; color: var(--text-main);">
                                                <div style="margin-bottom: 10px; font-weight: bold; color: #ef4444;">"⚠️ Environment Spawn Error"</div>
                                                <div style="color: var(--text-muted);">{error}</div>
                                            </div>
                                        }.into_view()
                                    } else {
                                        match container_id.get().as_str() {
                                            "" => view! { <div style="padding: 20px; color: var(--text-muted);">"Initializing Environment..."</div> }.into_view(),
                                            id => view! { <TerminalView container_id=id.to_string() /> }.into_view()
                                        }
                                    }
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
