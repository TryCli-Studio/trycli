use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use crate::api::api_base;
use crate::components::terminal::TerminalView;
use crate::components::limit::LimitReached;
use serde::{Serialize, Deserialize};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
enum ProjectState {
    Loading,
    NotFound,
    LimitReached,
    Ready(serde_json::Value),
}

#[component]
pub fn EmbedPage() -> impl IntoView {
    let params = use_params_map();
    let username = move || params.get().get("username").cloned().unwrap_or_default();
    let slug = move || params.get().get("slug").cloned().unwrap_or_default();
    let (started, set_started) = create_signal(false);

    // Resource now returns ProjectState
    let project_data = create_resource(
        move || (started.get(), username(), slug()), 
        |(is_started, u, s)| async move {
            if !is_started { return ProjectState::Loading; } 
            
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

    view! {
        <div class="embed-container" style="width: 100vw; height: 100vh; background: #000; overflow: hidden; position: relative;">
            {move || if !started.get() {
                view! {
                    <div class="embed-overlay" 
                         style="position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; background: rgba(0,0,0,0.8); z-index: 10;">
                        <div style="text-align: center; color: white;">
                            <h3 style="margin-bottom: 1rem; font-family: var(--font-sans);">"TryCli Studio Demo"</h3>
                            <button class="btn-primary" 
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