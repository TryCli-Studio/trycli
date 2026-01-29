use leptos::*;
use leptos_router::A;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::api::api_base;
use crate::types::User;

#[component]
pub fn LandingPage() -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    let (checked, set_checked) = create_signal(false);

    // --- RESTORED AUTHENTICATION LOGIC ---
    {
        let navigate = navigate.clone();
        create_resource(|| (), move |_| {
            let navigate = navigate.clone();
            async move {
                // Check if user is authenticated
                let url = format!("{}/api/me", api_base());
                let auth_req = Request::get(&url)
                    .credentials(RequestCredentials::Include)
                    .send()
                    .await;

                match auth_req {
                    Ok(resp) => {
                        if resp.ok() {
                            if let Ok(_user) = resp.json::<User>().await {
                                // User is authenticated, redirect to dashboard immediately
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
                // Show spinner while checking auth
                view! {
                    <div class="loading-overlay">
                        <div class="spinner"></div>
                    </div>
                }
            } else {
                // Show Landing Page if not logged in
                view! {
                    <div class="landing-container">
                        // Navigation Bar
                        <nav class="landing-nav">
                            <div class="nav-brand">
                                <span class="logo-icon">>_</span>
                                <span class="logo-text">"TryCli Studio"</span>
                            </div>
                            <div class="nav-actions">
                                <A href="/dashboard" class="btn-nav">
                                    "Login"
                                </A>
                                <A href="/dashboard" class="btn-primary btn-lg">
                                    "Launch Dashboard"
                                </A>
                            </div>
                        </nav>

                        // Hero Section
                        <main class="hero-main">
                            <div class="hero-content">
                                <div class="badge">
                                    "100% Rust • WebAssembly"
                                </div>
                                
                                <h1 class="hero-title">
                                    "Demo your CLI tools"<br />
                                    <span class="text-gradient">"in the browser."</span>
                                </h1>
                                
                                <p class="hero-subtitle">
                                    "Instantly spin up isolated Docker containers. Interact via terminal, "
                                    "edit guides with Markdown, and share your projects with a single link."
                                </p>

                                <div class="cta-group">
                                    <A href="/dashboard" class="btn-primary btn-hero">
                                        "Start Building"
                                        <span class="arrow">"→"</span>
                                    </A>
                                    <a href="https://github.com/joshikarthikey/trycli" target="_blank" class="btn-secondary">
                                        "View Source"
                                    </a>
                                </div>

                                // Visual Terminal Preview (Pure CSS)
                                <div class="terminal-preview">
                                    <div class="terminal-header-preview">
                                        <div class="dot red"></div>
                                        <div class="dot yellow"></div>
                                        <div class="dot green"></div>
                                        <span class="terminal-title-preview">"guest@TryCli Studio:~"</span>
                                    </div>
                                    <div class="terminal-body-preview">
                                        <div class="line">
                                            <span class="prompt">"$"</span> 
                                            <span class="cmd">" TryCli"</span>
                                        </div>
                                        <div class="line output">
                                            <span>"Downloading crates.io..."</span>
                                        </div>
                                        <div class="line output">
                                            <span class="success">"✓ Installed successfully"</span>
                                        </div>
                                        <div class="line">
                                            <span class="prompt">"$"</span> 
                                            <span class="cursor">"_"</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </main>
                    </div>
                }
            }
        }}
    }
}