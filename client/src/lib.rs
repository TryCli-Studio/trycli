use leptos::*;

pub mod types;
pub mod api;
pub mod app;
pub mod components {
    pub mod terminal;
    pub mod protected;
    pub mod limit;
    pub mod navbar;
    pub mod modal;
}
pub mod pages {
    pub mod home;
    pub mod dashboard;
    pub mod create;
    pub mod view;
    pub mod embed;
    pub mod docs;
    pub mod blogs;
}

pub use app::App;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(|| view! { <App /> })
}