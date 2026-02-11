use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::window;
use std::rc::Rc;
use std::cell::RefCell;

/// Hook to detect portrait mode on mobile and provide a signal for blocking modal
/// Returns a ReadSignal<bool> that is true when device is in portrait orientation
pub fn use_landscape_lock() -> ReadSignal<bool> {
    let (is_portrait, set_is_portrait) = create_signal(false);

    create_effect(move |_| {
        if let Some(w) = window() {
            // Initial check: portrait if height > width
            let check_portrait = || {
                let height = w.inner_height().ok().and_then(|h| h.as_f64()).unwrap_or(0.0);
                let width = w.inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(0.0);
                height > width
            };

            set_is_portrait.set(check_portrait());

            // Create shared references for the callbacks
            let set_portrait_clone1 = Rc::new(RefCell::new(set_is_portrait));
            let set_portrait_clone2 = set_portrait_clone1.clone();

            // Listen for both orientationchange and resize events
            let closure1 = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Some(window) = window() {
                    let height = window.inner_height().ok().and_then(|h| h.as_f64()).unwrap_or(0.0);
                    let width = window.inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(0.0);
                    let portrait = height > width;
                    
                    if let Ok(mut setter) = set_portrait_clone1.try_borrow_mut() {
                        setter.set(portrait);
                    }
                }
            }) as Box<dyn Fn(web_sys::Event)>);

            w.add_event_listener_with_callback("orientationchange", closure1.as_ref().unchecked_ref()).ok();
            closure1.forget();

            let closure2 = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Some(window) = window() {
                    let height = window.inner_height().ok().and_then(|h| h.as_f64()).unwrap_or(0.0);
                    let width = window.inner_width().ok().and_then(|w| w.as_f64()).unwrap_or(0.0);
                    let portrait = height > width;
                    
                    if let Ok(mut setter) = set_portrait_clone2.try_borrow_mut() {
                        setter.set(portrait);
                    }
                }
            }) as Box<dyn Fn(web_sys::Event)>);

            w.add_event_listener_with_callback("resize", closure2.as_ref().unchecked_ref()).ok();
            closure2.forget();
        }
    });

    is_portrait
}
