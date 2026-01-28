use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use pulldown_cmark::{Parser, Options, html};
use crate::api::api_base;
use crate::components::terminal::TerminalView;

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
