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
        .icon_name("open-notch")
        .default_width(config.general.expanded_width)
        .default_height(config.general.expanded_height)
        .decorated(false)
        .focusable(true)
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

    // Configure window positioning & input shape immediately on realization
    window.connect_realize(move |_| {
        position::position_x11_top_center(
            &win_setup,
            cfg_setup.general.expanded_width,
            cfg_setup.general.expanded_height,
        );
        position::update_notch_input_region(
            &win_setup,
            false,
            cfg_setup.general.collapsed_width,
            cfg_setup.general.collapsed_height,
            cfg_setup.general.expanded_width,
            cfg_setup.general.expanded_height,
        );

        let win_post = win_setup.clone();
        let cfg_post = cfg_setup.clone();
        // Re-enforce position after Mutter maps the window
        glib::idle_add_local_once(move || {
            position::position_x11_top_center(
                &win_post,
                cfg_post.general.expanded_width,
                cfg_post.general.expanded_height,
            );
        });
    });

    let notch = NotchWidget::new(&window, config);
    window.set_child(Some(&notch.root_box));

    window.present();
}
