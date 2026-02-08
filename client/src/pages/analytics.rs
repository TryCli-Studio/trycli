use leptos::*;
use leptos_router::A;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::components::navbar::Navbar;
use crate::api::api_base;
use crate::types::{User, AnalyticsDashboardData};

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let (_user, set_user) = create_signal(None::<User>);
    let (data, set_data) = create_signal(None::<AnalyticsDashboardData>);
    // Add an error state
    let (_error, set_error) = create_signal(None::<String>);
    
    // Auth & Data Fetch
    create_resource(|| (), move |_| async move {
        // 1. Check Auth
        let auth_url = format!("{}/api/me", api_base());
        let auth_resp = Request::get(&auth_url)
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        if let Ok(resp) = auth_resp {
            if resp.ok() {
                if let Ok(u) = resp.json::<User>().await {
                    set_user.set(Some(u));
                    
                    // 2. Fetch Analytics
                    let analytics_url = format!("{}/api/analytics", api_base());
                    match Request::get(&analytics_url)
                        .credentials(RequestCredentials::Include)
                        .send()
                        .await 
                    {
                        Ok(data_resp) => {
                            if data_resp.ok() {
                                match data_resp.json::<AnalyticsDashboardData>().await {
                                    Ok(d) => set_data.set(Some(d)),
                                    Err(e) => set_error.set(Some(format!("Failed to parse data: {}", e))),
                                }
                            } else {
                                set_error.set(Some(format!("API Error: {}", data_resp.status())));
                            }
                        },
                        Err(e) => set_error.set(Some(format!("Network Error: {}", e))),
                    }
                }
            } else {
                 set_error.set(Some("Please log in to view analytics".to_string()));
            }
        }
    });

    let format_uptime = |seconds: u64| {
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    };

    view! {
        <div style="min-height: 100vh; background: var(--bg-dark);">
            <Navbar>
                <div class="navbar-actions">
                    <A href="/dashboard" class="btn-nav">"← Back to Dashboard"</A>
                </div>
            </Navbar>

            <div class="dashboard-section">
                <div class="section-header">
                    <h2>"Project Analytics"</h2>
                    {move || {
                       let d = data.get();
                       view! {
                           <span style="color: var(--text-muted); font-family: var(--font-mono);">
                               {move || match d.clone() {
                                   Some(_val) => format!("Last Updated: Just now"),
                                   None => "Loading...".to_string()
                               }}
                           </span>
                       }
                    }}
                </div>

                {move || match data.get() {
                    Some(stats) => view! {
                        // 1. TOP STATS
                        <div class="stats-grid">
                            <div class="stat-card">
                                <span class="stat-label">"Total Lifetime Views"</span>
                                <span class="stat-value">{stats.total_lifetime_views}</span>
                                <span class="stat-sub">"Across all projects"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Active Viewers Now"</span>
                                <span class="stat-value">{stats.active_sessions.len()}</span>
                                <span class="stat-sub">"Live Containers"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Total Projects"</span>
                                <span class="stat-value">{stats.project_breakdown.len()}</span>
                                <span class="stat-sub">"Published Images"</span>
                            </div>
                        </div>

                        // 2. LIVE SESSIONS
                        <h3 style="margin-bottom: 20px; color: var(--text-main);">"Live Sessions"</h3>
                        <div class="data-table-container">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Project"</th>
                                        <th>"Status"</th>
                                        <th>"Uptime"</th>
                                        <th>"Container ID"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {if stats.active_sessions.is_empty() {
                                        view! {
                                            <tr>
                                                <td colspan="4" style="text-align: center; color: var(--text-muted); padding: 32px;">
                                                    "No active viewers right now."
                                                </td>
                                            </tr>
                                        }.into_view()
                                    } else {
                                        stats.active_sessions.into_iter().map(|session| {
                                            view! {
                                                <tr>
                                                    <td style="font-weight: 600;">{session.slug}</td>
                                                    <td>
                                                        <span class="status-dot active"></span> "Running"
                                                    </td>
                                                    <td>
                                                        <span class="uptime-badge">{format_uptime(session.uptime_seconds)}</span>
                                                    </td>
                                                    <td style="font-family: var(--font-mono); color: var(--text-muted); font-size: 0.85rem;">
                                                        {session.container_name}
                                                    </td>
                                                </tr>
                                            }
                                        }).collect_view()
                                    }}
                                </tbody>
                            </table>
                        </div>

                        // 3. PROJECT PERFORMANCE
                        <h3 style="margin-bottom: 20px; color: var(--text-main);">"Performance by Project"</h3>
                        <div class="data-table-container">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Project Name"</th>
                                        <th>"Total Views"</th>
                                        <th style="width: 40%;">"Engagement"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {stats.project_breakdown.into_iter().map(|proj| {
                                        // Calculate percentage for bar width
                                        let percent = if stats.total_lifetime_views > 0 {
                                            (proj.view_count as f64 / stats.total_lifetime_views as f64) * 100.0
                                        } else {
                                            0.0
                                        };
                                        
                                        view! {
                                            <tr>
                                                <td style="font-weight: 600;">{proj.slug}</td>
                                                <td style="font-family: var(--font-mono);">{proj.view_count}</td>
                                                <td>
                                                    <div style="display: flex; align-items: center; gap: 12px;">
                                                        <div class="progress-bar-bg">
                                                            <div class="progress-bar-fill" style=format!("width: {}%", percent)></div>
                                                        </div>
                                                        <span style="font-size: 0.8rem; color: var(--text-muted); width: 40px; text-align: right;">
                                                            {format!("{:.1}%", percent)}
                                                        </span>
                                                    </div>
                                                </td>
                                            </tr>
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>

                    }.into_view(),
                    None => view! {
                        <div style="display: flex; justify-content: center; padding: 100px;">
                            <div class="spinner"></div>
                        </div>
                    }.into_view()
                }}
            </div>
        </div>
    }
}