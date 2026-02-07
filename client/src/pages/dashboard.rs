use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use std::rc::Rc;
use crate::types::{User, ProjectSummary};
use crate::api::api_base;
use crate::components::navbar::Navbar;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let (user, set_user) = create_signal(None::<User>);
    let (projects, set_projects) = create_signal(Vec::<ProjectSummary>::new());
    let (loading, set_loading) = create_signal(true);
    let (error, set_error) = create_signal(None::<String>);

    create_resource(|| (), move |_| async move {

        let url = format!("{}/api/me", api_base());
        let auth_req = Request::get(&url)
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        match auth_req {
            Ok(resp) => {
                if resp.ok() {
                    if let Ok(u) = resp.json::<User>().await {
                        web_sys::console::log_1(&JsValue::from_str(&format!("User authenticated: {}", u.login)));
                        set_user.set(Some(u.clone()));

                        let proj_url = format!("{}/api/my-projects", api_base());
                        let projects_req = Request::get(&proj_url)
                            .credentials(RequestCredentials::Include)
                            .send()
                            .await;

                        match projects_req {
                            Ok(p_resp) => {
                                if p_resp.ok() {
                                    if let Ok(projs) = p_resp.json::<Vec<ProjectSummary>>().await {
                                        set_projects.set(projs);
                                        set_error.set(None);
                                    } else {
                                        set_error.set(Some("Failed to parse projects".to_string()));
                                    }
                                } else {
                                    set_error.set(Some("Failed to fetch projects".to_string()));
                                }
                            }
                            Err(_) => {
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


    // Search state and debounce logic
    let (search_input, set_search_input) = create_signal(String::new());
    let (search_results, set_search_results) = create_signal(Vec::<ProjectSummary>::new());
    let (show_suggestions, set_show_suggestions) = create_signal(false);
    let (debounce_timer, set_debounce_timer) = create_signal::<Option<i32>>(None);

    let perform_search = move |query: String| {
        if query.is_empty() {
            set_search_results.set(Vec::new());
            set_show_suggestions.set(false);
            return;
        }

        let query_clone = query.clone();
        spawn_local(async move {
            let encoded_query = js_sys::encode_uri_component(&query_clone).to_string();
            let search_url = format!("{}/api/search-projects?q={}", api_base(), encoded_query);
            web_sys::console::log_1(&JsValue::from_str(&format!("Searching for: {}", search_url)));
            match Request::get(&search_url)
                .credentials(RequestCredentials::Include)
                .send()
                .await
            {
                Ok(resp) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!("Search response status: {}", resp.status())));
                    if resp.ok() {
                        if let Ok(results) = resp.json::<Vec<ProjectSummary>>().await {
                            web_sys::console::log_1(&JsValue::from_str(&format!("Found {} results", results.len())));
                            set_search_results.set(results);
                            set_show_suggestions.set(true);
                        } else {
                            web_sys::console::log_1(&JsValue::from_str("Failed to parse search results JSON"));
                        }
                    } else {
                        let err_text = resp.text().await.unwrap_or_default();
                        web_sys::console::log_1(&JsValue::from_str(&format!("Search failed: {}", err_text)));
                    }
                }
                Err(e) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!("Search network error: {:?}", e)));
                }
            }
        });
    };

    let handle_search_input = move |ev: ev::Event| {
        let input_value = event_target_value(&ev);
        set_search_input.set(input_value.clone());
        set_show_suggestions.set(true);

        // Clear previous timer
        if let Some(timer_id) = debounce_timer.get() {
            if let Some(window) = web_sys::window() {
                window.clear_timeout_with_handle(timer_id);
            }
        }

        // Set new debounce timer (300ms)
        if let Some(window) = web_sys::window() {
            let input_clone = input_value.clone();
            let closure = Closure::once(move || {
                perform_search(input_clone);
            });
            if let Ok(timer_id) = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                300
            ) {
                set_debounce_timer.set(Some(timer_id));
                closure.forget();
            }
        }
    };

    view! {
        <Navbar>
            <div class="controls">
                {move || match user.get() {
                    Some(u) => view! {
                        <div style="display: flex; align-items: center; gap: 16px;">
                            <img src=u.avatar_url 
                                 style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                            <span style="color: var(--text-main); font-weight: 500;">{u.login.clone()}</span>
                        </div>
                        <a href=format!("{}/auth/logout", api_base()) 
                            class="btn-primary btn-logout" 
                            rel="external"  
                            style="text-decoration: none; font-size: 0.9rem;">
                            "Logout"
                        </a>
                    }.into_view(),
                    None => view! {
                        <a href=format!("{}/auth/github", api_base()) class="btn-primary" rel="external" style="text-decoration: none;">                            "Login with GitHub"
                        </a>
                    }.into_view()
                }}
            </div>
        </Navbar>

        {
            let user_signal = user.clone();
            move || {
                match (user_signal.get(), loading.get()) {
                    (Some(u), false) => {
                        let user_login_rc = Rc::new(u.login.clone());
                        view! {
                            <div class="dashboard-container">
                                <div class="dashboard-hero">
                                    <div class="hero-content">
                                        <h1 class="hero-title">"Dream it, Build it."</h1>
                                        <p class="hero-subtitle">"Create and manage your interactive CLI projects"</p>
                                        <DashboardSearch 
                                            search_input=search_input
                                            set_search_input=set_search_input
                                            show_suggestions=show_suggestions
                                            set_show_suggestions=set_show_suggestions
                                            search_results=search_results
                                            handle_search_input=handle_search_input
                                            user_login=user_login_rc.clone()
                                        />
                                    </div>
                                </div>

                                <div class="dashboard-section">
                                   <div class="section-header">
                                        <h2>"Your Projects"</h2>
                                        <A href="/new" class="btn-primary">
                                            "+ New Project"
                                        </A>
                                    </div>

                                    <DashboardProjectList
                                        error=error
                                        set_error=set_error
                                        set_loading=set_loading
                                        projects=projects
                                        set_projects=set_projects 
                                        user_login=user_login_rc.clone()
                                    />
                                </div>
                            </div>
                        }.into_view()
                    },
                    (None, false) => view! {
                        <div style="display: flex; height: calc(100vh - 60px); justify-content: center; align-items: center; flex-direction: column; gap: 20px;">
                            <h2 style="color: var(--text-main);">"Welcome to TryCli Studio"</h2>
                            <p style="color: var(--text-muted);">"Please sign in to start creating interactive demos."</p>
                        </div>
                    }.into_view(),
                    _ => view! {
                        <div style="display: flex; height: calc(100vh - 60px); justify-content: center; align-items: center;">
                            <div class="spinner"></div>
                        </div>
                    }.into_view()
                }
            }
        }
    }
}

