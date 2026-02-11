use leptos::*;
use leptos_router::A;
use gloo_net::http::Request;
use web_sys::RequestCredentials;
use crate::components::navbar::Navbar;
use crate::api::api_base;
use crate::types::{User, AnalyticsDashboardData};

fn format_uptime(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

fn format_duration(seconds: f64) -> String {
    let secs = seconds.round() as u64;
    format_uptime(secs)
}

#[component]
pub fn AnalyticsPage() -> impl IntoView {
    let (_user, set_user) = create_signal(None::<User>);
    let (data, set_data) = create_signal(None::<AnalyticsDashboardData>);
    let (error, set_error) = create_signal(None::<String>);
    
    // Auth & Data Fetch
    let _analytics_resource = create_resource(|| (), move |_| async move {
        // 1. Check Auth
        let auth_url = format!("{}/api/me", api_base());
        let auth_resp = Request::get(&auth_url)
            .credentials(RequestCredentials::Include)
            .send()
            .await;

        let resp = match auth_resp {
            Ok(r) if r.ok() => r,
            Ok(r) => {
                set_error.set(Some(format!("Auth failed: {}", r.status())));
                return;
            }
            Err(e) => {
                set_error.set(Some(format!("Auth error: {}", e)));
                return;
            }
        };

        let user = match resp.json::<User>().await {
            Ok(u) => u,
            Err(_) => {
                set_error.set(Some("Invalid user response".to_string()));
                return;
            }
        };

        set_user.set(Some(user));

        // 2. Fetch Analytics
let analytics_url = format!("{}/api/analytics", api_base());
match Request::get(&analytics_url)
    .credentials(RequestCredentials::Include)
    .send()
    .await 
{
    // Success case: Status is 200-299
    Ok(r) if r.ok() => {
        if let Ok(d) = r.json::<AnalyticsDashboardData>().await {
            set_data.set(Some(d));
        } else {
            set_error.set(Some("Invalid analytics response".to_string()));
        }
    }
    // Fallback case: Status is NOT 200-299 (e.g. 401, 403, 500)
    // This replaces your 'else' block
    Ok(r) => {
        if r.status() == 401 || r.status() == 403 {
            set_error.set(Some("Please log in to view analytics".to_string()));
        } else {
            set_error.set(Some(format!("Analytics failed: {}", r.status())));
        }
    }
    // Network error case
    Err(e) => set_error.set(Some(format!("Analytics error: {}", e))),
}
    });

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

                {move || error.get().map(|e| view! {
                    <div style="padding: 16px; color: #ff6b6b;">{e}</div>
                })}

                {move || match data.get() {
                    Some(stats) => view! {
                        // 1. TOP STATS
                        <div class="stats-grid">
                            <div class="stat-card">
                                <span class="stat-label">"Views (24h)"</span>
                                <span class="stat-value">{stats.views_24h}</span>
                                <span class="stat-sub">"Last 24 hours"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Views (7d)"</span>
                                <span class="stat-value">{stats.views_7d}</span>
                                <span class="stat-sub">"Last 7 days"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Views (30d)"</span>
                                <span class="stat-value">{stats.views_30d}</span>
                                <span class="stat-sub">"Last 30 days"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Total Lifetime Views"</span>
                                <span class="stat-value">{stats.views_lifetime}</span>
                                <span class="stat-sub">"All-time"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Avg Session Duration"</span>
                                <span class="stat-value">{format_duration(stats.avg_session_duration)}</span>
                                <span class="stat-sub">"Across all sessions"</span>
                            </div>
                            <div class="stat-card">
                                <span class="stat-label">"Error Events"</span>
                                <span class="stat-value">{stats.error_count}</span>
                                <span class="stat-sub">"Total errors"</span>
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
                                        <th class="truncate-cell">"Container ID"</th>
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
                                                    <td class="truncate-cell" title={session.container_name.clone()} style="font-family: var(--font-mono); color: var(--text-muted); font-size: 0.85rem;">
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
                                        <th>"Avg Session"</th>
                                        <th>"Errors"</th>
                                        <th style="width: 30%; min-width: 180px;">"Engagement"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {stats.project_breakdown.into_iter().map(|proj| {
                                        // Calculate percentage for bar width
                                        let percent = if stats.views_lifetime > 0 {
                                            (proj.view_count as f64 / stats.views_lifetime as f64) * 100.0
                                        } else {
                                            0.0
                                        };
                                        
                                        view! {
                                            <tr>
                                                <td style="font-weight: 600;">{proj.slug}</td>
                                                <td style="font-family: var(--font-mono);">{proj.view_count}</td>
                                                <td style="font-family: var(--font-mono);">{format_duration(proj.avg_session_duration)}</td>
                                                <td style="font-family: var(--font-mono);">{proj.error_count}</td>
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