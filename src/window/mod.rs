pub mod notch;

use crate::config::Config;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use notch::NotchWidget;

pub fn build_ui(app: &Application, config: Config) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("OpenNotch")
        .default_width(config.general.collapsed_width)
        .default_height(config.general.collapsed_height)
        .decorated(false)
        .build();

    // Initialize Layer Shell for Wayland
    if gtk4_layer_shell::is_supported() {
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::OnDemand);
        window.set_anchor(Edge::Top, true);
        window.set_margin(Edge::Top, 0);
    }

    let notch = NotchWidget::new(&window, config);
    window.set_child(Some(&notch.root_box));

    window.present();
}