#[component]
fn DashboardSearch(
    search_input: ReadSignal<String>,
    set_search_input: WriteSignal<String>,
    show_suggestions: ReadSignal<bool>,
    set_show_suggestions: WriteSignal<bool>,
    search_results: ReadSignal<Vec<ProjectSummary>>,
    handle_search_input: impl Fn(ev::Event) + 'static,
    user_login: Rc<String>,
) -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    view! {
        <div class="input-hero-wrapper" style="position: relative;">
            <input type="text" 
                   class="input-hero" 
                   placeholder="Search or create a new project..."
                   value=search_input.get()
                   on:input=handle_search_input
                   on:focus=move |_| {
                       if !search_input.get().is_empty() {
                           set_show_suggestions.set(true);
                       }
                   }
                   on:blur=move |_| {
                       set_timeout(move || {
                           set_show_suggestions.set(false);
                       }, std::time::Duration::from_millis(200));
                   } />
            
            {
                let nav = navigate.clone();
                move || {
                if show_suggestions.get() {
                    let results = search_results.get();
                    let input_val = search_input.get();
                    let user_login_clone = user_login.clone();
                    let navigate_fn = nav.clone();
                    
                    view! {
                        <div style="position: absolute; top: 100%; left: 0; right: 0; background: var(--bg-main); border: 1px solid var(--border); border-top: none; border-radius: 0 0 8px 8px; max-height: 300px; overflow-y: auto; z-index: 10;">
                            {if results.is_empty() && !input_val.is_empty() {
                                let input_val_copy = input_val.clone();
                                let input_val_display = input_val.clone();
                                view! {
                                    <div style="padding: 16px; color: var(--text-muted);">
                                        <p style="margin: 0 0 8px 0;">"No project found."</p>
                                        <button class="btn-primary" 
                                                style="font-size: 0.9rem; padding: 8px 12px; width: 100%; text-align: left;"
                                                on:click=move |_| {
                                            set_show_suggestions.set(false);
                                            set_search_input.set(String::new());
                                            let encoded_name = js_sys::encode_uri_component(&input_val_copy).to_string();
                                            navigate_fn(&format!("/new?name={}", encoded_name), Default::default());
                                        }>
                                            {format!("Create project '{}'?", input_val_display)}
                                        </button>
                                    </div>
                                }.into_view()
                            } else {
                                let login_str = user_login_clone.as_ref().clone();
                                view! {
                                    <For
                                        each=move || results.clone()
                                        key=|p| p.slug.clone()
                                        children=move |proj| {
                                            let login = login_str.clone();
                                            let proj_slug = proj.slug.clone();
                                            let proj_image = proj.image_tag.clone();
                                            view! {
                                                <a href=format!("/{}/{}", login, proj_slug.clone())
                                                   style="display: block; padding: 12px 16px; color: var(--text-main); text-decoration: none; border-bottom: 1px solid var(--bg-dark); cursor: pointer;"
                                                   on:click=move |_| {
                                                       set_show_suggestions.set(false);
                                                       set_search_input.set(String::new());
                                                   }>
                                                    <div style="font-weight: 500;">{proj_slug}</div>
                                                    <div style="font-size: 0.85rem; color: var(--text-muted);">{proj_image}</div>
                                                </a>
                                            }
                                        }
                                    />
                                }.into_view()
                            }}
                        </div>
                    }.into_view()
                } else {
                    view! { <></> }.into_view()
                }
            }
            }
        </div>
    }
}

