use leptos::*;

#[component]
pub fn Modal(
    show: MaybeSignal<bool>,
    title: MaybeSignal<String>,
    body: MaybeSignal<String>,
    button_label: MaybeSignal<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        {move || {
            let on_close = on_close.clone();
            let title = title.clone();
            let body = body.clone();
            let button_label = button_label.clone();
            let show = show.clone();
            if show.get() {
                view! {
                    <div class="modal-overlay" role="dialog" aria-modal="true">
                        <div class="modal-card">
                            <h3 class="modal-title">{move || title.get()}</h3>
                            <div class="modal-body">{move || body.get()}</div>
                            <div class="modal-actions">
                                <button class="btn-secondary btn-action" on:click=move |_| on_close.call(())>
                                    {move || button_label.get()}
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
pub fn EmbedModal(
    show: MaybeSignal<bool>,
    title: MaybeSignal<String>,
    code: MaybeSignal<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    let (copied, set_copied) = create_signal(false);
    let textarea_ref = create_node_ref::<leptos::html::Textarea>();
    view! {
        {move || {
            let on_close = on_close.clone();
            let title = title.clone();
            let code = code.clone();
            let code_for_click = code.clone();
            let show = show.clone();
            if show.get() {
                view! {
                    <div class="modal-overlay" role="dialog" aria-modal="true">
                        <div class="modal-card">
                            <h3 class="modal-title">{move || title.get()}</h3>
                            <p class="modal-description">
                                "Copy and paste this embedd in your blogs and articles to demo your environment."
                            </p>
                            <textarea
                                class="modal-code"
                                readonly
                                node_ref=textarea_ref
                                prop:value=move || code.get()
                            ></textarea>
                            {move || if copied.get() {
                                view! { <div class="modal-copy-status">"Embed copied to clipboard"</div> }.into_view()
                            } else {
                                view! { <></> }.into_view()
                            }}
                            <div class="modal-actions">
                                <button
                                    class="modal-copy-btn"
                                    aria-label="Copy embed code"
                                    on:click=move |_| {
                                        let text = code_for_click.get();
                                        let _ = window().navigator().clipboard().write_text(&text);
                                        if let Some(el) = textarea_ref.get() {
                                            el.focus().ok();
                                            el.select();
                                        }
                                        set_copied.set(true);
                                        set_timeout(move || {
                                            set_copied.set(false);
                                        }, std::time::Duration::from_millis(2000));
                                    }
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                                        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                                    </svg>
                                </button>
                                <button class="btn-secondary btn-action" on:click=move |_| on_close.call(())>
                                    "Close"
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
