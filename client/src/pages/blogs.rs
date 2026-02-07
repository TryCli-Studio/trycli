use leptos::*;
use leptos_router::A;
use crate::components::navbar::Navbar;

#[component]
pub fn BlogsPage() -> impl IntoView {
    view! {
        <div style="min-height: 100vh; background: var(--bg-dark); display: flex; flex-direction: column;">
            <Navbar>
                <div style="display: flex; gap: 1rem; align-items: center;">
                    <A href="/" class="btn-nav">"Home"</A>
                    <A href="/dashboard" class="btn-primary">"Dashboard"</A>
                </div>
            </Navbar>

            <main style="flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 2rem; text-align: center;">
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