use crate::api::api_base;
use crate::components::hamburger::HamburgerMenu;
use crate::components::navbar::Navbar;
use crate::types::User;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::A;
use web_sys::RequestCredentials;

#[component]
pub fn BlogsPage() -> impl IntoView {
    let (user, set_user) = create_signal(None::<User>);
    let (auth_checked, set_auth_checked) = create_signal(false);

    create_resource(
        || (),
        move |_| async move {
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
        },
    );

    view! {
        <div style="min-height: 100vh; background: var(--bg-dark); display: flex; flex-direction: column; overflow-x: hidden;">
            <Navbar>
                <div class="nav-actions">
                    // User Profile Picture
                    {move || {
                        if auth_checked.get() {
                            if let Some(u) = user.get() {
                                view! {
                                    <img src=u.avatar_url
                                         style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);"
                                         alt="User Avatar" />
                                }.into_view()
                            } else {
                                view! { <div></div> }.into_view()
                            }
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}

                    <HamburgerMenu
                        button_class="hamburger-menu dashboard-hamburger"
                        menu_class="mobile-menu"
                        item_class="menu-item"
                        show_home=true
                        show_dashboard=true
                        show_blogs=false
                        use_open_class=true
                        close_on_item_click=true
                    />
                </div>
            </Navbar>

            <main style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 2rem; text-align: center; overflow-x: hidden;">
                <div class="badge">"Under Construction"</div>

                <h1 class="hero-title" style="font-size: 3.5rem; margin-bottom: 1.5rem; line-height: 1.1;">
                    "Engineering" <br />
                    <span class="text-gradient">"Insights"</span>
                </h1>

                <p class="hero-subtitle" style="max-width: 500px; margin: 0 auto 2.5rem auto;">
                    "We are currently writing deep dives on how we built TryCLI using Rust, WebAssembly, and Docker. Stay tuned."
                </p>

                <div style="display: flex; gap: 1rem; justify-content: center;">
                    <A href="/" class="btn-secondary">"← Back Home"</A>
                </div>
            </main>
        </div>
    }
}
