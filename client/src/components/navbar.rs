//! Shared navbar component: logo (octopus_terminal_opt.png) + "TryCLI" on desktop,
//! logo only on mobile; right-side content is passed as children per page.

use leptos::*;
use leptos_router::A;

const LOGO_SRC: &str = "/octopus_terminal_opt.png";

#[component]
pub fn Navbar(children: Children) -> impl IntoView {
    view! {
        <nav class="navbar" aria-label="Main Navigation">
            <A href="/" class="navbar-brand">
                <img src=LOGO_SRC alt="TryCLI" class="navbar-logo" />
                <span class="navbar-text">"TryCLI"</span>
            </A>
            <div class="navbar-actions">
                {children()}
            </div>
        </nav>
    }
}
