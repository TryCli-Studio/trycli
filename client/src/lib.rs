use leptos::*;

pub mod api;
pub mod app;
pub mod types;
pub mod components {
    pub mod limit;
    pub mod modal;
    pub mod navbar;
    pub mod protected;
    pub mod terminal;
}
pub mod pages {
    pub mod admin;
    pub mod analytics;
    pub mod blogs;
    pub mod create;
    pub mod dashboard;
    pub mod docs;
    pub mod embed;
    pub mod home;
    pub mod policy;
    pub mod view;
    pub mod notfound;
}

pub use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App /> })
}
