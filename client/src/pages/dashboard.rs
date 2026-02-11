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
use crate::components::modal::ConfirmModal;

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
                        // user authenticated
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
                                        set_error.set(Some("Failed to parse project list".to_string()));
                                    }
                                } else {
                                    set_error.set(Some("Failed to fetch deployments".to_string()));
                                }
                            }
                            Err(_) => {
                                set_error.set(Some("Network error connecting to API".to_string()));
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
                set_error.set(Some("Authentication check failed".to_string()));
            }
        }
    });

    // Search state and debounce logic
    let (search_input, set_search_input) = create_signal(String::new());
    let (search_results, set_search_results) = create_signal(Vec::<ProjectSummary>::new());
    let (show_suggestions, set_show_suggestions) = create_signal(false);
    let (debounce_timer, set_debounce_timer) = create_signal::<Option<i32>>(None);
    let (menu_open, set_menu_open) = create_signal(false);

    // Close menu when clicking outside
    create_effect(move |_| {
        if menu_open.get() {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                        set_menu_open.set(false);
                    }) as Box<dyn Fn(web_sys::Event)>);
                    
                    let _ = document.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
                    closure.forget();
                }
            }
        }
    });

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
            
            match Request::get(&search_url)
                .credentials(RequestCredentials::Include)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.ok() {
                        if let Ok(results) = resp.json::<Vec<ProjectSummary>>().await {
                            set_search_results.set(results);
                            set_show_suggestions.set(true);
                        }
                    }
                }
                Err(e) => {
                    web_sys::console::log_1(&JsValue::from_str(&format!("Search error: {:?}", e)));
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
        <Navbar is_logged_in=user.get().is_some()>
            <div class="controls">
                {move || match user.get() {
                    Some(u) => view! {
                        <div style="display: flex; align-items: center; gap: 16px;">
                            <img src=u.avatar_url 
                                 style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" />
                            <span class="dashboard-username" style="color: var(--text-main); font-weight: 500;">{u.login.clone()}</span>
                        </div>
                        <div style="position: relative;">
                            <button 
                                class="hamburger-menu dashboard-hamburger"
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
                                        <a href=format!("{}/auth/logout", api_base()) 
                                           class="dropdown-item dropdown-item-danger" 
                                           rel="external"  
                                           style="text-decoration: none;">
                                           "Logout"
                                        </a>
                                    </div>
                                }.into_view()
                            } else {
                                view! { <></> }.into_view()
                            }}
                        </div>
                    }.into_view(),
                    None => view! {
                        <a href=format!("{}/auth/github", api_base()) class="btn-secondary btn-action" rel="external" style="text-decoration: none;">                            "Login with GitHub"
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
                                        <h1 class="hero-title">"Workspace Overview"</h1>
                                        <p class="hero-subtitle">"Manage your interactive sandboxes, monitor deployments, and publish new snapshots."</p>
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
                                        <h2>"Active Deployments"</h2>
                                        
                                        
                                        <div style="display: flex; gap: 12px; align-items: center;">
                                            <A href="/analytics" class="btn-secondary btn-action">
                                                "Analytics"
                                            </A>
                                            
                                            <A href="/new" class="btn-secondary btn-action">
                                                "+ Initialize Environment"
                                            </A>
                                        </div>
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
                            <h2 style="color: var(--text-main);">"TryCli Studio Session Expired"</h2>
                            <p style="color: var(--text-muted);">"Please authenticate via GitHub to access your workspace."</p>
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
    let (active_index, set_active_index) = create_signal(-1i32);
    let navigate_key = navigate.clone();
    let user_login_key = user_login.clone();
    view! {
        <div class="input-hero-wrapper" style="position: relative;">
            <input type="text" 
                   class="input-hero" 
                   placeholder="Search repositories or initialize a new sandbox..."
                   value=search_input.get()
                   on:input=move |ev| {
                       handle_search_input(ev);
                       set_active_index.set(-1);
                   }
                   on:focus=move |_| {
                       if !search_input.get().is_empty() {
                           set_show_suggestions.set(true);
                       }
                   }
                   on:keydown=move |ev: ev::KeyboardEvent| {
                       let key = ev.key();
                       let results = search_results.get();
                       let input_val = search_input.get();
                       let count = if results.is_empty() && !input_val.is_empty() {
                           1
                       } else {
                           results.len()
                       };

                       match key.as_str() {
                           "ArrowDown" => {
                               ev.prevent_default();
                               if count > 0 {
                                   set_show_suggestions.set(true);
                                   let next = (active_index.get() + 1).rem_euclid(count as i32);
                                   set_active_index.set(next);
                               }
                           }
                           "ArrowUp" => {
                               ev.prevent_default();
                               if count > 0 {
                                   set_show_suggestions.set(true);
                                   let next = (active_index.get() - 1).rem_euclid(count as i32);
                                   set_active_index.set(next);
                               }
                           }
                           "Enter" => {
                               if count > 0 {
                                   ev.prevent_default();
                                   if results.is_empty() && !input_val.is_empty() {
                                       let encoded_name = js_sys::encode_uri_component(&input_val).to_string();
                                       set_show_suggestions.set(false);
                                       set_search_input.set(String::new());
                                       navigate_key(&format!("/new?name={}", encoded_name), Default::default());
                                   } else if active_index.get() >= 0 {
                                       if let Some(proj) = results.get(active_index.get() as usize) {
                                           let login = user_login_key.as_ref().clone();
                                           set_show_suggestions.set(false);
                                           set_search_input.set(String::new());
                                           navigate_key(&format!("/{}/{}", login, proj.slug), Default::default());
                                       }
                                   }
                               }
                           }
                           "Escape" => {
                               set_show_suggestions.set(false);
                           }
                           _ => {}
                       }
                   }
                   on:blur=move |_| {
                       set_timeout(move || {
                           set_show_suggestions.set(false);
                       }, std::time::Duration::from_millis(200));
                   } />
            
            {
                let nav = navigate.clone();
                let user_login_list = user_login.clone();
                move || {
                if show_suggestions.get() {
                    let results = search_results.get();
                    let input_val = search_input.get();
                    let user_login_clone = user_login_list.clone();
                    let navigate_fn = nav.clone();
                    
                    view! {
                        <div style="position: absolute; top: 100%; left: 0; right: 0; background: var(--bg-panel); border: 1px solid var(--border); border-top: none; border-radius: 0 0 8px 8px; max-height: 300px; overflow-y: auto; z-index: 1; box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.5);">
                            {if results.is_empty() && !input_val.is_empty() {
                                let input_val_copy = input_val.clone();
                                let input_val_display = input_val.clone();
                                view! {
                                    <div style="padding: 16px; color: var(--text-muted);">
                                        <p style="margin: 0 0 8px 0; font-size: 0.9rem;">"No existing environment found."</p>
                                        <button class="btn-secondary btn-action" 
                                                style=move || {
                                                    let base = "font-size: 0.9rem; padding: 8px 12px; width: 100%; text-align: left;";
                                                    if active_index.get() == 0 {
                                                        format!("{} background: rgba(59, 130, 246, 0.2);", base)
                                                    } else {
                                                        base.to_string()
                                                    }
                                                }
                                                on:click=move |_| {
                                                    set_show_suggestions.set(false);
                                                    set_search_input.set(String::new());
                                                    let encoded_name = js_sys::encode_uri_component(&input_val_copy).to_string();
                                                    navigate_fn(&format!("/new?name={}", encoded_name), Default::default());
                                                }>
                                                {format!("Initialize repository '{}'?", input_val_display)}
                                        </button>
                                    </div>
                                }.into_view()
                            } else {
                                let login_str = user_login_clone.as_ref().clone();
                                let indexed_results: Vec<(usize, ProjectSummary)> = results
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, proj)| (idx, proj.clone()))
                                    .collect();
                                view! {
                                    <For
                                        each=move || indexed_results.clone()
                                        key=|(_, p)| p.slug.clone()
                                        children=move |(idx, proj)| {
                                            let login = login_str.clone();
                                            let proj_slug = proj.slug.clone();
                                            let proj_image = proj.image_tag.clone();
                                            view! {
                                                <a href=format!("/{}/{}", login, proj_slug.clone())
                                                   style=move || {
                                                       let base = "display: block; padding: 12px 16px; color: var(--text-main); text-decoration: none; border-bottom: 1px solid rgba(255, 255, 255, 0.05); cursor: pointer; transition: background 0.2s;";
                                                       if active_index.get() == idx as i32 {
                                                           format!("{} background: rgba(255, 255, 255, 0.05);", base)
                                                       } else {
                                                           base.to_string()
                                                       }
                                                   }
                                                   on:click=move |_| {
                                                       set_show_suggestions.set(false);
                                                       set_search_input.set(String::new());
                                                   }>
                                                    <div style="font-weight: 600; font-size: 0.95rem;">{proj_slug}</div>
                                                    <div style="font-size: 0.8rem; color: var(--text-muted); font-family: var(--font-mono); margin-top: 2px;">{proj_image}</div>
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
    set_projects: WriteSignal<Vec<ProjectSummary>>, 
    user_login: Rc<String>,
) -> impl IntoView {

    let (confirm_open, set_confirm_open) = create_signal(false);
    let (confirm_slug, set_confirm_slug) = create_signal(String::new());
    let (confirm_message, set_confirm_message) = create_signal(String::new());

    // Handler for deletion logic
    let handle_delete = move |slug: String| {
        set_confirm_slug.set(slug.clone());
        set_confirm_message.set(format!(
            "This will permanently delete the snapshot '{}' and cannot be undone.",
            slug
        ));
        set_confirm_open.set(true);
    };

    let on_confirm_delete = {
        let set_projects = set_projects.clone();
        Callback::new(move |_| {
            let slug = confirm_slug.get();
            let slug_clone = slug.clone();
            set_confirm_open.set(false);
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
                        } else {
                        }
                    },
                    Err(_) => {
                    }
                }
            });
        })
    };

    let on_cancel_delete = Callback::new(move |_| {
        set_confirm_open.set(false);
    });

    view! {
        <ConfirmModal
            show=confirm_open.into()
            title="Terminate environment".to_string().into()
            body=confirm_message.into()
            expected_name=confirm_slug.into()
            confirm_label="Terminate".to_string().into()
            cancel_label="Cancel".to_string().into()
            on_confirm=on_confirm_delete
            on_cancel=on_cancel_delete
        />
        {move || match error.get() {
            Some(err) => view! {
                <div class="error-state">
                    <p class="error-message">{err}</p>
                    <button class="btn-secondary btn-action" on:click=move |_| {
                        set_error.set(None);
                        set_loading.set(true);
                    }>
                        "Retry Connection"
                    </button>
                </div>
            }.into_view(),
            None => {
                let proj_list = projects.get();
                if proj_list.is_empty() {
                    view! {
                        <div class="empty-state">
                            <p class="empty-message">"No active environments. Initialize a new sandbox to start building."</p>
                            <A href="/new" class="btn-secondary btn-action">
                                "Initialize Environment"
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
                                                <p class="card-meta">"Base Image: "<code>{proj.image_tag.clone()}</code></p>
                                            </div>
                                            <div class="card-footer" style="display: flex;">
                                                <A href=format!("/{}/{}", login, proj.slug)
                                                   class="btn-card">
                                                    "Launch Viewer"
                                                </A>
                                                <button 
                                                    class="btn-danger"
                                                    on:click=move |_| handle_delete(delete_slug.clone())>
                                                    "Terminate"
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