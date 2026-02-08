use leptos::*;
use leptos_router::*;
use crate::components::protected::ProtectedRoute;
use crate::pages::{home::LandingPage, dashboard::DashboardPage, create::CreatePage , embed::EmbedPage, docs::DocsPage, view::ViewPage, blogs::BlogsPage};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/" view=LandingPage />
                <Route path="/docs" view=DocsPage />
                <Route path="/blogs" view=BlogsPage />
                <Route path="/dashboard" view=move || view! {
                    <ProtectedRoute>
                        <DashboardPage />
                    </ProtectedRoute>
                } />
                
                
                <Route path="/new" view=move || view! {
                    <ProtectedRoute>
                        <CreatePage />
                    </ProtectedRoute>
                } />
                <Route path="/:username/:slug" view=ViewPage />
                <Route path="/embed/:username/:slug" view=EmbedPage />
            </Routes>
        </Router>
    }
}
