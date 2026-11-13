mod config;
mod modules;
mod window;

use config::Config;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{Application, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};

const APP_ID: &str = "com.github.open_notch.OpenNotch";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| {
        load_css();
    });

    app.connect_activate(|app| {
        let config = Config::load();
        window::build_ui(app, config);
    });

    app.run();
}

fn load_css() {
    let provider = CssProvider::new();
    let css_data = include_str!("../resources/style.css");
    provider.load_from_data(css_data);

    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
