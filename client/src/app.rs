use leptos::*;
use leptos_router::*;
use crate::components::protected::ProtectedRoute;
use crate::pages::{home::LandingPage, dashboard::DashboardPage, create::CreatePage, view::ViewPage, embed::EmbedPage};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/" view=LandingPage />
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
