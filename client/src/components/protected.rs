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
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center; flex-direction: column; gap: 20px; background: var(--bg-dark);">
                        <h2 style="color: var(--text-main);">"Authentication Required"</h2>
                        <p style="color: var(--text-muted);">"Please log in to access this page"</p>
                        <a href=format!("{}/auth/github", api_base()) 
                           class="btn-secondary btn-action" 
                           rel="external" 
                           style="text-decoration: none;">
                            "Login with GitHub"
                        </a>
                    </div>
                }.into_view()
            }
        }}
    }
}
