use leptos::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::api::api_base;
use crate::types::User;

#[component]
pub fn ProtectedRoute(children: Children) -> impl IntoView {
    let (user, set_user) = create_signal(None::<User>);
    let (checked, set_checked) = create_signal(false);

    create_resource(|| (), move |_| async move {
        // FIX: Use dynamic API URL
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
                    }
                }
            }
            Err(_) => {}
        }
        set_checked.set(true);
    });

    let children_view = children();

    view! {
        {move || {
            if !checked.get() {
                return view! {
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center;">
                        <div class="spinner"></div>
                    </div>
                }.into_view();
            }

            if user.get().is_some() {
                children_view.clone().into_view()
            } else {
                view! {
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center; flex-direction: column; gap: 20px; background: var(--bg-dark); padding: 32px 40px;">
                        <h2 style="color: var(--text-main);">"Authentication Required"</h2>
                        <p style="color: var(--text-muted);">"Please log in to access this page"</p>
                        <a href=format!("{}/auth/github", api_base()) 
                           class="btn-primary btn-hero" rel="external" style="display: flex; align-items: center; gap: 10px;">
                                <svg height="24" width="24" viewBox="0 0 16 16" fill="currentColor" style="color: black;">
                                    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path>
                                </svg>
                                "Start Building Free"
                        </a> 
                    </div>
                }.into_view()
            }
        }}
    }
}
