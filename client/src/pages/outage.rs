use crate::components::navbar::Navbar;
use leptos::*;
use leptos_router::*;

const GITHUB_REPO_URL: &str = "https://github.com/TryCli-Studio/trycli";

#[component]
pub fn OutagePage() -> impl IntoView {
    view! {
        <div class="landing-container">
            <Navbar>
                <div class="nav-actions">
                    <A href="/" class="btn-nav">"Home"</A>
                    <A href="/docs" class="btn-nav">"Documentation"</A>
                    <a
                        href=GITHUB_REPO_URL
                        target="_blank"
                        rel="noopener noreferrer"
                        class="btn-nav"
                    >
                        "GitHub Repo"
                    </a>
                </div>
            </Navbar>

            <main class="hero-main">
                <div class="hero-content">
                    <img
                        src="/octopus-x.png"
                        alt="TryCLI Mascot"
                        style="width: 160px; height: auto; margin-bottom: 2rem; opacity: 0.9; filter: drop-shadow(0 0 30px rgba(255,255,255,0.15));"
                    />
                    <h1 class="hero-title">
                        "Maintenance Mode"<br />
                        <span class="text-gradient">"We'll be back soon."</span>
                    </h1>
                    <p class="hero-subtitle">
                        "TryCLI is currently under scheduled maintenance. The studio is offline for now, but documentation and project links are still available."
                    </p>
                    <div class="cta-group">
                        <A href="/" class="btn-primary btn-hero">
                            "Back to Homepage"
                        </A>
                        <A href="/docs" class="btn-secondary">
                            "Read Documentation"
                        </A>
                        <a
                            href=GITHUB_REPO_URL
                            target="_blank"
                            rel="noopener noreferrer"
                            class="btn-secondary"
                        >
                            "GitHub Repo"
                        </a>
                    </div>
                </div>
            </main>
        </div>
    }
}
