use leptos::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::api::api_base;
use crate::types::User;

#[component]
pub fn LandingPage() -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    let (checked, set_checked) = create_signal(false);

    {
        let navigate = navigate.clone();
        create_resource(|| (), move |_| {
            let navigate = navigate.clone();
            async move {
                // FIX: Use dynamic API URL
                let url = format!("{}/api/me", api_base());
                let auth_req = Request::get(&url)
                    .credentials(RequestCredentials::Include)
                    .send()
                    .await;

                match auth_req {
                    Ok(resp) => {
                        if resp.ok() {
                            if let Ok(_user) = resp.json::<User>().await {
                                // User is authenticated, redirect to dashboard
                                navigate("/dashboard", Default::default());
                            }
                        }
                    }
                    Err(_) => {}
                }
                set_checked.set(true);
            }
        });
    }

    view! {
        {move || {
            if !checked.get() {
                return view! {
                    <div style="display: flex; height: 100vh; justify-content: center; align-items: center; background: var(--bg-dark);">
                        <div class="spinner"></div>
                    </div>
                }.into_view();
            }

            view! {
                <div style="display: flex; flex-direction: column; height: 100vh; justify-content: center; align-items: center; gap: 40px; background: var(--bg-dark);">
                    <div style="text-align: center;">
                        <h1 style="font-size: 4rem; font-weight: 800; color: var(--text-main); margin: 0 0 16px 0; letter-spacing: -1px;">
                            "TryCLI Studio"
                        </h1>
                        <p style="font-size: 1.2rem; color: var(--text-muted); margin: 0 0 8px 0;">
                            "Create and share interactive CLI experiences"
                        </p>
                        <p style="font-size: 1rem; color: var(--text-muted); margin: 0;">
                            "Build, demo, and deploy your tools instantly"
                        </p>
                    </div>

                    // FIX: Use dynamic API URL
                    <a href=format!("{}/auth/github", api_base()) 
                       class="btn-primary"
                       style="padding: 14px 32px; font-size: 1.1rem; text-decoration: none;">
                        "Sign in with GitHub"
                    </a>
                </div>
            }.into_view()
        }}
    }
}
