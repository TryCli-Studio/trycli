use leptos::*;
use leptos_router::*;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::api::api_base;
use crate::types::User;
use crate::components::navbar::Navbar;

#[component]
pub fn OutagePage() -> impl IntoView {
    view!{
        <div class = "landing-container">
            <Navbar>
                <div class="nav-actions">
                    <A href="/" class="btn-nav">"Home"</A>
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
                        "Service Outage"<br />
                        <span class="text-gradient">"We'll be back soon!"</span>
                    </h1>
                    <p class="hero-subtitle">
                    "Due to your love we are handling way more traffic than we expected! Our octopus is working hard to get everything back up and running. We appreciate your patience and support."
                    </p>
                </div>
            </main>

            <section class="section-features demo-video-section" style="padding: 2rem 0 6rem 0; text-align: center;">
                <div class="container-narrow" style="max-width: 980px; margin: 0 auto; padding: 0 1.5rem;">
                    <div class="video-placeholder-card" style="background: linear-gradient(180deg, rgba(13, 13, 15, 0.96), rgba(7, 7, 8, 0.98)); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 22px; padding: 1.5rem; text-align: left; box-shadow: 0 28px 80px -45px rgba(0, 0, 0, 0.9);">
                        <div class="video-placeholder-header" style="margin-bottom: 1rem;">
                            <div>
                                <p class="video-placeholder-eyebrow" style="margin: 0 0 0.4rem 0; color: #a1a1aa; font-size: 0.8rem; letter-spacing: 0.14em; text-transform: uppercase;">"Demo Video"</p>
                                <h2 id="demo-video-placeholder" style="margin: 0; font-size: 1.5rem;">"Watch TryCLI in action"</h2>
                            </div>
                        </div>

                        <iframe
                            class="demo-video-embed"
                            src="https://www.youtube.com/embed/mw_ausmS4vc"
                            title="TryCLI demo video"
                            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                            style="width: 100%; aspect-ratio: 16 / 9; border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 18px; background: #000; display: block;"
                            allowfullscreen=true
                        ></iframe>
                    </div>
                </div>
            </section>
        </div>
    }
}