#[component]
fn DashboardProjectList(
    error: ReadSignal<Option<String>>,
    set_error: WriteSignal<Option<String>>,
    set_loading: WriteSignal<bool>,
    projects: ReadSignal<Vec<ProjectSummary>>,
    set_projects: WriteSignal<Vec<ProjectSummary>>, // < Receive Setter
    user_login: Rc<String>,
) -> impl IntoView {

    // Handler for deletion logic
    let handle_delete = move |slug: String| {
        let prompt_text = format!(
            "⚠️ DESTRUCTIVE ACTION\n\nThis will permanently delete the project '{}' and its Docker image.\n\nPlease type the project name to confirm:", 
            slug
        );

        // FIX: Match against Ok(Some(input)) to handle the Result wrapper
        if let Ok(Some(input)) = window().prompt_with_message(&prompt_text) {
            if input == slug {
                let slug_clone = slug.clone();
                
                spawn_local(async move {
                    let url = format!("{}/api/project/{}", api_base(), slug_clone);
                    
                    let req = Request::delete(&url)
                        .credentials(RequestCredentials::Include)
                        .send()
                        .await;

                    match req {
                        Ok(resp) => {
                            if resp.ok() {
                                set_projects.update(|list| {
                                    list.retain(|p| p.slug != slug_clone);
                                });
                                let _ = window().alert_with_message("Project and Docker image deleted.");
                            } else {
                                let _ = window().alert_with_message("Failed to delete project. Check server logs.");
                            }
                        },
                        Err(_) => {
                            let _ = window().alert_with_message("Network error occurred.");
                        }
                    }
                });
            } else {
                let _ = window().alert_with_message("Project name mismatch. Deletion cancelled.");
            }
        }
    };

    view! {
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
                    let user_login_clone = user_login.clone();
                    view! {
                        <div class="dashboard-grid">
                            <For
                                each=move || projects.get()
                                key=|p| p.slug.clone()
                                children=move |proj| {
                                    let login = user_login_clone.as_ref().clone();
                                    let delete_slug = proj.slug.clone();
                                    
                                    view! {
                                        <div class="project-card">
                                            <div class="card-header">
                                                <h3 class="card-title">{proj.slug.clone()}</h3>
                                            </div>
                                            <div class="card-body">
                                                <p class="card-meta">"Image: "<code>{proj.image_tag.clone()}</code></p>
                                            </div>
                                            <div class="card-footer" style="display: flex;">
                                                <A href=format!("/{}/{}", login, proj.slug)
                                                   class="btn-card">
                                                    "View"
                                                </A>
                                                <button 
                                                    class="btn-danger"
                                                    on:click=move |_| handle_delete(delete_slug.clone())>
                                                    "Delete"
                                                </button>
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
    }
}