use leptos::*;
use leptos_router::A;
use leptos_meta::{Title, Meta, Link, Script};
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::components::navbar::Navbar;
use crate::api::api_base;
use crate::types::User;

#[component]
pub fn ViewPage() -> impl IntoView {
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

    let auth_github_url = move || format!("{}/auth/github", api_base());

    // 2. STRUCTURED DATA (JSON-LD)
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
            // 4. SEO METADATA
            <Title text="TryCLI - Interactive CLI Demos & Embeds" />
            <Meta name="description" content="Host, share, and embed fully interactive CLI demos directly in the browser. Think Replit, but purpose-built for command-line applications." />
            
            <Link rel="canonical" href="https://trycli.com" />

            <Script type_="application/ld+json">
                {schema_json}
            </Script>
            
            <Meta property="og:type" content="website" />
            <Meta property="og:title" content="TryCLI - Interactive CLI Demos & Embeds" />
            <Meta property="og:description" content="Instantly spin up isolated Docker containers and share your CLI projects with a single link." />
            <Meta property="og:url" content="https://trycli.com" />
            
            <Meta name="twitter:card" content="summary_large_image" />
            <Meta name="twitter:title" content="TryCLI - Interactive CLI Demos" />
            <Meta name="twitter:description" content="Host, share, and embed fully interactive CLI demos directly in the browser." />
        
            // MAIN CONTENT
            <div class="landing-container">
                
                <Navbar>
                    <div class="nav-actions">
                        {move || {
                            if auth_checked.get() {
                                if let Some(u) = user.get() {
                                    // LOGGED IN STATE
                                    view! {
                                        <div style="display: flex; align-items: center; gap: 20px;">
                                            <div style="display: flex; align-items: center; gap: 12px;">
                                                <img src=u.avatar_url 
                                                     style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);" 
                                                     alt="User Avatar" />
                                                <span style="color: var(--text-main); font-weight: 500; font-size: 0.95rem;">
                                                    {u.login}
                                                </span>
                                            </div>
                                            <A href="/dashboard" class="btn-primary btn-lg">"Dashboard"</A>
                                        </div>
                                    }.into_view()
                                } else {
                                    // LOGGED OUT STATE
                                    let url = auth_github_url();
                                    view! {
                                        <a href=url class="btn-primary btn-lg" rel="external" style="display: flex; align-items: center; gap: 8px;">
                                            <svg height="20" width="20" viewBox="0 0 16 16" fill="currentColor">
                                                <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path>
                                            </svg>
                                            "Login with GitHub"
                                        </a>
                                    }.into_view()
                                }
                            } else {
                                // Loading Spinner
                                view! { <div class="spinner" style="width: 20px; height: 20px; border-width: 2px;"></div> }.into_view()
                            }
                        }}
                    </div>
                </Navbar>

                // Hero Section
                <main class="hero-main">
                    <div class="hero-content">
                        <div class="badge">"Run Anywhere • Embed Everywhere"</div>
                        
                        <h1 class="hero-title">
                            "Interactive CLI Demos"<br />
                            <span class="text-gradient">"for the Modern Web."</span>
                        </h1>
                        
                        <p class="hero-subtitle">
                            "Host, share, and embed fully interactive CLI demos directly in the browser. "
                            "Think Replit, but purpose-built for command-line applications."
                        </p>

                        <div class="cta-group">
                            {move || {
                                let url = auth_github_url();
                                if auth_checked.get() && user.get().is_some() {
                                    view! {
                                        <A href="/dashboard" class="btn-primary btn-hero">
                                            "Start Building"
                                            <span class="arrow">"→"</span>
                                        </A>
                                    }.into_view()
                                } else {
                                    view! {
                                        <a href=url class="btn-primary btn-hero" rel="external" style="display: flex; align-items: center; gap: 10px;">
                                            <svg height="24" width="24" viewBox="0 0 16 16" fill="currentColor" style="color: black;">
                                                <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path>
                                            </svg>
                                            "Login with GitHub"
                                        </a>
                                    }.into_view()
                                }
                            }}
                            <A href="/docs" class="btn-secondary">
                                "View Docs"
                            </A>
                        </div>

                        // TERMINAL PREVIEW 
                        <div class="terminal-preview" role="log" aria-label="Terminal Preview Demo">
                            <div class="terminal-header-preview" aria-hidden="true">
                                <div class="dot red"></div>
                                <div class="dot yellow"></div>
                                <div class="dot green"></div>
                                <span class="terminal-title-preview">"guest@tryCLI-demo:~"</span>
                            </div>
                            <div class="terminal-body-preview">
                                <div class="line">
                                    <span class="prompt">"$"</span> 
                                    <span class="cmd">" TryCLI embed --target documentation"</span>
                                </div>
                                <div class="line output"><span>"✔ Snapshotting environment state..."</span></div>
                                <div class="line output"><span>"✔ Generating embed code..."</span></div>
                                <div class="line output"><span class="success">"✓ Live Demo Ready: https://trycli.com/e/xyz123"</span></div>
                                <div class="line"><span class="prompt">"$"</span> <span class="cursor">"_"</span></div>
                            </div>
                        </div>
                    </div>
                </main>

                // FEATURES 
                <section class="section-features" style="background: rgba(255,255,255,0.01);">
                    <div class="container-narrow">
                        <h2 class="section-title">"What Is TryCLI?"</h2>
                        <p class="section-subtitle" style="text-align: left; margin-bottom: 2rem;">
                            "TryCLI orchestrates on-demand, isolated Docker environments that run real Linux terminals in the browser. "
                            "Each user gets a fresh Ubuntu sandbox where they can execute commands, explore tools, and follow guided instructions — without installing anything locally."
                        </p>
                        <p class="section-subtitle" style="text-align: left;">
                            "Once published, a terminal session can be shared as a URL or embedded directly into external sites as a fully interactive component."
                        </p>
                    </div>
                </section>

                <section class="section-features">
                    <div class="container-narrow">
                        <h2 class="section-title"><span class="text-gradient">"Key Features"</span></h2>
                        <div class="features-grid">
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>
                                </div>
                                <h3>"Instant Sandboxes"</h3>
                                <p>"Every session launches a fresh, isolated Ubuntu container. No shared state, no conflicts, and automatic teardown."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline></svg>
                                </div>
                                <h3>"Embed Everywhere"</h3>
                                <p>"Snapshot your environment and embed it in docs, blogs, or wikis. Each embed launches a new isolated session per viewer."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path></svg>
                                </div>
                                <h3>"Interactive Guides"</h3>
                                <p>"Split-pane interface pairs a real-time terminal with a GitHub-flavored Markdown editor for step-by-step walkthroughs."</p>
                            </article>
                        </div>
                    </div>
                </section>

                // USE CASES 
                <section class="section-usage">
                    <div class="container-narrow">
                        <h2 class="section-title">"Use Cases"</h2>
                        <p class="section-subtitle">"If it runs in a terminal, it runs — and embeds — on TryCLI."</p>
                        <div class="features-grid">
                            <div class="feature-card" style="border-left: 3px solid #22c55e;">
                                <h3>"Documentation"</h3>
                                <p>"Embed live CLI demos in docs instead of static screenshots."</p>
                            </div>
                            <div class="feature-card" style="border-left: 3px solid #3b82f6;">
                                <h3>"Open Source"</h3>
                                <p>"Showcase tools instantly without forcing users to install dependencies."</p>
                            </div>
                            <div class="feature-card" style="border-left: 3px solid #a855f7;">
                                <h3>"DevRel"</h3>
                                <p>"Create interactive tutorials, workshops, and hands-on content."</p>
                            </div>
                        </div>
                    </div>
                </section>

                // FINAL CTA 
                <section class="section-usage" style="border-bottom: none;">
                    <div class="container-narrow">
                        <div class="final-cta">
                            <h2 class="section-title" style="margin-bottom: 1rem;">
                                <span class="text-gradient">"Why TryCLI?"</span>
                            </h2>
                            <p style="font-size: 1.2rem; color: #a1a1aa; max-width: 700px; margin: 0 auto 2rem auto; line-height: 1.6;">
                                "Most CLI tools fail at the first step: getting users to try them. "
                                "TryCLI removes that barrier by turning CLI tools into embeddable, interactive experiences that run instantly in the browser."
                            </p>
                            <div style="margin-top: 2rem;">
                                {move || {
                                    let url = auth_github_url();
                                    if auth_checked.get() && user.get().is_some() {
                                        view! { <A href="/dashboard" class="btn-primary btn-lg">"Start Building Now"</A> }.into_view()
                                    } else {
                                        view! { 
                                            <a href=url class="btn-primary btn-lg" rel="external" style="display: flex; align-items: center; gap: 10px;">
                                                <svg height="24" width="24" viewBox="0 0 16 16" fill="currentColor" style="color: black;">
                                                    <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"></path>
                                                </svg>
                                                "Login with GitHub"
                                            </a> 
                                        }.into_view()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                </section>

                // FOOTER 
                <footer class="landing-footer">
                    <div class="footer-container">
                        <div class="footer-top">
                            <div class="footer-brand flex flex-row">
                                <img src="/octopus_terminal_opt.png" alt="TryCLI" class="footer-logo" />
                                <span class="brand-name">"TryCLI"</span>
                            </div>
                            <div class="footer-links">
                                <a href="https://ko-fi.com/V7V21TRPL5" target="_blank" rel="noopener noreferrer">"Support us"</a>
                                <a href="/blogs" rel="noopener noreferrer">"Blogs"</a>
                                <a href="/docs" rel="noopener noreferrer">"Documentation"</a>
                                <a href="https://x.com/TryCliStudio" rel="noopener noreferrer">"Twitter"</a>
                            </div>
                        </div>
                        <div class="footer-bottom">
                            <span class="copyright">"© 2026 TryCLI Studio. All rights reserved."</span><br/>
                            <span class="copyright">"Built with ❤️"</span>
                        </div>
                    </div>
                </footer>
            </div>
        </>
    }
}