use crate::components::hamburger::HamburgerMenu;
use crate::components::navbar::Navbar;
use leptos::*;
use leptos_meta::{Link, Meta, Script, Title};
use leptos_router::A;

const GITHUB_REPO_URL: &str = "https://github.com/TryCli-Studio/trycli";
const SUPPORT_URL: &str = "https://ko-fi.com/V7V21TRPL5";
const TWITTER_URL: &str = "https://x.com/TryCliStudio";

#[component]
pub fn LandingPage() -> impl IntoView {
    let schema_json = r#"{
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "name": "TryCLI Studio",
        "applicationCategory": "DeveloperApplication",
        "operatingSystem": "WebBrowser, WASM",
        "offers": {
            "@type": "Offer",
            "price": "0",
            "priceCurrency": "USD"
        },
        "featureList": "Docker Integration, In-Browser Terminal, Markdown Guides"
    }"#;

    view! {
        <>
            <Title text="TryCLI - The Standard for Interactive Documentation" />
            <Meta
                name="description"
                content="Turn your static documentation into interactive live demos. Zero-config Docker sandboxes for onboarding users to your CLI tools instantly."
            />

            <Link rel="canonical" href="https://trycli.com" />

            <Script type_="application/ld+json">
                {schema_json}
            </Script>

            <Meta property="og:type" content="website" />
            <Meta property="og:title" content="TryCLI - The Standard for Interactive Documentation" />
            <Meta
                property="og:description"
                content="Instantly spin up isolated Docker containers and share your CLI projects with a single link."
            />
            <Meta property="og:url" content="https://trycli.com" />
            <Meta property="og:image" content="https://trycli.com/logo_black.png" />

            <Meta name="twitter:card" content="summary_large_image" />
            <Meta name="twitter:title" content="TryCLI Studio" />
            <Meta
                name="twitter:description"
                content="Host, share, and embed fully interactive CLI demos directly in the browser."
            />
            <Meta name="twitter:image" content="https://trycli.com/logo_black.png" />

            <div class="landing-container">
                <Navbar>
                    <div style="display: flex; align-items: center; gap: 20px; width: 100%;">
                        <A
                            href="/outage"
                            class="btn-secondary btn-action btn-login header-login-button"
                            attr:title="Maintenance mode"
                        >
                            <span class="header-login-label">"Login"</span>
                            <span class="header-maintenance-label">"Maintenance"</span>
                        </A>

                        <HamburgerMenu
                            button_class="hamburger-menu"
                            menu_class="mobile-menu"
                            item_class="menu-item"
                            show_blogs=false
                            support_target_blank=true
                            use_open_class=true
                            close_on_item_click=true
                        />
                    </div>
                </Navbar>

                <main class="hero-main">
                    <div class="hero-content">
                        <div class="badge">"Now supporting Alpine, Debian & Fish Shell"</div>

                        <h1 class="hero-title">
                            "Interactive CLI Demos"<br />
                            <span class="text-gradient">"for the Modern Web"</span>
                        </h1>

                        <p class="hero-subtitle">
                            "The modern way to showcase CLI tools. Spin up instant, sandboxed Linux environments directly in your browser. No downloads, no configuration, just code."
                        </p>

                        <div class="cta-group">
                            <A href="/outage" class="btn-primary btn-hero">
                                "Studio In Maintenance"
                                <span class="arrow">"→"</span>
                            </A>
                            <A href="/docs" class="btn-secondary">
                                "Read Documentation"
                            </A>
                        </div>

                        <div class="terminal-preview" role="log" aria-label="Terminal Preview Demo">
                            <div class="terminal-header-preview" aria-hidden="true">
                                <div class="dot red"></div>
                                <div class="dot yellow"></div>
                                <div class="dot green"></div>
                                <span class="terminal-title-preview">"developer@trycli-studio:~"</span>
                            </div>
                            <div class="terminal-body-preview">
                                <div class="line">
                                    <span class="prompt">"➜"</span>
                                    <span class="cmd">" curl -fsSL https://trycli.com/install.sh | sh"</span>
                                </div>
                                <div class="line output"><span>"→ Initializing environment (Ubuntu 22.04)..."</span></div>
                                <div class="line output"><span>"→ Installing dependencies..."</span></div>
                                <div class="line output"><span class="success">"✔ Environment Ready! Session ID: 9f8a-2b1c"</span></div>
                                <div class="line">
                                    <span class="prompt">"➜"</span>
                                    <span class="cmd">" trycli publish --public"</span>
                                </div>
                                <div class="line output"><span>"Snapshotting container state... Done (1.2s)"</span></div>
                                <div class="line"><span class="prompt">"➜"</span> <span class="cursor">"_"</span></div>
                            </div>
                        </div>
                    </div>
                </main>

                <section class="section-features demo-video-section">
                    <div class="container-narrow">
                        <div class="video-placeholder-card" aria-labelledby="demo-video-placeholder">
                            <div class="video-placeholder-header">
                                <div>
                                    <p class="video-placeholder-eyebrow">"Demo Video"</p>
                                    <h2 id="demo-video-placeholder">"Watch TryCLI in action"</h2>
                                </div>
                            </div>

                            <iframe
                                class="demo-video-embed"
                                src="https://www.youtube.com/embed/mw_ausmS4vc"
                                title="TryCLI demo video"
                                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                                referrerpolicy="strict-origin-when-cross-origin"
                                allowfullscreen=true
                            ></iframe>
                        </div>
                    </div>
                </section>

                <section class="section-features" style="background: rgba(255,255,255,0.01);">
                    <div class="container-narrow">
                        <h2 class="section-title">"Frictionless Onboarding"</h2>

                        <p class="section-subtitle" style="text-align: left; margin-bottom: 3rem;">
                            "The biggest drop-off in developer adoption happens before the first command is ever run. "
                            "TryCLI bridges the gap between reading about a tool and actually experiencing it."
                        </p>

                        <div style="display: flex; flex-wrap: wrap; gap: 40px; align-items: center;">
                            <div style="flex: 1 1 400px; text-align: left;">
                                <h3 style="font-size: 1.5rem; margin-bottom: 1rem; color: #fff; font-weight: 700;">"Stop Losing Users at 'npm install'"</h3>
                                <p style="color: var(--text-muted); line-height: 1.6; margin-bottom: 1.5rem;">
                                    "The biggest barrier to adoption isn't your API design, it's the setup process. Every step in your 'Getting Started' guide is a chance for a user to bounce."
                                </p>
                                <p style="color: var(--text-muted); line-height: 1.6;">
                                    "TryCLI replaces static code blocks with live, interactive playgrounds. Let developers experience the value of your tool immediately, without polluting their local machine."
                                </p>
                            </div>

                            <div style="flex: 1 1 300px; background: var(--bg-panel); border: 1px solid var(--border); border-radius: 12px; padding: 30px; box-shadow: 0 10px 30px -10px rgba(0,0,0,0.5);">
                                <ul style="list-style: none; padding: 0; margin: 0; font-family: var(--font-mono); font-size: 0.9rem;">
                                    <li style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px; color: #71717a;">
                                        <span style="color: #ef4444;">"✕"</span> "git clone https://github.com/..."
                                    </li>
                                    <li style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px; color: #71717a;">
                                        <span style="color: #ef4444;">"✕"</span> "npm install / cargo build"
                                    </li>
                                    <li style="display: flex; align-items: center; gap: 12px; margin-bottom: 16px; color: #71717a;">
                                        <span style="color: #ef4444;">"✕"</span> "Error: OpenSSL not found"
                                    </li>
                                    <li style="height: 1px; background: rgba(255,255,255,0.1); margin: 20px 0;"></li>
                                    <li style="display: flex; align-items: center; gap: 12px; color: #fff; font-weight: 600;">
                                        <span style="color: #22c55e;">"✓"</span> "Click Link → Start Coding"
                                    </li>
                                </ul>
                            </div>
                        </div>
                    </div>
                </section>

                <section class="section-features">
                    <div class="container-narrow">
                        <h2 class="section-title"><span class="text-gradient">"Engineered for DevTools"</span></h2>
                        <div class="features-grid">
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>
                                </div>
                                <h3>"Multi-Environment Support"</h3>
                                <p>"Choose your base. We support Ubuntu, Alpine, and Debian. Configure your preferred shell (Bash, Zsh, Fish) via our setup wizard."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline></svg>
                                </div>
                                <h3>"Instant Snapshots"</h3>
                                <p>"Configure your environment interactively, then click 'Publish'. We freeze the filesystem state into a lightweight image that loads instantly for your users."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path></svg>
                                </div>
                                <h3>"Split-Pane Studio"</h3>
                                <p>"Write beautiful Markdown documentation on the right while running commands on the left. The perfect interface for tutorials and workshops."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"></path><polyline points="16 6 12 2 8 6"></polyline><line x1="12" y1="2" x2="12" y2="15"></line></svg>
                                </div>
                                <h3>"One-Click Embeds"</h3>
                                <p>"Generate a copy-paste iframe snippet instantly. Embed your live terminal demo directly into your documentation, blog, or landing page."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M23 4v6h-6"></path><path d="M1 20v-6h6"></path><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"></path></svg>
                                </div>
                                <h3>"Pristine Environments"</h3>
                                <p>"Eliminate configuration drift. Every user session initializes from a clean, immutable snapshot, ensuring consistent behavior every single time."</p>
                            </article>
                        </div>
                    </div>
                </section>

                <section class="section-usage" style="border-bottom: none;">
                    <div class="container-narrow">
                        <div class="final-cta">
                            <h2 class="section-title" style="margin-bottom: 1rem;">
                                <span class="text-gradient">"Ready to Ship?"</span>
                            </h2>
                            <p style="font-size: 1.2rem; color: #a1a1aa; max-width: 700px; margin: 0 auto 2rem auto; line-height: 1.6;">
                                "Join the developers using TryCLI to build the next generation of interactive documentation."
                            </p>
                            <div style="margin-top: 2rem;">
                                <A href="/outage" class="btn-primary btn-hero btn-lg">
                                    "Studio In Maintenance"
                                </A>
                            </div>
                        </div>
                    </div>
                </section>

                <footer class="landing-footer">
                    <div class="footer-container">
                        <div class="footer-top">
                            <div class="footer-brand flex flex-row">
                                <img src="/octopus_terminal_opt.png" alt="TryCLI" class="footer-logo" />
                                <span class="brand-name">"TryCLI"</span>
                            </div>
                            <div class="footer-links">
                                <a href=SUPPORT_URL target="_blank" rel="noopener noreferrer">"Support us"</a>
                                <A href="/docs">"Documentation"</A>
                                <a href=GITHUB_REPO_URL target="_blank" rel="noopener noreferrer">"GitHub Repo"</a>
                                <a href=TWITTER_URL target="_blank" rel="noopener noreferrer">"Twitter"</a>
                            </div>
                        </div>
                        <div class="footer-bottom">
                            <span class="copyright">"© 2026 TryCLI Studio. All rights reserved."</span><br />
                            <span class="copyright">"Built with ❤️"</span>
                        </div>
                    </div>
                </footer>
            </div>
        </>
    }
}
