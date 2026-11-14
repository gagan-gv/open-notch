use crate::config::Config;
use crate::modules::ModulesContainer;
use gtk4::prelude::*;
use gtk4::{
    Align, ApplicationWindow, Box as GtkBox, DrawingArea, EventControllerMotion, EventControllerScroll,
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

        // Pill layout: [ Left Media Icon ] [ Center Dot ] [ Right Equalizer Waveform ]
        let left_media_box = GtkBox::new(Orientation::Horizontal, 0);
        left_media_box.set_halign(Align::Start);
        left_media_box.set_valign(Align::Center);
        left_media_box.set_hexpand(true);
        left_media_box.set_visible(false);

        let left_media_icon = Label::new(Some("🎵"));
        left_media_icon.add_css_class("pill-media-icon");
        left_media_icon.set_margin_start(14);
        left_media_box.append(&left_media_icon);

        let center_box = GtkBox::new(Orientation::Horizontal, 0);
        center_box.set_halign(Align::Center);
        center_box.set_valign(Align::Center);

        // Minimal Dot Indicator (Changes color when Camera/Mic are active)
        let pill_dot = Label::new(Some("●"));
        pill_dot.set_halign(Align::Center);
        pill_dot.set_valign(Align::Center);
        pill_dot.add_css_class("pill-dot");
        center_box.append(&pill_dot);

        let right_media_box = GtkBox::new(Orientation::Horizontal, 0);
        right_media_box.set_halign(Align::End);
        right_media_box.set_valign(Align::Center);
        right_media_box.set_hexpand(true);
        right_media_box.set_visible(false);

        // Animated Audio Waveform Equalizer (DrawingArea)
        let eq_area = DrawingArea::new();
        eq_area.set_content_width(22);
        eq_area.set_content_height(14);
        eq_area.set_valign(Align::Center);
        eq_area.set_halign(Align::Center);
        eq_area.set_margin_end(14);
        eq_area.add_css_class("pill-eq-area");
        right_media_box.append(&eq_area);

        pill_box.append(&left_media_box);
        pill_box.append(&center_box);
        pill_box.append(&right_media_box);

        // Waveform Equalizer Drawing Function
        let animation_phase = Rc::new(RefCell::new(0.0f64));
        let is_animating = Rc::new(RefCell::new(false));
        let player_is_spotify = Rc::new(RefCell::new(false));

        let phase_draw = Rc::clone(&animation_phase);
        let spotify_draw = Rc::clone(&player_is_spotify);

        eq_area.set_draw_func(move |_, cr, width, height| {
            let t = *phase_draw.borrow();
            let num_bars = 4;
            let bar_w = 2.4;
            let bar_spacing = 2.0;
            let total_w = num_bars as f64 * bar_w + (num_bars - 1) as f64 * bar_spacing;
            let start_x = (width as f64 - total_w) / 2.0;
            let max_h = (height as f64 - 2.0).max(4.0);

            // Spotify vibrant green or Dynamic Island cyan
            if *spotify_draw.borrow() {
                cr.set_source_rgba(0.13, 0.84, 0.38, 0.95);
            } else {
                cr.set_source_rgba(0.22, 0.77, 0.98, 0.95);
            }

            for i in 0..num_bars {
                let x = start_x + i as f64 * (bar_w + bar_spacing);
                let wave = match i {
                    0 => (t * 5.2).sin().abs() * 0.7 + (t * 2.3).cos().abs() * 0.3,
                    1 => (t * 7.8 + 1.2).sin().abs() * 0.85 + (t * 3.4).sin().abs() * 0.15,
                    2 => (t * 4.6 + 2.4).sin().abs() * 0.8 + (t * 6.1).cos().abs() * 0.2,
                    _ => (t * 6.5 + 0.8).sin().abs() * 0.75 + (t * 1.9).sin().abs() * 0.25,
                };
                let bar_h = (3.5 + (max_h - 3.5) * wave).clamp(3.0, max_h);
                let y = (height as f64 - bar_h) / 2.0;

                let r = bar_w / 2.0;
                cr.new_sub_path();
                let _ = cr.arc(x + r, y + r, r, std::f64::consts::PI, 0.0);
                let _ = cr.arc(x + r, y + bar_h - r, r, 0.0, std::f64::consts::PI);
                let _ = cr.close_path();
                let _ = cr.fill();
            }
        });

        // 40ms animation ticker (~25 fps), only runs when media is playing and resting
        let phase_ticker = Rc::clone(&animation_phase);
        let is_animating_ticker = Rc::clone(&is_animating);
        let eq_area_ticker = eq_area.clone();

        glib::timeout_add_local(Duration::from_millis(40), move || {
            if *is_animating_ticker.borrow() {
                *phase_ticker.borrow_mut() += 0.05;
                eq_area_ticker.queue_draw();
            }
            glib::ControlFlow::Continue
        });

        let is_hovered = Rc::new(RefCell::new(false));

        // Media playback state monitor on resting pill (every 250ms)
        let left_media_box_monitor = left_media_box.clone();
        let right_media_box_monitor = right_media_box.clone();
        let left_icon_monitor = left_media_icon.clone();
        let is_hovered_media = Rc::clone(&is_hovered);
        let is_animating_media = Rc::clone(&is_animating);
        let spotify_media = Rc::clone(&player_is_spotify);
        let pill_box_media = pill_box.clone();

        glib::timeout_add_local(Duration::from_millis(250), move || {
            let media = crate::modules::mpris::get_current_active_media();
            let is_playing = media.as_ref().map(|m| m.is_playing).unwrap_or(false);
            let hovered = *is_hovered_media.borrow();

            let should_show = is_playing && !hovered;

            if should_show {
                if let Some(ref info) = media {
                    let is_spotify = info.player_display_name.to_lowercase().contains("spotify");
                    *spotify_media.borrow_mut() = is_spotify;
                    if is_spotify {
                        left_icon_monitor.set_text("🟢");
                    } else {
                        left_icon_monitor.set_text("🎵");
                    }
                }
                left_media_box_monitor.set_visible(true);
                right_media_box_monitor.set_visible(true);
                pill_box_media.add_css_class("media-active");
                *is_animating_media.borrow_mut() = true;
            } else {
                left_media_box_monitor.set_visible(false);
                right_media_box_monitor.set_visible(false);
                pill_box_media.remove_css_class("media-active");
                *is_animating_media.borrow_mut() = false;
            }

            glib::ControlFlow::Continue
        });

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

        let revealer_enter = revealer.clone();
        let is_hovered_enter = Rc::clone(&is_hovered);
        let window_enter = window.clone();
        let cfg_enter = config.clone();
        let left_media_enter = left_media_box.clone();
        let right_media_enter = right_media_box.clone();
        let is_animating_enter = Rc::clone(&is_animating);

        motion_ctrl.connect_enter(move |_, _x, _y| {
            *is_hovered_enter.borrow_mut() = true;
            left_media_enter.set_visible(false);
            right_media_enter.set_visible(false);
            *is_animating_enter.borrow_mut() = false;
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
        let left_media_leave = left_media_box.clone();
        let right_media_leave = right_media_box.clone();
        let is_animating_leave = Rc::clone(&is_animating);

        motion_ctrl.connect_leave(move |_| {
            let is_hovered_check = Rc::clone(&is_hovered_leave);
            let revealer_check = revealer_leave.clone();
            let window_check = window_leave.clone();
            let cfg_check = cfg_leave.clone();
            let ctrl_check = motion_ctrl_leave.clone();
            let left_media_check = left_media_leave.clone();
            let right_media_check = right_media_leave.clone();
            let is_animating_check = Rc::clone(&is_animating_leave);

            // Debounce before collapsing
            glib::timeout_add_local_once(
                Duration::from_millis(cfg_leave.general.hover_debounce_ms),
                move || {
                    // If pointer is still inside the notch (or clicking interactive children), do not close
                    if ctrl_check.contains_pointer() {
                        *is_hovered_check.borrow_mut() = true;
                        return;
                    }

                    // If user is actively typing in a text field (e.g. notes scratchpad), do not close
                    let has_text_focus = gtk4::prelude::GtkWindowExt::focus(&window_check)
                        .map(|w| w.is::<gtk4::TextView>() || w.is::<gtk4::Entry>())
                        .unwrap_or(false);
                    if has_text_focus {
                        *is_hovered_check.borrow_mut() = true;
                        return;
                    }

                    *is_hovered_check.borrow_mut() = false;
                    revealer_check.set_reveal_child(false);

                    // Resume pill media animation if media is playing
                    let media = crate::modules::mpris::get_current_active_media();
                    if media.as_ref().map(|m| m.is_playing).unwrap_or(false) {
                        left_media_check.set_visible(true);
                        right_media_check.set_visible(true);
                        *is_animating_check.borrow_mut() = true;
                    }

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

        // When notch window loses focus to an external window and pointer is not inside, collapse
        let is_hovered_active = Rc::clone(&is_hovered);
        let revealer_active = revealer.clone();
        let window_active = window.clone();
        let cfg_active = config.clone();
        let ctrl_active = motion_ctrl.clone();
        let left_media_active = left_media_box.clone();
        let right_media_active = right_media_box.clone();
        let is_animating_active = Rc::clone(&is_animating);

        window.connect_is_active_notify(move |win| {
            if !win.is_active() && !ctrl_active.contains_pointer() {
                *is_hovered_active.borrow_mut() = false;
                revealer_active.set_reveal_child(false);

                let media = crate::modules::mpris::get_current_active_media();
                if media.as_ref().map(|m| m.is_playing).unwrap_or(false) {
                    left_media_active.set_visible(true);
                    right_media_active.set_visible(true);
                    *is_animating_active.borrow_mut() = true;
                }

                super::position::update_notch_input_region(
                    &window_active,
                    false,
                    cfg_active.general.collapsed_width,
                    cfg_active.general.collapsed_height,
                    cfg_active.general.expanded_width,
                    cfg_active.general.expanded_height,
                );
            }
        });

        // Escape key to dismiss/collapse the notch
        let key_ctrl = gtk4::EventControllerKey::new();
        let revealer_esc = revealer.clone();
        let window_esc = window.clone();
        let cfg_esc = config.clone();
        let is_hovered_esc = Rc::clone(&is_hovered);
        let left_media_esc = left_media_box.clone();
        let right_media_esc = right_media_box.clone();
        let is_animating_esc = Rc::clone(&is_animating);

        key_ctrl.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                *is_hovered_esc.borrow_mut() = false;
                revealer_esc.set_reveal_child(false);

                let media = crate::modules::mpris::get_current_active_media();
                if media.as_ref().map(|m| m.is_playing).unwrap_or(false) {
                    left_media_esc.set_visible(true);
                    right_media_esc.set_visible(true);
                    *is_animating_esc.borrow_mut() = true;
                }

                super::position::update_notch_input_region(
                    &window_esc,
                    false,
                    cfg_esc.general.collapsed_width,
                    cfg_esc.general.collapsed_height,
                    cfg_esc.general.expanded_width,
                    cfg_esc.general.expanded_height,
                );
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });

        root_box.add_controller(motion_ctrl);
        root_box.add_controller(scroll_ctrl);
        root_box.add_controller(key_ctrl);

        Self { root_box }
    }
}
