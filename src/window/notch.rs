use crate::config::Config;
use crate::modules::ModulesContainer;
use gtk4::prelude::*;
use gtk4::{
    Align, ApplicationWindow, Box as GtkBox, EventControllerMotion, Label, Orientation, Revealer,
    RevealerTransitionType,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub struct NotchWidget {
    pub root_box: GtkBox,
}

impl NotchWidget {
    pub fn new(window: &ApplicationWindow, config: Config) -> Self {
        let root_box = GtkBox::new(Orientation::Vertical, 0);
        root_box.set_halign(Align::Center);
        root_box.set_valign(Align::Start);
        root_box.add_css_class("notch-root");

        // Pill / Collapsed Bar Header
        let pill_box = GtkBox::new(Orientation::Horizontal, 8);
        pill_box.set_halign(Align::Center);
        pill_box.set_valign(Align::Center);
        pill_box.set_size_request(config.general.collapsed_width, config.general.collapsed_height);
        pill_box.add_css_class("notch-pill");

        let pill_indicator = Label::new(Some("●  OpenNotch"));
        pill_indicator.add_css_class("pill-label");
        pill_box.append(&pill_indicator);

        // Expanded Content Container with Revealer
        let modules = ModulesContainer::new(&config);
        let revealer = Revealer::new();
        revealer.set_transition_type(RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(180);
        revealer.set_reveal_child(false);
        revealer.set_child(Some(&modules.container));

        root_box.append(&pill_box);
        root_box.append(&revealer);

        // Motion Controller for Hover Events
        let motion_ctrl = EventControllerMotion::new();
        let is_hovered = Rc::new(RefCell::new(false));
        let revealer_enter = revealer.clone();
        let is_hovered_enter = Rc::clone(&is_hovered);
        let window_enter = window.clone();
        let cfg_enter = config.clone();

        motion_ctrl.connect_enter(move |_, _x, _y| {
            *is_hovered_enter.borrow_mut() = true;
            revealer_enter.set_reveal_child(true);
            window_enter.set_size_request(
                cfg_enter.general.expanded_width,
                cfg_enter.general.expanded_height,
            );
            super::position::position_x11_top_center(
                &window_enter,
                cfg_enter.general.expanded_width,
                cfg_enter.general.expanded_height,
            );
        });

        let is_hovered_leave = Rc::clone(&is_hovered);
        let revealer_leave = revealer.clone();
        let window_leave = window.clone();
        let cfg_leave = config.clone();

        motion_ctrl.connect_leave(move |_| {
            *is_hovered_leave.borrow_mut() = false;
            let is_hovered_check = Rc::clone(&is_hovered_leave);
            let revealer_check = revealer_leave.clone();
            let window_check = window_leave.clone();
            let cfg_check = cfg_leave.clone();

            // Debounce before collapsing
            glib::timeout_add_local_once(
                Duration::from_millis(cfg_leave.general.hover_debounce_ms),
                move || {
                    if !*is_hovered_check.borrow() {
                        revealer_check.set_reveal_child(false);
                        window_check.set_size_request(
                            cfg_check.general.collapsed_width,
                            cfg_check.general.collapsed_height,
                        );
                        super::position::position_x11_top_center(
                            &window_check,
                            cfg_check.general.collapsed_width,
                            cfg_check.general.collapsed_height,
                        );
                    }
                },
            );
        });

        root_box.add_controller(motion_ctrl);

        Self { root_box }
    }
}
