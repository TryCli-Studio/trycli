use leptos::*;
use leptos_router::A;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div class="landing-container">
            // Navigation Bar
            <nav class="landing-nav">
                <div class="nav-brand">
                    <span class="logo-icon">>_</span>
                    <span class="logo-text">"TryCLI"</span>
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
                        <a href="https://github.com/your-repo/trycli" target="_blank" class="btn-secondary">
                            "View Source"
                        </a>
                    </div>

                    // Visual Terminal Preview (Pure CSS)
                    <div class="terminal-preview">
                        <div class="terminal-header-preview">
                            <div class="dot red"></div>
                            <div class="dot yellow"></div>
                            <div class="dot green"></div>
                            <span class="terminal-title-preview">guest@trycli:~</span>
                        </div>
                        <div class="terminal-body-preview">
                            <div class="line">
                                <span class="prompt">$</span> 
                                <span class="cmd">"cargo install trycli"</span>
                            </div>
                            <div class="line output">
                                <span>"Downloading crates.io..."</span>
                            </div>
                            <div class="line output">
                                <span class="success">"✓ Installed successfully"</span>
                            </div>
                            <div class="line">
                                <span class="prompt">$</span> 
                                <span class="cursor">"_"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    }
}