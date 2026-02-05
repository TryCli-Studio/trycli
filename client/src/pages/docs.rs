use leptos::*;
use leptos_router::A;

#[component]
pub fn DocsPage() -> impl IntoView {
    view! {
        <div class="docs-container">
            // Navigation (Public)
            <nav class="nav">
                <div class="brand">
                    <A href="/" class="brand-link">"TryCli Studio"</A>
                </div>
                <div class="controls">
                    <A href="/dashboard" class="btn-primary">"Dashboard"</A>
                </div>
            </nav>

            // Main Content
            <main class="docs-content">
                <h1>"Documentation"</h1>
                
                <section>
                    <h2>"Getting Started"</h2>
                    <p>"TryCli Studio allows you to create interactive CLI demos in seconds."</p>
                </section>

                <section>
                    <h2>"1. Create a Project"</h2>
                    <p>"Go to your dashboard and click 'New Project'. You will be dropped into a live terminal environment."</p>
                    <pre><code>"apt-get update && apt-get install my-tool"</code></pre>
                </section>

                <section>
                    <h2>"2. Write the Guide"</h2>
                    <p>"Use the Markdown editor on the right to write instructions. These will be shown to your users alongside the terminal."</p>
                </section>

                <section>
                    <h2>"3. Publish"</h2>
                    <p>"Click 'Publish'. We snapshot your container state and give you a shareable link."</p>
                </section>
            </main>
        </div>
    }
}