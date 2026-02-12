use crate::api::api_base;
use crate::components::navbar::Navbar;
use crate::types::User;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::A;
use web_sys::RequestCredentials;

#[component]
pub fn DocsPage() -> impl IntoView {
    let (sidebar_open, set_sidebar_open) = create_signal(false);
    let (menu_open, set_menu_open) = create_signal(false);
    let (user, set_user) = create_signal(None::<User>);
    let (auth_checked, set_auth_checked) = create_signal(false);

    create_resource(
        || (),
        move |_| async move {
            let url = format!("{}/api/me", api_base());
            if let Ok(resp) = Request::get(&url)
                .credentials(RequestCredentials::Include)
                .send()
                .await
            {
                if resp.ok() {
                    if let Ok(u) = resp.json::<User>().await {
                        set_user.set(Some(u));
                    }
                }
            }
            set_auth_checked.set(true);
        },
    );

    view! {
        <div class="docs-container">
            <Navbar>
                <div class="nav-actions">
                    // User Profile Picture
                    {move || {
                        if auth_checked.get() {
                            if let Some(u) = user.get() {
                                view! {
                                    <img src=u.avatar_url
                                         style="width: 32px; height: 32px; border-radius: 50%; border: 1px solid var(--border);"
                                         alt="User Avatar" />
                                }.into_view()
                            } else {
                                view! { <div></div> }.into_view()
                            }
                        } else {
                            view! { <div></div> }.into_view()
                        }
                    }}

                    // Hamburger Menu Button
                    <button
                        class="hamburger-menu dashboard-hamburger"
                        class:open=move || menu_open.get()
                        on:click=move |_| set_menu_open.update(|open| *open = !*open)
                        aria-label="Toggle menu"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="3" y1="12" x2="21" y2="12"></line>
                            <line x1="3" y1="6" x2="21" y2="6"></line>
                            <line x1="3" y1="18" x2="21" y2="18"></line>
                        </svg>
                    </button>

                    // Mobile Menu Dropdown
                    <div class="mobile-menu" class:open=move || menu_open.get()>
                        <A href="/" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Home"</A>
                        <A href="/dashboard" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Dashboard"</A>
                        <A href="/blogs" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Blogs"</A>
                        <a href="https://twitter.com" target="_blank" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Twitter"</a>
                        <a href="https://ko-fi.com/tryclistudio" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Support Us"</a>
                    </div>
                </div>
            </Navbar>

            <div class="docs-layout">
                // Mobile menu toggle button
                <button
                    class="docs-sidebar-toggle"
                    on:click=move |_| set_sidebar_open.update(|open| *open = !*open)
                    aria-label="Toggle sidebar"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="3" y1="12" x2="21" y2="12"></line>
                        <line x1="3" y1="6" x2="21" y2="6"></line>
                        <line x1="3" y1="18" x2="21" y2="18"></line>
                    </svg>
                    <span>"Contents"</span>
                </button>

                // Sidebar Navigation
                <aside class="docs-sidebar" class:open=move || sidebar_open.get()>
                    <button
                        class="docs-sidebar-close"
                        on:click=move |_| set_sidebar_open.set(false)
                        aria-label="Close sidebar"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                    </button>
                    <div class="docs-toc">
                        <h3 class="toc-title">"Contents"</h3>
                        <ul class="toc-list" on:click=move |_| set_sidebar_open.set(false)>
                            <li><a href="#introduction">"Introduction"</a></li>
                            <li><a href="#getting-started">"Getting Started"</a>
                                <ul>
                                    <li><a href="#authentication">"Authentication"</a></li>
                                    <li><a href="#dashboard">"The Dashboard"</a></li>
                                </ul>
                            </li>
                            <li><a href="#creating-project">"Creating a Project"</a>
                                <ul>
                                    <li><a href="#studio-interface">"Studio Interface"</a></li>
                                    <li><a href="#setup-wizard">"Setup Wizard"</a></li>
                                    <li><a href="#installing-tool">"Installing Your CLI Tool"</a></li>
                                    <li><a href="#writing-guide">"Writing the Guide"</a></li>
                                </ul>
                            </li>
                            <li><a href="#publishing">"Publishing Your Demo"</a></li>
                            <li><a href="#managing">"Managing Your Projects"</a>
                                <ul>
                                    <li><a href="#viewing-projects">"Viewing Your Projects"</a></li>
                                    <li><a href="#deleting-projects">"Deleting Projects"</a></li>
                                </ul>
                            </li>
                            <li><a href="#sharing">"Sharing & Embedding"</a>
                                <ul>
                                    <li><a href="#public-links">"Public Project Links"</a></li>
                                    <li><a href="#embedding">"Embedding on Websites"</a></li>
                                </ul>
                            </li>
                            <li><a href="#security">"Security & Sandbox"</a></li>
                        </ul>
                    </div>
                </aside>

                // Main Documentation Content
                <main class="docs-content">
                    <h1>"TryCli Studio User Documentation"</h1>

                    <section id="introduction">
                        <h2>"1. Introduction"</h2>
                        <p>"TryCli Studio is a platform for developers to build, host, and share interactive Command Line Interface (CLI) demos directly in the browser. It removes the need for users to install dependencies locally by spinning up isolated, ephemeral Docker containers on demand."</p>
                        <p><strong>"Core Philosophy:"</strong>" \"Dream it, Build it.\" We provide a split-pane interface: one side for a live Linux terminal, the other for a rich Markdown guide, allowing you to walk users through your tool step-by-step."</p>
                    </section>

                    <section id="getting-started">
                        <h2>"2. Getting Started"</h2>

                        <h3 id="authentication">"Authentication"</h3>
                        <p>"To create projects, you must be logged in. We use GitHub OAuth for secure and quick authentication."</p>
                        <ol>
                            <li>"Click the \"Login with GitHub\" button on the home page or navigation bar."</li>
                            <li>"Grant TryCli Studio permission to read your public profile (we only store your Username, ID, and Avatar)."</li>
                            <li>"Once authenticated, you will be redirected to your personal Dashboard."</li>
                        </ol>

                        <h3 id="dashboard">"The Dashboard"</h3>
                        <p>"Your Dashboard is the command center for your projects."</p>
                        <ul>
                            <li><strong>"Your Projects:"</strong>" Displays a grid of all the demos you have created. Each card shows the project slug and the base Docker image used."</li>
                            <li><strong>"Search:"</strong>" Use the search bar at the top to filter your projects by name. It supports fuzzy search logic."</li>
                            <li><strong>"New Project:"</strong>" Click the \"+ New Project\" button to enter the Studio."</li>
                        </ul>
                    </section>

                    <section id="creating-project">
                        <h2>"3. Creating a Project"</h2>
                        <p>"The Studio is where the magic happens. It features a responsive split-pane layout."</p>

                        <h3 id="studio-interface">"The Studio Interface"</h3>
                        <ul>
                            <li><strong>"Left Pane (Terminal):"</strong>" A fully functional xterm.js terminal connected to a live container via WebSockets."</li>
                            <li><strong>"Right Pane (Editor):"</strong>" A Markdown editor where you write the tutorial or documentation for your tool."</li>
                            <li><strong>"Resize:"</strong>" You can drag the divider between the panes to adjust their width."</li>
                        </ul>

                        <h3 id="setup-wizard">"The Setup Wizard"</h3>
                        <p>"When you first load the Studio, the terminal will automatically launch the Setup Wizard. You will be prompted to configure your environment using your keyboard:"</p>

                        <p><strong>"Select Base Image:"</strong></p>
                        <ul>
                            <li><strong>"Ubuntu 22.04:"</strong>" Best for general compatibility and heavier tools."</li>
                            <li><strong>"Alpine Linux:"</strong>" Lightweight and fast, ideal for simple binaries."</li>
                            <li><strong>"Debian Bookworm:"</strong>" Stable and widely supported."</li>
                        </ul>

                        <p><strong>"Select Shell:"</strong></p>
                        <ul>
                            <li><strong>"Bash:"</strong>" The standard shell."</li>
                            <li><strong>"Zsh:"</strong>" Feature-rich, often preferred by developers."</li>
                            <li><strong>"Fish:"</strong>" User-friendly interactive shell."</li>
                        </ul>

                        <p class="note">"Note: The system will automatically provision the container and install the selected shell for you."</p>

                        <h3 id="installing-tool">"Installing Your CLI Tool"</h3>
                        <p>"Once the wizard completes, you have full root access (or pseudo-root capabilities depending on configuration) inside the container."</p>
                        <ul>
                            <li>"Run standard Linux commands: "<code>"apt-get update"</code>", "<code>"curl"</code>", "<code>"wget"</code>", "<code>"git clone"</code>"."</li>
                            <li>"Install your specific CLI tool."</li>
                        </ul>

                        <p><strong>"Example:"</strong></p>
                        <pre><code>"apt-get update && apt-get install -y curl
    curl -fsSL https://my-tool.com/install.sh | sh"</code></pre>

                        <p class="tip"><strong>"Tip:"</strong>" Clean up temporary files (like downloaded zips) to keep your project image small."</p>

                        <h3 id="writing-guide">"Writing the Guide"</h3>
                        <p>"In the right-hand pane, document your tool using Markdown."</p>
                        <ul>
                            <li><strong>"Headers:"</strong>" "<code>"# My Tool"</code>", "<code>"## Installation"</code>"."</li>
                            <li><strong>"Code Blocks:"</strong>" Use triple backticks (```) for code snippets."</li>
                            <li><strong>"Lists & Formatting:"</strong>" Standard bold, italics, and lists are supported."</li>
                        </ul>
                        <p>"This text will be rendered into beautiful HTML for your viewers."</p>
                    </section>

                    <section id="publishing">
                        <h2>"4. Publishing Your Demo"</h2>
                        <p>"Once your environment is set up and your guide is written:"</p>

                        <ol>
                            <li>
                                <strong>"Set a Slug:"</strong>" Enter a unique URL identifier for your project in the top navigation bar (e.g., "<code>"my-awesome-cli"</code>")."
                                <ul>
                                    <li><strong>"Validation:"</strong>" Only letters, numbers, and hyphens are allowed."</li>
                                </ul>
                            </li>
                            <li>
                                <strong>"Click \"Publish\":"</strong>
                                <ul>
                                    <li>"The system will pause your running container."</li>
                                    <li>"It creates a Docker Snapshot (Image) of the exact state of your file system."</li>
                                    <li>"It saves your Markdown guide and links it to this new image."</li>
                                    <li>"It generates a permanent link to your project."</li>
                                </ul>
                            </li>
                        </ol>

                        <p class="warning"><strong>"Warning:"</strong>" Publishing creates a static image. Any changes made afterwards requires re-publishing."</p>
                    </section>

                    <section id="managing">
                        <h2>"5. Managing Your Projects"</h2>

                        <h3 id="viewing-projects">"Viewing Your Projects"</h3>
                        <p>"All your published projects are accessible from your Dashboard. Each project card displays:"</p>
                        <ul>
                            <li>"The project slug (URL identifier)"</li>
                            <li>"The base Docker image used"</li>
                            <li>"Quick action buttons (View and Delete)"</li>
                        </ul>

                        <h3 id="deleting-projects">"Deleting Projects"</h3>
                        <p>"You can permanently delete any of your projects from the Dashboard."</p>

                        <p><strong>"To delete a project:"</strong></p>
                        <ol>
                            <li>"Navigate to your Dashboard."</li>
                            <li>"Find the project card you want to remove."</li>
                            <li>"Click the red \"Delete\" button on the project card."</li>
                            <li>"Confirm the deletion when prompted."</li>
                        </ol>

                        <p class="warning"><strong>"Warning:"</strong>" Deleting a project is permanent and cannot be undone. This will:"</p>
                        <ul>
                            <li>"Remove the project from your dashboard"</li>
                            <li>"Delete the Docker image snapshot from our servers"</li>
                            <li>"Make the public URL ("<code>"/<username>/<slug>"</code>") inaccessible"</li>
                            <li>"Break any existing embeds of this project"</li>
                        </ul>

                        <p class="tip"><strong>"Tip:"</strong>" If you just want to update your project, use the \"Re-publish\" feature instead of deleting and recreating it. This preserves your project's URL."</p>
                    </section>

                    <section id="sharing">
                        <h2>"6. Sharing & Embedding"</h2>

                        <h3 id="public-links">"Public Project Links"</h3>
                        <p>"Share your project using the URL format: "<code>"https://trycli.com/<your-username>/<project-slug>"</code></p>
                        <p>"Viewers who visit this link will:"</p>
                        <ul>
                            <li>"See your Markdown guide rendered on the left."</li>
                            <li>"Get a fresh, isolated copy of your container on the right."</li>
                            <li>"Have full interactive access to try the tool you installed."</li>
                        </ul>

                        <h3 id="embedding">"Embedding on Websites"</h3>
                        <p>"You can embed your CLI demo directly into your own documentation, blog, or landing page."</p>
                        <ol>
                            <li>"Go to your Project Page."</li>
                            <li>"Click the \"Share / Embed\" button in the top right."</li>
                            <li>"Copy the generated "<code>"<iframe>"</code>" code."</li>
                            <li>"Paste it into your website's HTML."</li>
                        </ol>

                        <p><strong>"Embed Features:"</strong></p>
                        <ul>
                            <li><strong>"Lazy Loading:"</strong>" The terminal only boots up when the user clicks \"Start Terminal\" to save resources."</li>
                            <li><strong>"Responsive:"</strong>" The embed adapts to the container width."</li>
                        </ul>
                    </section>

                    <section id="security">
                        <h2>"7. Security & Sandbox"</h2>
                        <p>"We take security seriously. While you (the creator) have broad permissions during the setup phase, the Viewer Containers (what your users see) are strictly sandboxed:"</p>

                        <ul>
                            <li><strong>"Network Isolation:"</strong>" Viewer containers run on a restricted bridge network. They cannot access internal services or other user containers."</li>
                            <li><strong>"Resource Limits:"</strong>" Each session is capped at 512MB RAM and 1.0 CPU Core to prevent abuse."</li>
                            <li><strong>"Anti-Abuse:"</strong>" We drop dangerous Linux capabilities (like CAP_SYS_ADMIN) and limit process counts (PIDs) to prevent \"fork bombs\"."</li>
                            <li><strong>"Ephemeral:"</strong>" All viewer sessions are temporary. As soon as the user closes the tab or the session times out, the container is destroyed and all data is wiped."</li>
                        </ul>
                    </section>
                </main>
            </div>
        </div>
    }
}
