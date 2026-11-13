use crate::config::Config;
use crate::modules::ModulesContainer;
use gtk4::prelude::*;
use gtk4::{
    Align, ApplicationWindow, Box as GtkBox, EventControllerMotion, EventControllerScroll,
    EventControllerScrollFlags, Label, Orientation, PropagationPhase, Revealer,
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

        // Resting Pill (Top-Center)
        let pill_box = GtkBox::new(Orientation::Horizontal, 0);
        pill_box.set_halign(Align::Center);
        pill_box.set_valign(Align::Start);
        pill_box.set_size_request(config.general.collapsed_width, config.general.collapsed_height);
        pill_box.add_css_class("notch-pill");

        // Minimal Dot Indicator (Changes color when Camera/Mic are active)
        let pill_dot = Label::new(Some("●"));
        pill_dot.set_halign(Align::Center);
        pill_dot.set_valign(Align::Center);
        pill_dot.add_css_class("pill-dot");
        pill_box.append(&pill_dot);

        // Privacy Status Monitor (Camera = Green, Mic = Purple, Camera > Mic priority)
        let pill_box_privacy = pill_box.clone();
        let pill_dot_privacy = pill_dot.clone();
        let last_privacy_state = Rc::new(RefCell::new(crate::privacy::PrivacyState::Idle));

        glib::timeout_add_local(Duration::from_millis(500), move || {
            let current = crate::privacy::check_privacy_status();
            let mut last = last_privacy_state.borrow_mut();

            if *last != current {
                *last = current;
                match current {
                    crate::privacy::PrivacyState::CameraActive => {
                        pill_box_privacy.remove_css_class("mic-active");
                        pill_box_privacy.add_css_class("camera-active");
                        pill_dot_privacy.remove_css_class("mic-dot");
                        pill_dot_privacy.add_css_class("camera-dot");
                    }
                    crate::privacy::PrivacyState::MicrophoneActive => {
                        pill_box_privacy.remove_css_class("camera-active");
                        pill_box_privacy.add_css_class("mic-active");
                        pill_dot_privacy.remove_css_class("camera-dot");
                        pill_dot_privacy.add_css_class("mic-dot");
                    }
                    crate::privacy::PrivacyState::Idle => {
                        pill_box_privacy.remove_css_class("camera-active");
                        pill_box_privacy.remove_css_class("mic-active");
                        pill_dot_privacy.remove_css_class("camera-dot");
                        pill_dot_privacy.remove_css_class("mic-dot");
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        // Expanded Shelf with Modules
        let modules = ModulesContainer::new(&config);
        let revealer = Revealer::new();
        revealer.set_transition_type(RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(180);
        revealer.set_reveal_child(false);
        revealer.set_child(Some(&modules.container));

        root_box.append(&pill_box);
        root_box.append(&revealer);

        // Motion & Scroll Controllers for Hover & Scroll Events
        let motion_ctrl = EventControllerMotion::new();
        let scroll_ctrl = EventControllerScroll::new(EventControllerScrollFlags::BOTH_AXES);
        scroll_ctrl.set_propagation_phase(PropagationPhase::Capture);

        let is_hovered = Rc::new(RefCell::new(false));

        let revealer_enter = revealer.clone();
        let is_hovered_enter = Rc::clone(&is_hovered);
        let window_enter = window.clone();
        let cfg_enter = config.clone();

        motion_ctrl.connect_enter(move |_, _x, _y| {
            *is_hovered_enter.borrow_mut() = true;
            revealer_enter.set_reveal_child(true);
            super::position::update_notch_input_region(
                &window_enter,
                true,
                cfg_enter.general.collapsed_width,
                cfg_enter.general.collapsed_height,
                cfg_enter.general.expanded_width,
                cfg_enter.general.expanded_height,
            );
        });

        let is_hovered_motion = Rc::clone(&is_hovered);
        motion_ctrl.connect_motion(move |_, _x, _y| {
            *is_hovered_motion.borrow_mut() = true;
        });

        // Ensure scrolling inside the notch (e.g. schedule list, notes) keeps the notch expanded
        let is_hovered_scroll = Rc::clone(&is_hovered);
        let revealer_scroll = revealer.clone();
        let window_scroll = window.clone();
        let cfg_scroll = config.clone();

        scroll_ctrl.connect_scroll(move |_, _dx, _dy| {
            *is_hovered_scroll.borrow_mut() = true;
            revealer_scroll.set_reveal_child(true);
            super::position::update_notch_input_region(
                &window_scroll,
                true,
                cfg_scroll.general.collapsed_width,
                cfg_scroll.general.collapsed_height,
                cfg_scroll.general.expanded_width,
                cfg_scroll.general.expanded_height,
            );
            glib::Propagation::Proceed
        });

        let is_hovered_leave = Rc::clone(&is_hovered);
        let revealer_leave = revealer.clone();
        let window_leave = window.clone();
        let cfg_leave = config.clone();
        let motion_ctrl_leave = motion_ctrl.clone();

        motion_ctrl.connect_leave(move |_| {
            let is_hovered_check = Rc::clone(&is_hovered_leave);
            let revealer_check = revealer_leave.clone();
            let window_check = window_leave.clone();
            let cfg_check = cfg_leave.clone();
            let ctrl_check = motion_ctrl_leave.clone();

            // Debounce before collapsing
            glib::timeout_add_local_once(
                Duration::from_millis(cfg_leave.general.hover_debounce_ms),
                move || {
                    // If pointer is still inside the notch (or clicking interactive children), do not close
                    if ctrl_check.contains_pointer() {
                        *is_hovered_check.borrow_mut() = true;
                        return;
                    }

                    *is_hovered_check.borrow_mut() = false;
                    revealer_check.set_reveal_child(false);
                    super::position::update_notch_input_region(
                        &window_check,
                        false,
                        cfg_check.general.collapsed_width,
                        cfg_check.general.collapsed_height,
                        cfg_check.general.expanded_width,
                        cfg_check.general.expanded_height,
                    );
                },
            );
        });

        root_box.add_controller(motion_ctrl);
        root_box.add_controller(scroll_ctrl);

        Self { root_box }
    }
}
