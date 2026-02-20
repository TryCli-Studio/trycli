use crate::components::protected::ProtectedRoute;
use crate::pages::{
    admin::AdminPage, analytics::AnalyticsPage, blogs::BlogsPage, create::CreatePage,
    dashboard::DashboardPage, docs::DocsPage, embed::EmbedPage, home::LandingPage,
    policy::PolicyPage, view::ViewPage,notfound::NotFoundPage, outage::OutagePage
};
use leptos::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="/" view=LandingPage />
                <Route path="/docs" view=DocsPage />
                <Route path="/blogs" view=BlogsPage />
                <Route path="/policy" view=PolicyPage />
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

                <Route path="/admin" view=move || view! {
                    <ProtectedRoute>
                        <AdminPage />
                    </ProtectedRoute>
                } />

                <Route path="/:username/:slug" view=ViewPage />
                <Route path="/embed/:username/:slug" view=EmbedPage />
                <Route path="/*any" view=move || view! { <NotFoundPage /> } />
                <Route path="/outage" view=move || view! { <OutagePage /> } />
            </Routes>
        </Router>
    }
}
