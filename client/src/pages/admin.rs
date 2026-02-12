use crate::api::api_base;
use crate::components::navbar::Navbar;
use crate::types::ProjectSummary;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::use_navigate;
use serde::{Deserialize, Serialize};
use web_sys::RequestCredentials;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    pub image: String,
    pub is_managed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemStats {
    pub total_projects: i64,
    pub total_views: i64,
    pub active_containers: usize,
    pub container_list: Vec<ContainerInfo>,
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let (stats, set_stats) = create_signal(None::<SystemStats>);
    let (projects, set_projects) = create_signal(Vec::<ProjectSummary>::new());
    let (is_authorized, set_is_authorized) = create_signal(false);
    let navigate = use_navigate();

    // 1. Define the Refresh Action
    // Actions are Copy, so we can use 'refresh_action' everywhere without cloning
    let refresh_action = create_action(move |_: &()| {
        // FIX: Clone navigate here so the async block owns this copy
        let navigate = navigate.clone();

        async move {
            let url = format!("{}/api/admin/stats", api_base());
            let req = Request::get(&url)
                .credentials(RequestCredentials::Include)
                .send()
                .await;

            match req {
                Ok(resp) => {
                    if resp.status() == 403 {
                        // Use the cloned navigate
                        navigate("/analytics", Default::default());
                        return;
                    }
                    if resp.ok() {
                        set_is_authorized.set(true);
                        if let Ok(data) = resp.json::<SystemStats>().await {
                            set_stats.set(Some(data));
                        }
                    }
                }
                Err(_) => {}
            }

            let p_url = format!("{}/api/admin/projects", api_base());
            if let Ok(resp) = Request::get(&p_url)
                .credentials(RequestCredentials::Include)
                .send()
                .await
            {
                if let Ok(data) = resp.json::<Vec<ProjectSummary>>().await {
                    set_projects.set(data);
                }
            }
        }
    });

    // 2. Initial Load
    create_effect(move |_| {
        refresh_action.dispatch(());
    });

    // 3. Define Kill Container Action
    let kill_container = create_action(move |id: &String| {
        let id = id.clone();
        async move {
            if !window()
                .confirm_with_message(&format!("Kill container {}?", id))
                .unwrap_or(false)
            {
                return;
            }
            let url = format!("{}/api/admin/container/{}", api_base(), id);
            let _ = Request::delete(&url)
                .credentials(RequestCredentials::Include)
                .send()
                .await;
            refresh_action.dispatch(()); // Trigger refresh
        }
    });

    // 4. Define Delete Project Action
    let delete_project = create_action(move |slug: &String| {
        let slug = slug.clone();
        async move {
            if !window()
                .confirm_with_message(&format!(
                    "DELETE project '{}'?\nThis deletes the DB entry AND Docker image.",
                    slug
                ))
                .unwrap_or(false)
            {
                return;
            }
            let url = format!("{}/api/admin/project/{}", api_base(), slug);
            let _ = Request::delete(&url)
                .credentials(RequestCredentials::Include)
                .send()
                .await;
            refresh_action.dispatch(()); // Trigger refresh
        }
    });

    view! {
        <div style="min-height: 100vh; background: #000; color: #fff;">
            {move || if !is_authorized.get() {
                view! {
                    <div style="height: 100vh; display: flex; align-items: center; justify-content: center;">
                        <div class="spinner"></div>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div>
                        <Navbar>
                            <div style="display:flex; align-items:center; gap: 16px; margin-left: auto;">
                                <span class="badge"
                                      style="border: 1px solid #ef4444; color: #ef4444; margin: 0; padding: 4px 12px; font-size: 0.75rem; letter-spacing: 0.05em;">
                                    "ADMIN MODE"
                                </span>
                                <button class="btn-secondary"
                                        style="font-size: 0.85rem; padding: 6px 12px; height: 32px; display: flex; align-items: center;"
                                        on:click=move |_| refresh_action.dispatch(())>
                                    "Refresh Data"
                                </button>
                            </div>
                        </Navbar>

                        <div class="container-narrow" style="padding-top: 40px;">
                            <h1 style="margin-bottom: 30px;">"System Overview"</h1>

                            {move || match stats.get() {
                                Some(s) => view! {
                                    <div class="stats-grid" style="margin-bottom: 40px;">
                                        <div class="stat-card">
                                            <span class="stat-label">"Projects"</span>
                                            <span class="stat-value">{s.total_projects}</span>
                                        </div>
                                        <div class="stat-card">
                                            <span class="stat-label">"Total Views"</span>
                                            <span class="stat-value">{s.total_views}</span>
                                        </div>
                                        <div class="stat-card">
                                            <span class="stat-label">"Live Viewers"</span>
                                            <span class="stat-value" style="color: #ef4444;">{s.active_containers}</span>
                                        </div>
                                    </div>

                                    <h2 style="margin-bottom: 20px;">"Docker Containers (" {s.container_list.len()} ")"</h2>
                                    <div class="data-table-container">
                                        <table class="data-table">
                                            <thead>
                                                <tr>
                                                    <th>"Name"</th>
                                                    <th>"Image"</th>
                                                    <th>"State"</th>
                                                    <th>"Action"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {if s.container_list.is_empty() {
                                                    view! { <tr><td colspan="4" style="text-align:center; padding: 20px; color: #666;">"No containers found"</td></tr> }.into_view()
                                                } else {
                                                    s.container_list.into_iter().map(|c| {
                                                        let id_clone = c.id.clone();
                                                        view! {
                                                            <tr style=if c.is_managed { "background: rgba(34, 197, 94, 0.05);" } else { "" }>
                                                                <td style="font-family: monospace; font-size: 0.9rem;">
                                                                    {if c.is_managed { "🟢 " } else { "⚪ " }}
                                                                    {c.name}
                                                                </td>
                                                                <td style="font-size: 0.85rem; color: #a1a1aa; max-width: 200px; overflow:hidden; text-overflow:ellipsis;">{c.image}</td>
                                                                <td>
                                                                    <span class="status-dot" class:active={c.state == "running"}></span>
                                                                    {c.state}
                                                                </td>
                                                                <td>
                                                                    <button class="btn-danger" style="font-size: 0.75rem; padding: 4px 8px;"
                                                                        on:click=move |_| kill_container.dispatch(id_clone.clone())>
                                                                        "KILL"
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    }).collect_view()
                                                }}
                                            </tbody>
                                        </table>
                                    </div>
                                }.into_view(),
                                None => view! { <div class="spinner"></div> }.into_view()
                            }}

                            <h2 style="margin: 40px 0 20px 0;">"All Projects (" {move || projects.get().len()} ")"</h2>
                            <div class="data-table-container" style="margin-bottom: 100px;">
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>"Slug"</th>
                                            <th>"Views"</th>
                                            <th>"Image Tag"</th>
                                            <th>"Actions"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || if projects.get().is_empty() {
                                            view! { <tr><td colspan="4" style="text-align:center; padding: 20px; color: #666;">"No projects found."</td></tr> }.into_view()
                                        } else {
                                            projects.get().into_iter().map(|p| {
                                                let slug = p.slug.clone();
                                                let owner = p.owner_username.clone();

                                                view! {
                                                    <tr>
                                                        <td style="font-weight: bold; color: #fff;">{p.slug.clone()}</td>
                                                        <td>{p.view_count}</td>
                                                        <td style="font-family: monospace; color: #666; font-size: 0.85rem;">{p.image_tag}</td>
                                                        <td>
                                                            <div style="display:flex; gap: 8px;">
                                                                <a href=format!("/{}/{}", owner, p.slug)
                                                                   target="_blank"
                                                                   class="btn-secondary"
                                                                   style="font-size: 0.75rem; padding: 4px 8px; text-decoration:none;">
                                                                   "View"
                                                                </a>
                                                                <button class="btn-danger" style="font-size: 0.75rem; padding: 4px 8px;"
                                                                    on:click=move |_| delete_project.dispatch(slug.clone())>
                                                                    "DELETE"
                                                                </button>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            }).collect_view()
                                        }}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                }.into_view()
            }}
        </div>
    }
}
