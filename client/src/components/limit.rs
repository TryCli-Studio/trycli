use leptos::*;

#[component]
pub fn LimitReached() -> impl IntoView {
    view! {
        <div class="limit-container" style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; width: 100%; padding: 20px; text-align: center; background: var(--bg-dark);">
            <div style="max-width: 600px; padding: 40px; border: 1px solid var(--border); border-radius: 12px; background: var(--bg-panel); box-shadow: 0 20px 50px -10px rgba(0,0,0,0.5);">
                <h1 style="font-size: 2rem; font-weight: 800; margin-bottom: 1rem; color: var(--text-main);">
                    "Compute Limit Reached"
                </h1>
                <p style="color: var(--text-muted); font-size: 1.1rem; line-height: 1.6; margin-bottom: 2rem;">
                    "This publisher has reached the free tier limit of 5 concurrent viewers."
                    <br/>
                    "Please try again later or contact the owner."
                </p>
                
                <div style="display: flex; flex-direction: column; gap: 12px; align-items: center;">
                    <p style="color: var(--text-main); font-size: 0.9rem; margin-bottom: 0.5rem; font-weight: 600;">
                        "Are you the owner?"
                    </p>
                    <a href="mailto:tryclistudio@gmail.com" 
                       class="btn-primary"
                       style="width: 100%; max-width: 300px; text-decoration: none;">
                        "Request More Compute"
                    </a>
                    <a href="https://buymeacoffee.com" 
                       target="_blank" 
                       rel="noopener noreferrer"
                       class="btn-secondary"
                       style="width: 100%; max-width: 300px; justify-content: center; color: #FFDD00; border-color: #FFDD00;">
                        "☕ Support Us (Buy Me a Coffee)"
                    </a>
                </div>
            </div>
        </div>
    }
}