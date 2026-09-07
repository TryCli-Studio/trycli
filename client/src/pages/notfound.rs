use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::api::api_base;
use crate::types::User;
use crate::components::navbar::Navbar;

#[component]
pub fn NotFoundPage() -> impl IntoView {
    let (user, set_user) = create_signal(None::<User>);
    let (auth_checked, set_auth_checked) = create_signal(false);

    // Check if the user is logged in
    create_resource(|| (), move |_| async move {
        let url = format!("{}/api/me", api_base());
        if let Ok(resp) = Request::get(&url)
            .credentials(RequestCredentials::Include)
            .send()
            .await
        {
            if resp.ok() {
                if let Ok(u) = resp.json::<User>().await {
                    set_user.set(Some(u));
                }
            }
        }
        set_auth_checked.set(true);
    });

    view! {
        <div class="landing-container">
            <Navbar>
                <div class="nav-actions">
                    <A href="/" class="btn-nav">"Home"</A>
                </div>
            </Navbar>

            <main class="hero-main">
                <div class="hero-content">
                    <img 
                        src="/octopus_question-2.png" 
                        alt="TryCLI Mascot" 
                        style="width: 160px; height: auto; margin-bottom: 2rem; opacity: 0.9; filter: drop-shadow(0 0 30px rgba(255,255,255,0.15));" 
                    />
                    <h1 class="hero-title">
                        "404"<br />
                        <span class="text-gradient">"Page Not Found"</span>
                    </h1>
                    <p class="hero-subtitle">
                        "Oops! It looks like this link is broken or the page has been moved. Our octopus searched the whole filesystem but couldn't find it."
                    </p>

                    <div class="cta-group">
                        {move || {
                            if auth_checked.get() {
                                if user.get().is_some() {
                                    view! {
                                        <A href="/dashboard" class="btn-primary btn-hero">
                                            "Go to Dashboard"
                                            <span class="arrow">"→"</span>
                                        </A>
                                    }.into_view()
                                } else {
                                    view! {
                                        <A href="/" class="btn-primary btn-hero">
                                            "Return to Homepage"
                                            <span class="arrow">"→"</span>
                                        </A>
                                    }.into_view()
                                }
                            } else {
                                // Show a spinner while checking auth status
                                view! { <div class="spinner" style="margin: 0 auto;"></div> }.into_view()
                            }
                        }}
                    </div>
                </div>
            </main>
        </div>
    }
}