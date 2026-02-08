use leptos::*;
use leptos_router::*;
use crate::components::protected::ProtectedRoute;
use crate::pages::{home::LandingPage, dashboard::DashboardPage, create::CreatePage, view::ViewPage, embed::EmbedPage, docs::DocsPage, blogs::BlogsPage, analytics::AnalyticsPage};

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

                <Route path="/analytics" view=move || view! {
                    <ProtectedRoute>
                        <AnalyticsPage />
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
