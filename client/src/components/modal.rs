use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

#[component]
pub fn Modal(
    show: MaybeSignal<bool>,
    title: MaybeSignal<String>,
    body: MaybeSignal<String>,
    button_label: MaybeSignal<String>,
    on_close: Callback<()>,
    #[prop(optional)] children: Option<ChildrenFn>,
) -> impl IntoView {
    view! {
        {move || {
            let on_close = on_close.clone();
            let title = title.clone();
            let body = body.clone();
            let button_label = button_label.clone();
            let show = show.clone();
            let children = children.clone();
            if show.get() {
                let button_label_for_blocking = button_label.clone();
                let is_blocking = move || button_label_for_blocking.get().is_empty();
                view! {
                    <div 
                        class="modal-overlay" 
                        role="dialog" 
                        aria-modal="true"
                        on:click=move |ev: ev::MouseEvent| {
                            if !is_blocking() {
                                if let Some(target) = ev.target().and_then(|t| t.dyn_into::<HtmlElement>().ok()) {
                                    if target.class_list().contains("modal-overlay") {
                                        on_close.call(());
                                    }
                                }
                            }
                        }
                    >
                        <div class="modal-card">
                            <h3 class="modal-title">{move || title.get()}</h3>
                            <div class="modal-body">{move || body.get()}</div>
                            {move || {
                                if let Some(children) = children.clone() {
                                    view! { <div class="modal-body">{children()}</div> }.into_view()
                                } else {
                                    view! { <></> }.into_view()
                                }
                            }}
                            <div class="modal-actions">
                                {move || {
                                    let label = button_label.get();
                                    if !label.is_empty() {
                                        view! {
                                            <button class="btn-secondary btn-action" on:click=move |_| on_close.call(())>
                                                {label}
                                            </button>
                                        }.into_view()
                                    } else {
                                        view! { <></> }.into_view()
                                    }
                                }}
                            </div>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { <></> }.into_view()
            }
        }}
    }
}

#[component]
pub fn EmbedModal(
    show: MaybeSignal<bool>,
    title: MaybeSignal<String>,
    iframe_code: MaybeSignal<String>,
    smart_link: MaybeSignal<String>,
    vip_link: MaybeSignal<String>,
    whitelist: MaybeSignal<Vec<String>>,
    is_public: MaybeSignal<bool>,
    on_toggle_public: Callback<bool>,
    on_add_url: Callback<String>,
    on_remove_url: Callback<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    let (copied_iframe, set_copied_iframe) = create_signal(false);
    let (copied_link, set_copied_link) = create_signal(false);
    let (copied_vip, set_copied_vip) = create_signal(false);
    let (new_url, set_new_url) = create_signal(String::new());

    let iframe_ref = create_node_ref::<leptos::html::Textarea>();
    let link_ref = create_node_ref::<leptos::html::Input>();
    let vip_ref = create_node_ref::<leptos::html::Input>();

    view! {
        {move || {
            let show = show.clone();
            let title = title.clone();
            let iframe_code = iframe_code.clone();
            let smart_link = smart_link.clone();
            let vip_link = vip_link.clone();
            let whitelist = whitelist.clone();
            let on_close = on_close.clone();
            let on_add_url = on_add_url.clone();
            let on_remove_url = on_remove_url.clone();
            let is_public = is_public.clone();
            let on_toggle_public = on_toggle_public.clone();

            let iframe_code_for_click = iframe_code.clone();
            let smart_link_for_click = smart_link.clone();
            let vip_link_for_click = vip_link.clone();

            if show.get() {
                view! {
                    <div class="modal-overlay" role="dialog" aria-modal="true" style="backdrop-filter: blur(5px);">
                        <div class="modal-card" style="width: 600px; max-width: 95vw;">
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
                                <h3 class="modal-title" style="margin: 0; font-size: 1.25rem;">{move || title.get()}</h3>
                                <button class="btn-nav" on:click=move |_| on_close.call(()) style="font-size: 1.5rem; line-height: 1;">"×"</button>
                            </div>

                            // --- SECTION 0: VISIBILITY TOGGLE ---
                            <div style="background: rgba(34, 197, 94, 0.05); border: 1px solid rgba(34, 197, 94, 0.2); border-radius: 8px; padding: 16px; margin-bottom: 24px; display: flex; align-items: center; justify-content: space-between;">
                                <div>
                                    <div style="font-weight: 600; color: var(--text-main); margin-bottom: 4px;">"Public Access"</div>
                                    <div style="font-size: 0.85rem; color: var(--text-muted);">
                                        "Allow anyone to view/embed this project without whitelist restrictions."
                                    </div>
                                </div>
                                <label class="switch" style="position: relative; display: inline-block; width: 44px; height: 24px;">
                                    <input type="checkbox" 
                                        checked=is_public.get()
                                        on:change=move |ev| {
                                            let checked = event_target_checked(&ev);
                                            on_toggle_public.call(checked);
                                        }
                                        style="opacity: 0; width: 0; height: 0;"
                                    />
                                    <span class="slider" 
                                          style=move || {
                                              let bg = if is_public.get() { "#22c55e" } else { "#3f3f46" };
                                              format!("position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: {}; transition: .4s; border-radius: 34px;", bg)
                                          }
                                    >
                                        <span style=move || {
                                            let transform = if is_public.get() { "translateX(20px)" } else { "translateX(0)" };
                                            format!("position: absolute; content: ''; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; transition: .4s; border-radius: 50%; transform: {};", transform)
                                        }></span>
                                    </span>
                                </label>
                            </div>

                            // --- SECTION 1: IFRAME ---
                            <div style="margin-bottom: 24px; position: relative;">
                                <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                                    <label style="color:var(--text-main); font-weight:600; font-size: 0.9rem;">
                                        "Option 1: Iframe (For your website)"
                                    </label>
                                    {move || if copied_iframe.get() {
                                        view! { <span style="color: #22c55e; font-size: 0.8rem; font-weight: 600; animation: fadeIn 0.2s;">"✓ Copied!"</span> }.into_view()
                                    } else {
                                        view! { <span style="opacity: 0;">"Placeholder"</span> }.into_view()
                                    }}
                                </div>
                                <div class="input-hero-wrapper" style="display: flex; gap: 0;">
                                    <textarea
                                        class="modal-code"
                                        style="min-height: 100px; margin: 0; border-top-right-radius: 0; border-bottom-right-radius: 0; resize: none; font-size: 0.85rem;"
                                        readonly
                                        node_ref=iframe_ref
                                        prop:value=move || iframe_code.get()
                                    ></textarea>
                                    <button
                                        class="btn-secondary"
                                        style="border-top-left-radius: 0; border-bottom-left-radius: 0; border-left: none; width: 50px; display: flex; align-items: center; justify-content: center;"
                                        aria-label="Copy iframe code"
                                        on:click=move |_| {
                                            let text = iframe_code_for_click.get();
                                            let _ = window().navigator().clipboard().write_text(&text);
                                            if let Some(el) = iframe_ref.get() { el.select(); }
                                            set_copied_iframe.set(true);
                                            set_timeout(move || set_copied_iframe.set(false), std::time::Duration::from_millis(2000));
                                        }
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                        </svg>
                                    </button>
                                </div>
                            </div>

                            // --- SECTION 2: PRIVATE VIP LINK ---
                            <div style="margin-bottom: 24px; position: relative;">
                                <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                                    <label style="color:var(--text-main); font-weight:600; font-size: 0.9rem;">
                                        "Option 2: Private VIP Link"
                                    </label>
                                    {move || if copied_vip.get() {
                                        view! { <span style="color: #22c55e; font-size: 0.8rem; font-weight: 600; animation: fadeIn 0.2s;">"✓ Copied!"</span> }.into_view()
                                    } else {
                                        view! { <span style="opacity: 0;">"Placeholder"</span> }.into_view()
                                    }}
                                </div>

                                {move || {
                                    let link = vip_link.get();
                                    let vip_link_for_input = vip_link.clone();
                                    let vip_link_for_button = vip_link_for_click.clone();
                                    if link.is_empty() {
                                        view! {
                                            <div style="background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 8px; padding: 16px; text-align: center;">
                                                <p style="color: var(--text-muted); font-size: 0.85rem; margin: 0;">
                                                    "VIP link is being generated..."
                                                </p>
                                                <p style="color: var(--text-muted); font-size: 0.75rem; margin-top: 8px;">
                                                    "This may take a moment. The link will appear here once ready."
                                                </p>
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <>
                                                <div class="input-hero-wrapper" style="display: flex; gap: 0;">
                                                    <input
                                                        type="text"
                                                        class="input-slug" 
                                                        style="flex: 1; font-family: var(--font-mono); font-size: 0.85rem; border-top-right-radius: 0; border-bottom-right-radius: 0; padding: 10px;"
                                                        readonly
                                                        node_ref=vip_ref
                                                        prop:value=move || vip_link_for_input.get()
                                                    />
                                                    <button
                                                        class="btn-secondary"
                                                        style="border-top-left-radius: 0; border-bottom-left-radius: 0; border-left: none; width: 50px; display: flex; align-items: center; justify-content: center;"
                                                        aria-label="Copy VIP link"
                                                        on:click=move |_| {
                                                            let text = vip_link_for_button.get();
                                                            let _ = window().navigator().clipboard().write_text(&text);
                                                            if let Some(el) = vip_ref.get() { el.select(); }
                                                            set_copied_vip.set(true);
                                                            set_timeout(move || set_copied_vip.set(false), std::time::Duration::from_millis(2000));
                                                        }
                                                    >
                                                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                                            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                                        </svg>
                                                    </button>
                                                </div>
                                                <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 8px;">
                                                    <strong style="color: #dc2626;">"Security warning:"</strong>
                                                    " This VIP link bypasses the Guest List and must only be shared privately. Do NOT embed it on public websites, iframes, or forums; anyone with this link can access your terminal."
                                                </p>
                                            </>
                                        }.into_view()
                                    }
                                }}
                            </div>

                            // --- SECTION 3: SMART LINK ---
                            <div style="margin-bottom: 32px; position: relative;">
                                <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                                    <label style="color:var(--text-main); font-weight:600; font-size: 0.9rem;">
                                        "Option 3: Smart Link (Medium, Reddit)"
                                    </label>
                                    {move || if copied_link.get() {
                                        view! { <span style="color: #22c55e; font-size: 0.8rem; font-weight: 600; animation: fadeIn 0.2s;">"✓ Copied!"</span> }.into_view()
                                    } else {
                                        view! { <span style="opacity: 0;">"Placeholder"</span> }.into_view()
                                    }}
                                </div>
                                <div class="input-hero-wrapper" style="display: flex; gap: 0;">
                                    <input
                                        type="text"
                                        class="input-slug"
                                        style="flex: 1; font-family: var(--font-mono); font-size: 0.85rem; border-top-right-radius: 0; border-bottom-right-radius: 0; padding: 10px;"
                                        readonly
                                        node_ref=link_ref
                                        prop:value=move || smart_link.get()
                                    />
                                    <button
                                        class="btn-secondary"
                                        style="border-top-left-radius: 0; border-bottom-left-radius: 0; border-left: none; width: 50px; display: flex; align-items: center; justify-content: center;"
                                        aria-label="Copy link"
                                        on:click=move |_| {
                                            let text = smart_link_for_click.get();
                                            let _ = window().navigator().clipboard().write_text(&text);
                                            if let Some(el) = link_ref.get() { el.select(); }
                                            set_copied_link.set(true);
                                            set_timeout(move || set_copied_link.set(false), std::time::Duration::from_millis(2000));
                                        }
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                        </svg>
                                    </button>
                                </div>
                                <p style="font-size: 0.8rem; color: var(--text-muted); margin-top: 8px;">
                                    "Paste directly into Medium or Reddit to expand."
                                </p>
                            </div>

                            // --- SECTION 4: Guest List / Whitelist (CONDITIONAL) ---
                            {move || if !is_public.get() {
                                let whitelist = whitelist.clone(); // <--- Clone needed for Fn compliance
                                view! {
                                    <div style="margin-bottom: 24px;">
                                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;">
                                            <label style="color:var(--text-main); font-weight:600; font-size: 0.9rem;">
                                                "Guest List (Authorized URLs)"
                                            </label>
                                            <span style="font-size: 0.8rem; color: var(--text-muted);">
                                                "Only these pages can auto-launch your terminal."
                                            </span>
                                        </div>
                                        <div style="display: flex; gap: 10px; margin-bottom: 12px;">
                                            <input
                                                type="text"
                                                class="input-slug"
                                                style="flex: 1;"
                                                placeholder="https://medium.com/@user/article-slug"
                                                prop:value=new_url
                                                on:input=move |ev| set_new_url.set(event_target_value(&ev))
                                            />
                                            <button
                                                class="btn-primary"
                                                on:click=move |_| {
                                                    let url = new_url.get();
                                                    if !url.is_empty() {
                                                        on_add_url.call(url);
                                                        set_new_url.set(String::new());
                                                    }
                                                }
                                                prop:disabled=move || new_url.get().is_empty()
                                            >
                                                "Add URL"
                                            </button>
                                        </div>
                                        <div style="display: flex; flex-wrap: wrap; gap: 8px;">
                                            <For
                                                each=move || whitelist.get()
                                                key=|u| u.clone()
                                                children=move |url| {
                                                    let on_remove_url = on_remove_url.clone();
                                                    view! {
                                                        <span class="badge" style="margin: 0; display: flex; align-items: center; gap: 8px;">
                                                            {url.clone()}
                                                            <button
                                                                class="btn-nav"
                                                                style="padding: 0; color: #ef4444; font-weight: bold; font-size: 0.9rem;"
                                                                aria-label="Remove URL"
                                                                on:click=move |_| {
                                                                    on_remove_url.call(url.clone());
                                                                }
                                                            >
                                                                "×"
                                                            </button>
                                                        </span>
                                                    }
                                                }
                                            />
                                        </div>
                                    </div>
                                }.into_view()
                            } else {
                                view! {
                                    <div style="margin-bottom: 24px; padding: 16px; border: 1px dashed var(--border); border-radius: 8px; text-align: center; color: var(--text-muted); font-size: 0.9rem;">
                                        "Project is Public. Guest List restrictions are disabled."
                                    </div>
                                }.into_view()
                            }}

                            <div class="modal-actions">
                                <button class="btn-secondary btn-action" on:click=move |_| on_close.call(())>
                                    "Done"
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { <></> }.into_view()
            }
        }}
    }
}

#[component]
pub fn ConfirmModal(
    show: MaybeSignal<bool>,
    title: MaybeSignal<String>,
    body: MaybeSignal<String>,
    expected_name: MaybeSignal<String>,
    confirm_label: MaybeSignal<String>,
    cancel_label: MaybeSignal<String>,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (confirm_input, set_confirm_input) = create_signal(String::new());
    let (confirm_error, set_confirm_error) = create_signal(None::<String>);
    view! {
        {move || {
            let on_confirm = on_confirm.clone();
            let on_cancel = on_cancel.clone();
            let title = title.clone();
            let body = body.clone();
            let expected_name = expected_name.clone();
            let confirm_label = confirm_label.clone();
            let cancel_label = cancel_label.clone();
            let show = show.clone();
            if show.get() {
                view! {
                    <div class="modal-overlay" role="dialog" aria-modal="true">
                        <div class="modal-card">
                            <h3 class="modal-title">{move || title.get()}</h3>
                            <div class="modal-body">{move || body.get()}</div>
                            <div class="modal-body">
                                <p class="modal-description">
                                    "Type the environment name to confirm deletion:"
                                </p>
                                <input
                                    class="input-slug"
                                    type="text"
                                    prop:value=confirm_input
                                    on:input=move |ev| {
                                        set_confirm_input.set(event_target_value(&ev));
                                        set_confirm_error.set(None);
                                    }
                                />
                                {move || confirm_error.get().map(|err| view! {
                                    <div class="modal-copy-status" style="color:#ef4444;">{err}</div>
                                })}
                            </div>
                            <div class="modal-actions">
                                <button class="btn-secondary btn-action" on:click=move |_| on_cancel.call(())>
                                    {move || cancel_label.get()}
                                </button>
                                <button class="btn-secondary btn-action btn-danger" on:click=move |_| {
                                    if confirm_input.get().trim() == expected_name.get().trim() {
                                        on_confirm.call(());
                                        set_confirm_input.set(String::new());
                                        set_confirm_error.set(None);
                                    } else {
                                        set_confirm_error.set(Some("Name does not match.".to_string()));
                                    }
                                }>
                                    {move || confirm_label.get()}
                                </button>
                            </div>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! { <></> }.into_view()
            }
        }}
    }
}