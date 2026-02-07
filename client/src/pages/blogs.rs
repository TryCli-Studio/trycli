use leptos::*;
use leptos_router::A;
use crate::components::navbar::Navbar;

#[component]
pub fn BlogsPage() -> impl IntoView {
    view! {
         <Navbar>
                <A href="/" class="btn-nav">"Home"</A>
                <A href="/dashboard" class="btn-primary">"Dashboard"</A>
            </Navbar>
            <div class="container mx-auto px-4 py-8">
                <h1 class="text-4xl font-bold mb-6">"TryCLI Blogs"</h1>
                <p class="text-lg mb-4">"Welcome to the TryCLI blog section! Here you'll find the latest news, updates, and insights about TryCLI, as well as tips and tricks for getting the most out of our platform."</p>
                <p class="text-lg mb-4">"Stay tuned for upcoming blog posts where we'll share exciting developments, feature releases, and behind-the-scenes looks at how we're building TryCLI. In the meantime, feel free to explore our documentation and join our community on Twitter and Discord for the latest updates!"</p>
                <div class="mt-6">  
                    <A href="/docs" class="text-blue-500 hover:underline">"Explore Documentation"</A>
                </div>
            </div>
        
    }
}