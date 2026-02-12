use crate::api::api_base;
use crate::components::limit::LimitReached;
use crate::components::terminal::TerminalView;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::*;
use serde::{Deserialize, Serialize};
use web_sys::window;

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
enum ProjectState {
    Loading,
    NotFound,
    LimitReached,
    Ready(serde_json::Value),
    Unauthorized,
}

#[component]
pub fn EmbedPage() -> impl IntoView {
    let params = use_params_map();
    let query_params = use_query_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
    let (started, set_started) = create_signal(false);

    // Resource now returns ProjectState
let project_data = create_resource(
        move || (started.get(), username(), slug()), 
        move |(is_started, u, s)| async move {
            if !is_started { return ProjectState::Loading; } 
            
            // Fix: Include the 'key' parameter if present (for VIP/Secret embeds)
            let key = query_params.get_untracked().get("key").cloned().unwrap_or_default();
            let url = if key.is_empty() {
                format!("{}/api/project/{}/{}", api_base(), u, s)
            } else {
                format!("{}/api/project/{}/{}?key={}", api_base(), u, s, key)
            };

            let parent_referrer = window()
                .and_then(|w| w.document())
                .map(|d| d.referrer())
                .unwrap_or_default();

            let req_builder = if parent_referrer.is_empty() {
                Request::get(&url)
            } else {
                Request::get(&url).header("X-Embed-Referer", &parent_referrer)
            };

            let req = req_builder.send().await;

            match req {
                Ok(resp) => {
                    if resp.status() == 403 {
                        ProjectState::Unauthorized
                    } else if resp.status() == 429 {
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
                }
                Err(_) => ProjectState::NotFound,
            }
        },
    );

    view! {
        <div class="embed-container" style="width: 100vw; height: 100vh; background: #000; overflow: hidden; position: relative;">
            {move || if !started.get() {
                view! {
                    <div class="embed-overlay"
                         style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background: rgba(0,0,0,0.8); z-index: 10;">
                        <div style="text-align: center; color: white;">
                            <h3 style="margin-bottom: 1rem; font-family: var(--font-sans);">"TryCli Studio Demo"</h3>
                            <button class="btn-secondary btn-action"
                                    style="padding: 12px 24px; font-size: 1.1rem;"
                                    on:click=move |_| set_started.set(true)>
                                "▶ Start Terminal"
                            </button>
                            <p style="margin-top: 1rem; color: #666; font-size: 0.8rem;">"Powered by TryCli Studio"</p>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {}.into_view()
            }}

            {move || match project_data.get() {
                Some(ProjectState::Ready(data)) => {
                    let cid = data["container_id"].as_str().unwrap_or_default().to_string();
                    view! { <TerminalView container_id=cid /> }.into_view()
                },
                Some(ProjectState::Unauthorized) => {
        view! {
            <div style="display:flex; flex-direction:column; align-items:center; justify-content:center; height:100vh; background:#000; color:#ef4444; text-align:center; padding:20px;">
                <h3 style="font-family: var(--font-sans);">"403: Unauthorized Embed Location"</h3>
                <p style="color: #666; font-size: 0.9rem; margin-top: 10px;">
                    "This domain is not on the publisher's Guest List."
                </p>
            </div>
        }.into_view()
                },
                Some(ProjectState::LimitReached) => {
                     // Inside embed, we remove the "Start" overlay (already gone via if logic) and show Limit
                     view! { <LimitReached /> }.into_view()
                },
                Some(ProjectState::NotFound) => view! { <div style="color:red; padding:20px;">"Project not found"</div> }.into_view(),
                Some(ProjectState::Loading) | None => {
                    if started.get() {
                        view! {
                            <div style="display:flex; justify-content:center; align-items:center; height:100%;">
                                <div class="spinner"></div>
                            </div>
                        }.into_view()
                    } else {
                        view! {}.into_view()
                    }
                }
            }}
        </div>
    }
}
