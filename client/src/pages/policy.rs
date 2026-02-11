use crate::api::api_base;
use crate::components::navbar::Navbar;
use crate::types::User;
use gloo_net::http::Request;
use leptos::*;
use leptos_router::A;
use web_sys::RequestCredentials;

#[component]
pub fn PolicyPage() -> impl IntoView {
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

                    <button
                        class="hamburger-menu"
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

                    <div class="mobile-menu" class:open=move || menu_open.get()>
                        <A href="/" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Home"</A>
                        <A href="/dashboard" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Dashboard"</A>
                        <A href="/docs" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Docs"</A>
                        <A href="/blogs" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Blogs"</A>
                        <a href="https://twitter.com" target="_blank" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Twitter"</a>
                        <a href="https://ko-fi.com/tryclistudio" class="menu-item" on:click=move |_| set_menu_open.set(false)>"Support Us"</a>
                    </div>
                </div>
            </Navbar>

            <div class="docs-layout">
                // --- Mobile Sidebar Toggle ---
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
                    <span>"Sections"</span>
                </button>

                // --- Sidebar (Table of Contents) ---
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
                        <h3 class="toc-title">"Terms of Service"</h3>
                        <ul class="toc-list" on:click=move |_| set_sidebar_open.set(false)>
                            <li><a href="#introduction">"1. Introduction"</a></li>
                            <li><a href="#nature-of-service">"2. Nature of Service"</a></li>
                            <li><a href="#responsibilities">"3. Responsibilities & Conduct"</a></li>
                            <li><a href="#proprietary">"4. Proprietary Rights"</a></li>
                            <li><a href="#indemnification">"5. Indemnification"</a></li>
                            <li><a href="#disclaimer">"6. Disclaimer of Warranties"</a></li>
                            <li><a href="#liability">"7. Limitation of Liability"</a></li>
                            <li><a href="#termination">"8. Termination"</a></li>
                            <li><a href="#governing">"9. Governing Law"</a></li>
                        </ul>
                    </div>
                </aside>

                // --- Main Legal Content ---
                <main class="docs-content">
                    <h1>"TryCLI Terms of Service"</h1>
                    <p class="text-sm text-gray-500">"Last Updated: February 9, 2026"</p>

                    <section id="introduction">
                        <h2>"1. Introduction and Acceptance of Terms"</h2>
                        <p>"These Terms of Service (\"Terms\") constitute a binding legal agreement between you (\"Publisher,\" \"User,\" or \"You\") and TryCLI (\"Platform,\" \"We,\" or \"Us\"). By accessing, registering for, or using the TryCLI platform, including our browser-based sandbox environment, live-syncing Markdown editor, and related developer tools (collectively, the \"Services\"), you acknowledge that you have read, understood, and agree to be bound by these Terms."</p>
                        <p>"If you do not agree to these Terms, you must immediately discontinue use of the Services."</p>
                    </section>

                    <section id="nature-of-service">
                        <h2>"2. Nature of the Service: Passive Conduit"</h2>

                        <h3>"2.1 Platform Status"</h3>
                        <p>"You acknowledge and agree that TryCLI operates solely as a technological intermediary and hosting platform. We provide the infrastructure (containerized environments) for the execution of code and the display of documentation. We do not create, select, or modify the content, code, or applications (\"User Content\") uploaded or executed by Publishers."</p>

                        <h3>"2.2 No Editorial Control"</h3>
                        <p>"TryCLI exercises no editorial control over User Content. We act as a passive conduit for the transmission, storage, and execution of information provided by you. As such, TryCLI explicitly disclaims any liability for the accuracy, legality, safety, or functionality of any User Content submitted or executed on the Platform."</p>
                    </section>

                    <section id="responsibilities">
                        <h2>"3. User Responsibilities and Conduct"</h2>

                        <h3>"3.1 Publisher Sovereignty"</h3>
                        <p>"You are solely responsible for all activity that occurs under your account, including all code execution, software dependencies, and data transmission. You assume full liability for any damage, loss, or legal consequences resulting from your use of the Services."</p>

                        <h3>"3.2 Prohibited Activities (Acceptable Use Policy)"</h3>
                        <p>"To maintain the integrity and performance of the Service, you agree not to misuse the platform. The following activities are strictly prohibited:"</p>
                        <ul>
                            <li><strong>"Resource Abuse:"</strong>" You may not execute code designed to exhaust system resources, including but not limited to recursive process spawning (\"fork bombs\"), infinite loops intended to freeze the environment, or memory exhaustion attacks."</li>
                            <li><strong>"Cryptocurrency Mining:"</strong>" You are strictly prohibited from using the Service for cryptocurrency mining, Proof-of-Work calculations, or any other unauthorized distributed ledger verification tasks. We reserve the right to immediately terminate instances found violating this policy."</li>
                            <li><strong>"Malicious Code:"</strong>" You shall not upload, execute, or distribute viruses, worms, Trojan horses, rootkits, or any other malicious software intended to damage, interfere with, or gain unauthorized access to any system, data, or personal information."</li>
                            <li><strong>"Network Attacks:"</strong>" You may not use the Service to conduct Denial of Service (DoS) attacks, port scanning, or unauthorized penetration testing against third-party networks or TryCLI infrastructure."</li>
                        </ul>

                        <h3>"3.3 Security of Dependencies"</h3>
                        <p>"You acknowledge that modern software development relies on third-party libraries (e.g., packages from npm, pip, cargo). You are solely responsible for vetting and auditing any third-party dependencies you introduce into the sandbox environment. TryCLI is not responsible for \"supply chain\" attacks or vulnerabilities introduced via your selection of third-party software."</p>
                    </section>

                    <section id="proprietary">
                        <h2>"4. Proprietary Rights and License"</h2>
                        <h3>"4.1 Your Content"</h3>
                        <p>"You retain all ownership rights to the User Content you create, upload, or execute on TryCLI."</p>

                        <h3>"4.2 License to Host"</h3>
                        <p>"By submitting User Content to the Service, you grant TryCLI a worldwide, non-exclusive, royalty-free license to use, reproduce, modify, adapt, publish, and display such content solely for the purpose of providing the Services (e.g., running your code in a container, displaying your Markdown tutorial)."</p>
                    </section>

                    <section id="indemnification">
                        <h2>"5. Indemnification"</h2>
                        <p>"You agree to defend, indemnify, and hold harmless TryCLI, its affiliates, licensors, and service providers, and its and their respective officers, directors, employees, contractors, agents, licensors, suppliers, successors, and assigns from and against any claims, liabilities, damages, judgments, awards, losses, costs, expenses, or fees (including reasonable attorneys' fees) arising out of or relating to:"</p>
                        <ul>
                            <li>"Your violation of these Terms;"</li>
                            <li>"Your User Content, including but not limited to any claim that your code infringes the intellectual property rights of a third party or causes damage to a third party's systems;"</li>
                            <li>"Your use of any third-party dependencies or external APIs."</li>
                        </ul>
                    </section>

                    <section id="disclaimer">
                        <h2>"6. Disclaimer of Warranties"</h2>
                        <p><strong>"THE SERVICES ARE PROVIDED ON AN \"AS IS\" AND \"AS AVAILABLE\" BASIS, WITHOUT ANY WARRANTIES OF ANY KIND, EITHER EXPRESS OR IMPLIED."</strong></p>

                        <h3>"6.1 No Warranty of Functionality"</h3>
                        <p>"TryCLI does not warrant that the Services will be uninterrupted, secure, or error-free, or that any defects will be corrected. We do not guarantee that code which runs in our sandbox environment will function correctly in other environments or on local machines (\"Works on My Machine\" disclaimer)."</p>

                        <h3>"6.2 Data Persistence"</h3>
                        <p>"TryCLI is a sandbox environment designed for development and testing. We do not guarantee the permanent persistence of data, container states, or file systems. You are responsible for backing up your own data."</p>
                    </section>

                    <section id="liability">
                        <h2>"7. Limitation of Liability"</h2>
                        <p>"TO THE FULLEST EXTENT PERMITTED BY LAW, IN NO EVENT WILL TRYCLI BE LIABLE FOR DAMAGES OF ANY KIND, UNDER ANY LEGAL THEORY, ARISING OUT OF OR IN CONNECTION WITH YOUR USE, OR INABILITY TO USE, THE SERVICES, INCLUDING ANY DIRECT, INDIRECT, SPECIAL, INCIDENTAL, CONSEQUENTIAL, OR PUNITIVE DAMAGES, INCLUDING BUT NOT LIMITED TO, PERSONAL INJURY, PAIN AND SUFFERING, EMOTIONAL DISTRESS, LOSS OF REVENUE, LOSS OF PROFITS, LOSS OF BUSINESS OR ANTICIPATED SAVINGS, LOSS OF USE, LOSS OF GOODWILL, OR LOSS OF DATA, WHETHER CAUSED BY TORT (INCLUDING NEGLIGENCE), BREACH OF CONTRACT, OR OTHERWISE, EVEN IF FORESEEABLE."</p>
                    </section>

                    <section id="termination">
                        <h2>"8. Termination"</h2>
                        <p>"TryCLI reserves the right, in its sole discretion, to terminate or suspend your access to all or part of the Services for any reason, including, without limitation, breach of these Terms, or if we determine that your usage patterns (e.g., excessive CPU usage, suspected crypto-mining) pose a risk to the platform's stability."</p>
                    </section>

                    <section id="governing">
                        <h2>"9. Governing Law"</h2>
                        <p>"These Terms shall be governed by and construed in accordance with the laws of the jurisdiction in which TryCLI is incorporated, without giving effect to any choice or conflict of law provision or rule."</p>
                    </section>
                </main>
            </div>
        </div>
    }
}
