use crate::api::api_base;
use leptos::*;
use leptos_router::A;
use std::rc::Rc;
use wasm_bindgen::JsCast;

const GITHUB_URL: &str = "https://github.com/TryCli-Studio/trycli";
const TWITTER_URL: &str = "https://x.com/tryclistudio";
const SUPPORT_URL: &str = "https://ko-fi.com/tryclistudio";

#[component]
pub fn HamburgerMenu(
    #[prop(into)] button_class: String,
    #[prop(into)] menu_class: String,
    #[prop(into)] item_class: String,
    #[prop(optional, into)] logout_class: String,
    #[prop(optional, into)] support_url: String,
    #[prop(optional, into)] link_style: String,
    #[prop(optional)] show_home: bool,
    #[prop(optional)] show_dashboard: bool,
    #[prop(optional, default = true)] show_docs: bool,
    #[prop(optional, default = true)] show_blogs: bool,
    #[prop(optional)] show_github: bool,
    #[prop(optional, default = true)] show_twitter: bool,
    #[prop(optional, default = true)] show_support: bool,
    #[prop(optional)] show_logout: bool,
    #[prop(optional)] use_open_class: bool,
    #[prop(optional)] stop_propagation: bool,
    #[prop(optional)] close_on_item_click: bool,
    #[prop(optional)] close_on_outside_click: bool,
    #[prop(optional, into)] github_url: String,
    #[prop(optional, default = false)] support_target_blank: bool,
) -> impl IntoView {
    let (menu_open, set_menu_open) = create_signal(false);

    if close_on_outside_click {
        create_effect(move |_| {
            if menu_open.get() {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
                            move |_: web_sys::Event| {
                                set_menu_open.set(false);
                            },
                        )
                            as Box<dyn Fn(web_sys::Event)>);

                        let _ = document.add_event_listener_with_callback(
                            "click",
                            closure.as_ref().unchecked_ref(),
                        );
                        closure.forget();
                    }
                }
            }
        });
    }

    let logout_class = if logout_class.is_empty() {
        item_class.clone()
    } else {
        logout_class
    };

    let support_url = if support_url.is_empty() {
        SUPPORT_URL.to_string()
    } else {
        support_url
    };

    let github_url = if github_url.is_empty() {
        GITHUB_URL.to_string()
    } else {
        github_url
    };

    let menu_items: Rc<dyn Fn() -> View> = Rc::new(move || {
        let item_class_home = item_class.clone();
        let link_style_home = link_style.clone();
        let item_class_dashboard = item_class.clone();
        let link_style_dashboard = link_style.clone();
        let item_class_docs = item_class.clone();
        let link_style_docs = link_style.clone();
        let item_class_blogs = item_class.clone();
        let link_style_blogs = link_style.clone();
        let item_class_github = item_class.clone();
        let link_style_github = link_style.clone();
        let item_class_twitter = item_class.clone();
        let link_style_twitter = link_style.clone();
        let item_class_support = item_class.clone();
        let link_style_support = link_style.clone();
        let logout_class = logout_class.clone();
        let link_style_logout = link_style.clone();
        let github_url = github_url.clone();
        let support_url = support_url.clone();

        view! {
            {move || if show_home {
                view! {
                    <A href="/" class=item_class_home.clone() attr:style=link_style_home.clone() on:click=move |_| {
                        if close_on_item_click {
                            set_menu_open.set(false);
                        }
                    }>
                        "Home"
                    </A>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_dashboard {
                view! {
                    <A href="/dashboard" class=item_class_dashboard.clone() attr:style=link_style_dashboard.clone() on:click=move |_| {
                        if close_on_item_click {
                            set_menu_open.set(false);
                        }
                    }>
                        "Dashboard"
                    </A>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_docs {
                view! {
                    <A href="/docs" class=item_class_docs.clone() attr:style=link_style_docs.clone() on:click=move |_| {
                        if close_on_item_click {
                            set_menu_open.set(false);
                        }
                    }>
                        "Docs"
                    </A>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_blogs {
                view! {
                    <A href="/blogs" class=item_class_blogs.clone() attr:style=link_style_blogs.clone() on:click=move |_| {
                        if close_on_item_click {
                            set_menu_open.set(false);
                        }
                    }>
                        "Blogs"
                    </A>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_github {
                view! {
                    <a
                        href=github_url.clone()
                        target="_blank"
                        rel="noopener noreferrer"
                        class=item_class_github.clone()
                        style=link_style_github.clone()
                        on:click=move |_| {
                            if close_on_item_click {
                                set_menu_open.set(false);
                            }
                        }
                    >
                        "GitHub Repo"
                    </a>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_twitter {
                view! {
                    <a href=TWITTER_URL target="_blank" class=item_class_twitter.clone() style=link_style_twitter.clone() on:click=move |_| {
                        if close_on_item_click {
                            set_menu_open.set(false);
                        }
                    }>
                        "Twitter"
                    </a>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_support {
                if support_target_blank {
                    view! {
                        <a href=support_url.clone() target="_blank" class=item_class_support.clone() style=link_style_support.clone() on:click=move |_| {
                            if close_on_item_click {
                                set_menu_open.set(false);
                            }
                        }>
                            "Support Us"
                        </a>
                    }
                        .into_view()
                } else {
                    view! {
                        <a href=support_url.clone() class=item_class_support.clone() style=link_style_support.clone() on:click=move |_| {
                            if close_on_item_click {
                                set_menu_open.set(false);
                            }
                        }>
                            "Support Us"
                        </a>
                    }
                        .into_view()
                }
            } else {
                view! { <></> }.into_view()
            }}
            {move || if show_logout {
                view! {
                    <div style="border-top: 1px solid var(--border); margin: 4px 0;"></div>
                    <a
                        href=format!("{}/auth/logout", api_base())
                        class=logout_class.clone()
                        rel="external"
                        style=link_style_logout.clone()
                        on:click=move |_| {
                            if close_on_item_click {
                                set_menu_open.set(false);
                            }
                        }
                    >
                        "Logout"
                    </a>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }}
        }
        .into_view()
    });

    view! {
        <button
            class=button_class
            class:open=move || use_open_class && menu_open.get()
            on:click=move |e: ev::MouseEvent| {
                if stop_propagation {
                    e.stop_propagation();
                }
                set_menu_open.update(|open| *open = !*open);
            }
            aria-label="Toggle menu"
        >
            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="3" y1="12" x2="21" y2="12"></line>
                <line x1="3" y1="6" x2="21" y2="6"></line>
                <line x1="3" y1="18" x2="21" y2="18"></line>
            </svg>
        </button>
        {move || {
            let menu_items = menu_items.clone();
            if use_open_class {
                view! {
                    <div class=menu_class.clone() class:open=move || menu_open.get() on:click=move |e: ev::MouseEvent| {
                        if stop_propagation {
                            e.stop_propagation();
                        }
                    }>
                        {menu_items()}
                    </div>
                }
                    .into_view()
            } else if menu_open.get() {
                view! {
                    <div class=menu_class.clone() on:click=move |e: ev::MouseEvent| {
                        if stop_propagation {
                            e.stop_propagation();
                        }
                    }>
                        {menu_items()}
                    </div>
                }
                    .into_view()
            } else {
                view! { <></> }.into_view()
            }
        }}
    }
}
