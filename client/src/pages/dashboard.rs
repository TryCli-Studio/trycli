use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use wasm_bindgen::JsValue;
use crate::types::{User, ProjectSummary};

#[component]
pub fn DashboardPage() -> impl IntoView {
    let (user, set_user) = create_signal(None::<User>);
    let (projects, set_projects) = create_signal(Vec::<ProjectSummary>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(None::<String>);

    create_resource(|| (), move |_| async move {
        let auth_req = Request::get("http://localhost:3000/api/me")
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        match auth_req {
            Ok(resp) => {
                if resp.ok() {
                    if let Ok(u) = resp.json::<User>().await {
                        web_sys::console::log_1(&JsValue::from_str(&format!("User authenticated: {}", u.login)));
                        set_user.set(Some(u.clone()));

                        let projects_req = Request::get("http://localhost:3000/api/my-projects")
                            .credentials(RequestCredentials::Include)
                            .send()
                            .await;

                        match projects_req {
                            Ok(p_resp) => {
                                web_sys::console::log_1(&JsValue::from_str(&format!("Projects response status: {}", p_resp.status())));
                                if p_resp.ok() {
                                    if let Ok(projs) = p_resp.json::<Vec<ProjectSummary>>().await {
                                        web_sys::console::log_1(&JsValue::from_str(&format!("Fetched {} projects", projs.len())));
                                        set_projects.set(projs);
                                        set_error.set(None);
                                    } else {
                                        web_sys::console::log_1(&JsValue::from_str("Failed to parse projects JSON"));
                                        set_error.set(Some("Failed to parse projects".to_string()));
                                    }
                                } else {
                                    web_sys::console::log_1(&JsValue::from_str("Projects response not ok"));
                                    set_error.set(Some("Failed to fetch projects".to_string()));
                                }
                            }
                            Err(e) => {
                                web_sys::console::log_1(&JsValue::from_str(&format!("Network error: {:?}", e)));
                                set_error.set(Some("Network error".to_string()));
                            }
                        }
                        set_loading.set(false);
                    }
                } else {
                    set_loading.set(false);
                }
            }
            Err(_) => {
                set_loading.set(false);
                set_error.set(Some("Failed to authenticate".to_string()));
            }
        }
    });

    view! {
        <div class="nav">
            <div class="brand">"TryCLI Studio"</div>
            <div class="controls">
                {move || match user.get() {
                    Some(u) => view! {
                        <div style="display: flex; align-items: center; gap: 16px;">
                            <img src=u.avatar_url 
                                 style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                            <span style="color: var(--text-main); font-weight: 500;">{u.login.clone()}</span>
                        </div>
                        <a href="http://localhost:3000/auth/logout" 
                           class="btn-primary" 
                           style="background: #27272a; text-decoration: none; font-size: 0.9rem; border: 1px solid var(--border);">
                            "Logout"
                        </a>
                    }.into_view(),
                    None => view! {
                        <a href="http://localhost:3000/auth/github" class="btn-primary" style="text-decoration: none;">
                            "Login with GitHub"
                        </a>
                    }.into_view()
                }}
            </div>
        </div>

        {move || match (user.get(), loading.get()) {
            (Some(_), false) => view! {
                <div class="dashboard-container">
                    <div class="dashboard-hero">
                        <div class="hero-content">
                            <h1 class="hero-title">"Dream it, Build it."</h1>
                            <p class="hero-subtitle">"Create and manage your interactive CLI projects"</p>
                            <div class="input-hero-wrapper">
                                <input type="text" 
                                       class="input-hero" 
                                       placeholder="Search or create a new project..." />
                            </div>
                        </div>
                    </div>

                    <div class="dashboard-section">
                        <div class="section-header">
                            <h2>"Your Projects"</h2>
                            <A href="/new" class="btn-primary">
                                "+ New Project"
                            </A>
                        </div>

                        {move || match error.get() {
                            Some(err) => view! {
                                <div class="error-state">
                                    <p class="error-message">{err}</p>
                                    <button class="btn-primary" on:click=move |_| {
                                        set_error.set(None);
                                        set_loading.set(true);
                                    }>
                                        "Retry"
                                    </button>
                                </div>
                            }.into_view(),
                            None => {
                                let proj_list = projects.get();
                                if proj_list.is_empty() {
                                    view! {
                                        <div class="empty-state">
                                            <p class="empty-message">"No projects yet. Create your first one!"</p>
                                            <A href="/new" class="btn-primary">
                                                "Create Project"
                                            </A>
                                        </div>
                                    }.into_view()
                                } else {
                                    view! {
                                        <div class="dashboard-grid">
                                            <For
                                                each=move || projects.get()
                                                key=|p| p.slug.clone()
                                                children=move |proj| {
                                                    view! {
                                                        <div class="project-card">
                                                            <div class="card-header">
                                                                <h3 class="card-title">{proj.slug.clone()}</h3>
                                                            </div>
                                                            <div class="card-body">
                                                                <p class="card-meta">"Image: "<code>{proj.image_tag.clone()}</code></p>
                                                            </div>
                                                            <div class="card-footer">
                                                                <A href=format!("/viewer/{}", proj.slug)
                                                                   class="btn-card">
                                                                    "View"
                                                                </A>
                                                            </div>
                                                        </div>
                                                    }
                                                }
                                            />
                                        </div>
                                    }.into_view()
                                }
                            }
                        }}
                    </div>
                </div>
            }.into_view(),
            (None, false) => view! {
                <div style="display: flex; height: calc(100vh - 60px); justify-content: center; align-items: center; flex-direction: column; gap: 20px;">
                    <h2 style="color: var(--text-main);">"Welcome to TryCLI"</h2>
                    <p style="color: var(--text-muted);">"Please sign in to start creating interactive demos."</p>
                </div>
            }.into_view(),
            _ => view! {
                <div style="display: flex; height: calc(100vh - 60px); justify-content: center; align-items: center;">
                    <div class="spinner"></div>
                </div>
            }.into_view()
        }}
    }
}
