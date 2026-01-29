use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use pulldown_cmark::{Parser, Options, html};
use crate::api::api_base;
use crate::components::terminal::TerminalView;
use crate::types::User;

pub fn render_markdown(text: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser); 
    html_output
}

#[component]
pub fn ViewPage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
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
                    }
                }
            }
            Err(_) => {}
        }
    });
    let navigate = leptos_router::use_navigate();
    
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
        <>
            <div class="nav">
                <div class="nav-brand" style="cursor: pointer;" on:click=move |_| {
                    if user.get().is_some() {
                        navigate("/dashboard", Default::default());
                    } else {
                        navigate("/", Default::default());
                    }
                }>
                    <span class="logo-icon">">_"</span>
                    <span>"TryCli Studio"</span>
                </div>
                <div class="controls">
                    {move || match user.get() {
                        Some(u) => view! {
                            <div style="display: flex; align-items: center; gap: 16px;">
                                <img src=u.avatar_url 
                                     style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                                <span style="color: var(--text-main); font-weight: 500;">{u.login.clone()}</span>
                            </div>
                            <button class="btn-primary btn-success" 
                                    style="margin-right: 10px;"
                                    on:click=move |_| {
                                        let u_val = username();
                                        let s_val = slug();
                                        copy_embed_code(u_val, s_val);
                                    }>
                                "Share / Embed"
                            </button>
                            <a href="http://localhost:3000/auth/logout" 
                               class="btn-primary btn-logout" 
                               style="text-decoration: none; font-size: 0.9rem;">
                                "Logout"
                            </a>
                        }.into_view(),
                        None => view! {
                            <button class="btn-primary btn-success" 
                                    style="margin-right: 10px;"
                                    on:click=move |_| {
                                        let u_val = username();
                                        let s_val = slug();
                                        copy_embed_code(u_val, s_val);
                                    }>
                                "Share / Embed"
                            </button>
                        }.into_view()
                    }}
                </div>
            </div>
            {move || match project_data.get() {
                Some(Some(data)) => {
                    let cid = data["container_id"].as_str().unwrap_or_default().to_string();
                    let md_raw = data["markdown"].as_str().unwrap_or_default().to_string();
                    let html_output = render_markdown(&md_raw);
                    
                    view! {
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
        </>
    }
}
