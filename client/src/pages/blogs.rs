use leptos::*;
use leptos_router::A;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::components::navbar::Navbar;
use crate::api::api_base;
use crate::types::User;

#[component]
pub fn BlogsPage() -> impl IntoView {
    let (menu_open, set_menu_open) = create_signal(false);
    let (user, set_user) = create_signal(None::<User>);
    let (auth_checked, set_auth_checked) = create_signal(false);

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

                    // Hamburger Menu Button
                    <button
                        class="hamburger-menu dashboard-hamburger"
                        class:open=move || menu_open.get()
                        on:click=move |_| set_menu_open.update(|open| *open = !*open)
                        aria-label="Toggle menu"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="3" y1="12" x2="21" y2="12"></line>
                            <line x1="3" y1="6" x2="21" y2="6"></line>
                            <line x1="3" y1="18" x2="21" y2="18"></line>
                        </svg>
                    </button>

                    // Mobile Menu Dropdown
                    <div class="mobile-menu" class:open=move || menu_open.get()>
                        <A href="/" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Home"</A>
                        <A href="/dashboard" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Dashboard"</A>
                        <A href="/docs" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Docs"</A>
                        <a href="https://twitter.com" target="_blank" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Twitter"</a>
                        <a href="https://ko-fi.com/tryclistudio" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Support Us"</a>
                    </div>
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