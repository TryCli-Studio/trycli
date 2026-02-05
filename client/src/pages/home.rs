use leptos::*;
use leptos_router::A;
use leptos_meta::{Title, Meta, Link, Script};


#[component]
pub fn LandingPage() -> impl IntoView {
    //  2. STRUCTURED DATA (JSON-LD) 
    // Defines the site as a 'SoftwareApplication' for search engines[cite: 52].
    // This allows Google to display "Free", "Web/WASM", and feature lists in snippets[cite: 53].
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
            //  4. SEO METADATA 
            // Explicit Title and Description for correct indexing[cite: 22].
            <Title text="TryCLI - Interactive CLI Demos & Embeds" />
            <Meta name="description" content="Host, share, and embed fully interactive CLI demos directly in the browser. Think Replit, but purpose-built for command-line applications." />
            
            // Canonical Link (Essential for preventing duplicate content penalties)[cite: 33].
            <Link rel="canonical" href="https://trycli.com" />

            // JSON-LD Script Injection via leptos_meta[cite: 54].
            <Script type_="application/ld+json">
                {schema_json}
            </Script>
            
            // Open Graph & Twitter Cards (Social Sharing)[cite: 45].
            <Meta property="og:type" content="website" />
            <Meta property="og:title" content="TryCLI - Interactive CLI Demos & Embeds" />
            <Meta property="og:description" content="Instantly spin up isolated Docker containers and share your CLI projects with a single link." />
            <Meta property="og:url" content="https://trycli.com" />
            
            <Meta name="twitter:card" content="summary_large_image" />
            <Meta name="twitter:title" content="TryCLI - Interactive CLI Demos" />
            <Meta name="twitter:description" content="Host, share, and embed fully interactive CLI demos directly in the browser." />
        
            //  MAIN CONTENT 
            <div class="landing-container">
                
                // Navigation: Added aria-label for accessibility[cite: 75].
                <nav class="landing-nav" aria-label="Main Navigation">
                    <div class="nav-brand">
                        <span class="logo-icon">">_"</span>
                        <span class="logo-text">"TryCLI"</span>
                    </div>
                    <div class="nav-actions">
                        <A href="/dashboard" class="btn-nav">"Login"</A>
                        <A href="/dashboard" class="btn-primary btn-lg">"Launch Dashboard"</A>
                    </div>
                </nav>

                // Hero Section: Uses semantic <main>[cite: 73].
                <main class="hero-main">
                    <div class="hero-content">
                        <div class="badge">"Run Anywhere • Embed Everywhere"</div>
                        
                        // H1 contains primary keywords[cite: 75].
                        <h1 class="hero-title">
                            "Interactive CLI Demos"<br />
                            <span class="text-gradient">"for the Modern Web."</span>
                        </h1>
                        
                        <p class="hero-subtitle">
                            "Host, share, and embed fully interactive CLI demos directly in the browser. "
                            "Think Replit, but purpose-built for command-line applications."
                        </p>

                        <div class="cta-group">
                            <A href="/dashboard" class="btn-primary btn-hero">
                                "Start Building"
                                <span class="arrow">"→"</span>
                            </A>
                            
                            // Docs button - navigate to documentation page.
                            <A href="/docs" class="btn-secondary">
                                "View Docs"
                            </A>
                        </div>

                        //  5. TERMINAL PREVIEW 
                        // Added role="log" so search engines treat this as code output.
                        <div class="terminal-preview" role="log" aria-label="Terminal Preview Demo">
                            // Hidden decorative elements to reduce screen reader noise[cite: 70].
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

                //  FEATURES 
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
                                <div class="icon-box">"🚀"</div>
                                <h3>"Instant Sandboxes"</h3>
                                <p>"Every session launches a fresh, isolated Ubuntu container. No shared state, no conflicts, and automatic teardown."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">"📸"</div>
                                <h3>"Embed Everywhere"</h3>
                                <p>"Snapshot your environment and embed it in docs, blogs, or wikis. Each embed launches a new isolated session per viewer."</p>
                            </article>
                            <article class="feature-card">
                                <div class="icon-box">"📘"</div>
                                <h3>"Interactive Guides"</h3>
                                <p>"Split-pane interface pairs a real-time terminal with a GitHub-flavored Markdown editor for step-by-step walkthroughs."</p>
                            </article>
                        </div>
                    </div>
                </section>

                //  USE CASES 
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

                //  FINAL CTA 
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
                                <A href="/dashboard" class="btn-primary btn-lg">"Start Building Now"</A>
                            </div>
                        </div>
                    </div>
                </section>

                //  FOOTER 
                <footer class="landing-footer">
                    <div class="footer-container">
                        <div class="footer-top">
                            <div class="footer-brand flex flex-row">
                                <span class="logo-icon">">_"</span>
                                <span class="brand-name">"TryCLI"</span>
                            </div>
                            <div class="footer-links">
                                <a href="https://github.com/TryCli-Studio" target="_blank" rel="noopener noreferrer">"GitHub"</a>
                                <a href="/docs" rel="noopener noreferrer">"Documentation"</a>
                                <a href="https://x.com/TryCliStudio" rel="noopener noreferrer">"Twitter"</a>
                            </div>
                        </div>
                        <div class="footer-bottom">
                            <span class="copyright">"© 2025 TryCLI Studio. All rights reserved."</span><br/>
                            <span class="copyright">"Built with ❤️"</span>
                        </div>
                    </div>
                </footer>
            </div>
        </>
    }
}