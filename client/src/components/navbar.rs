//! Shared navbar component: logo (octopus_terminal_opt.png) + "TryCLI" on desktop,
//! logo only on mobile; right-side content is passed as children per page.

use leptos::*;
use leptos_router::{A, use_location};

const LOGO_SRC: &str = "/octopus_terminal_opt.png";

#[component]
pub fn Navbar(
    children: Children,
    #[prop(optional)] is_logged_in: Option<bool>,
) -> impl IntoView {
    let location = use_location();
    
    let logo_href = move || {
        let is_logged_in = is_logged_in.unwrap_or(false);
        let current_path = location.pathname.get();
        
        if is_logged_in {
            // Logged in
            if current_path == "/" {
                // On landing page -> refresh (same href)
                "/".to_string()
            } else if current_path == "/dashboard" {
                // On dashboard -> refresh (same href)
                "/dashboard".to_string()
            } else {
                // On any other page -> go to dashboard
                "/dashboard".to_string()
            }
        } else {
            // Logged out -> always go to landing page
            "/".to_string()
        }
    };
    
    view! {
        <nav class="navbar" aria-label="Main Navigation">
            <A href=logo_href class="navbar-brand">
                <img src=LOGO_SRC alt="TryCLI" class="navbar-logo" />
                <span class="navbar-text">"TryCLI"</span>
            </A>
            <div class="navbar-actions">
                {children()}
            </div>
        </nav>
    }
}
