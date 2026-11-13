pub mod notch;
pub mod position;

use crate::config::Config;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow};
use notch::NotchWidget;

#[cfg(feature = "layer-shell")]
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

pub fn build_ui(app: &Application, config: Config) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("OpenNotch")
        .default_width(config.general.collapsed_width)
        .default_height(config.general.collapsed_height)
        .decorated(false)
        .build();

    #[cfg(feature = "layer-shell")]
    {
        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::OnDemand);
            window.set_anchor(Edge::Top, true);
            window.set_margin(Edge::Top, 0);
        }
    }

    let win_setup = window.clone();
    let cfg_setup = config.clone();
    window.connect_realize(move |_| {
        let win_run = win_setup.clone();
        let cfg_run = cfg_setup.clone();
        glib::idle_add_local_once(move || {
            position::position_x11_top_center(
                &win_run,
                cfg_run.general.collapsed_width,
                cfg_run.general.collapsed_height,
            );
        });
    });

    let notch = NotchWidget::new(&window, config);
    window.set_child(Some(&notch.root_box));

    window.present();
}
