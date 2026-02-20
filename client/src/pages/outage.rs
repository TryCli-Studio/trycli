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
        </div>
    }
}