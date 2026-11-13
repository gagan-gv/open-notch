use crate::config::Config;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PomodoroMode {
    Work,
    Break,
}

struct PomodoroState {
    mode: PomodoroMode,
    is_running: bool,
    remaining_seconds: u32,
    work_seconds: u32,
    break_seconds: u32,
}

pub struct PomodoroWidget {
    pub container: GtkBox,
}

impl PomodoroWidget {
    pub fn new(work_min: u32, break_min: u32) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("pomodoro-widget");

        let state = Rc::new(RefCell::new(PomodoroState {
            mode: PomodoroMode::Work,
            is_running: false,
            remaining_seconds: work_min * 60,
            work_seconds: work_min * 60,
            break_seconds: break_min * 60,
        }));

        let mode_label = Label::new(Some("⚡ Focus Session"));
        mode_label.add_css_class("pomodoro-mode-label");

        let timer_label = Label::new(Some(&format_time(work_min * 60)));
        timer_label.add_css_class("pomodoro-timer-label");

        // Presets & Duration Adjusters Box
        let presets_box = GtkBox::new(Orientation::Horizontal, 6);
        presets_box.set_halign(Align::Center);
        presets_box.add_css_class("pomodoro-presets-box");

        let controls = GtkBox::new(Orientation::Horizontal, 12);
        controls.set_halign(Align::Center);

        let toggle_btn = Button::with_label("Start");
        toggle_btn.add_css_class("pomodoro-btn-primary");

        let reset_btn = Button::with_label("Reset");
        reset_btn.add_css_class("pomodoro-btn-secondary");

        let switch_mode_btn = Button::with_label("Switch to Break");
        switch_mode_btn.add_css_class("pomodoro-btn-secondary");

        controls.append(&toggle_btn);
        controls.append(&reset_btn);
        controls.append(&switch_mode_btn);

        container.append(&mode_label);
        container.append(&timer_label);
        container.append(&presets_box);
        container.append(&controls);

        // Helper to rebuild presets bar based on current mode
        let refresh_presets = {
            let presets_box = presets_box.clone();
            let state = Rc::clone(&state);
            let timer_label = timer_label.clone();
            let toggle_btn = toggle_btn.clone();

            Rc::new(move || {
                // Clear existing buttons
                while let Some(child) = presets_box.first_child() {
                    presets_box.remove(&child);
                }

                let current_mode = state.borrow().mode;
                let (presets, step, min_val, max_val, label_step) = match current_mode {
                    PomodoroMode::Work => (vec![15, 25, 45, 60], 5, 5, 120, "-5m"),
                    PomodoroMode::Break => (vec![3, 5, 10, 15], 1, 1, 60, "-1m"),
                };

                let step_plus_label = match current_mode {
                    PomodoroMode::Work => "+5m",
                    PomodoroMode::Break => "+1m",
                };

                // Minus button
                let minus_btn = Button::with_label(label_step);
                minus_btn.add_css_class("pomodoro-adjust-btn");
                let state_minus = Rc::clone(&state);
                let timer_label_minus = timer_label.clone();
                let toggle_btn_minus = toggle_btn.clone();
                let presets_box_minus = presets_box.clone();

                minus_btn.connect_clicked(move |_| {
                    let mut s = state_minus.borrow_mut();
                    if s.mode == PomodoroMode::Work {
                        if s.work_seconds > (min_val * 60) + (step * 60) {
                            s.work_seconds -= step * 60;
                        } else {
                            s.work_seconds = min_val * 60;
                        }
                        if !s.is_running {
                            s.remaining_seconds = s.work_seconds;
                            timer_label_minus.set_text(&format_time(s.remaining_seconds));
                        }
                    } else {
                        if s.break_seconds > (min_val * 60) + (step * 60) {
                            s.break_seconds -= step * 60;
                        } else {
                            s.break_seconds = min_val * 60;
                        }
                        if !s.is_running {
                            s.remaining_seconds = s.break_seconds;
                            timer_label_minus.set_text(&format_time(s.remaining_seconds));
                        }
                    }
                    toggle_btn_minus.set_label("Start");
                    s.is_running = false;
                    Config::update_pomodoro_durations(s.work_seconds / 60, s.break_seconds / 60);

                    // Update active button styles
                    update_preset_highlights(&presets_box_minus, &s);
                });
                presets_box.append(&minus_btn);

                // Preset Buttons
                for val in presets {
                    let btn = Button::with_label(&format!("{}m", val));
                    btn.add_css_class("pomodoro-preset-btn");

                    let current_duration = match current_mode {
                        PomodoroMode::Work => state.borrow().work_seconds / 60,
                        PomodoroMode::Break => state.borrow().break_seconds / 60,
                    };
                    if current_duration == val {
                        btn.add_css_class("pomodoro-preset-active");
                    }

                    let state_preset = Rc::clone(&state);
                    let timer_label_preset = timer_label.clone();
                    let toggle_btn_preset = toggle_btn.clone();
                    let presets_box_preset = presets_box.clone();

                    btn.connect_clicked(move |_| {
                        let mut s = state_preset.borrow_mut();
                        s.is_running = false;
                        toggle_btn_preset.set_label("Start");

                        if s.mode == PomodoroMode::Work {
                            s.work_seconds = val * 60;
                            s.remaining_seconds = s.work_seconds;
                        } else {
                            s.break_seconds = val * 60;
                            s.remaining_seconds = s.break_seconds;
                        }

                        timer_label_preset.set_text(&format_time(s.remaining_seconds));
                        Config::update_pomodoro_durations(s.work_seconds / 60, s.break_seconds / 60);
                        update_preset_highlights(&presets_box_preset, &s);
                    });

                    presets_box.append(&btn);
                }

                // Plus button
                let plus_btn = Button::with_label(step_plus_label);
                plus_btn.add_css_class("pomodoro-adjust-btn");
                let state_plus = Rc::clone(&state);
                let timer_label_plus = timer_label.clone();
                let toggle_btn_plus = toggle_btn.clone();
                let presets_box_plus = presets_box.clone();

                plus_btn.connect_clicked(move |_| {
                    let mut s = state_plus.borrow_mut();
                    if s.mode == PomodoroMode::Work {
                        s.work_seconds = (s.work_seconds + step * 60).min(max_val * 60);
                        if !s.is_running {
                            s.remaining_seconds = s.work_seconds;
                            timer_label_plus.set_text(&format_time(s.remaining_seconds));
                        }
                    } else {
                        s.break_seconds = (s.break_seconds + step * 60).min(max_val * 60);
                        if !s.is_running {
                            s.remaining_seconds = s.break_seconds;
                            timer_label_plus.set_text(&format_time(s.remaining_seconds));
                        }
                    }
                    toggle_btn_plus.set_label("Start");
                    s.is_running = false;
                    Config::update_pomodoro_durations(s.work_seconds / 60, s.break_seconds / 60);
                    update_preset_highlights(&presets_box_plus, &s);
                });
                presets_box.append(&plus_btn);
            })
        };

        // Initialize presets
        refresh_presets();

        // Timer Tick (1-second heartbeat)
        let state_tick = Rc::clone(&state);
        let timer_label_tick = timer_label.clone();
        let mode_label_tick = mode_label.clone();
        let toggle_btn_tick = toggle_btn.clone();
        let switch_mode_btn_tick = switch_mode_btn.clone();
        let refresh_presets_tick = Rc::clone(&refresh_presets);

        glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
            let mut time_is_up = false;
            let mut new_mode = PomodoroMode::Work;

            {
                let mut s = state_tick.borrow_mut();
                if s.is_running {
                    if s.remaining_seconds > 0 {
                        s.remaining_seconds -= 1;
                        timer_label_tick.set_text(&format_time(s.remaining_seconds));
                    } else {
                        // Time up! Auto switch mode
                        time_is_up = true;
                        if s.mode == PomodoroMode::Work {
                            s.mode = PomodoroMode::Break;
                            s.remaining_seconds = s.break_seconds;
                            mode_label_tick.set_text("☕ Short Break");
                            switch_mode_btn_tick.set_label("Switch to Focus");
                        } else {
                            s.mode = PomodoroMode::Work;
                            s.remaining_seconds = s.work_seconds;
                            mode_label_tick.set_text("⚡ Focus Session");
                            switch_mode_btn_tick.set_label("Switch to Break");
                        }
                        s.is_running = false;
                        toggle_btn_tick.set_label("Start");
                        timer_label_tick.set_text(&format_time(s.remaining_seconds));
                        new_mode = s.mode;
                    }
                }
            } // RefCell borrow_mut is explicitly dropped here before calling UI refresh

            if time_is_up {
                refresh_presets_tick();

                // Send desktop notification when timer completes
                let (summary, body) = if new_mode == PomodoroMode::Break {
                    ("⚡ Focus Session Complete!", "Great job! Time for a short break.")
                } else {
                    ("☕ Break Finished!", "Ready to dive back into focus mode?")
                };
                let _ = std::process::Command::new("notify-send")
                    .args(["-a", "OpenNotch", summary, body])
                    .spawn();
            }

            glib::ControlFlow::Continue
        });

        // Toggle action (Start / Pause)
        let state_toggle = Rc::clone(&state);
        let toggle_btn_action = toggle_btn.clone();
        toggle_btn.connect_clicked(move |_| {
            let mut s = state_toggle.borrow_mut();
            s.is_running = !s.is_running;
            if s.is_running {
                toggle_btn_action.set_label("Pause");
            } else {
                toggle_btn_action.set_label("Start");
            }
        });

        // Reset action
        let state_reset = Rc::clone(&state);
        let timer_label_reset = timer_label.clone();
        let toggle_btn_reset = toggle_btn.clone();
        reset_btn.connect_clicked(move |_| {
            let mut s = state_reset.borrow_mut();
            s.is_running = false;
            s.remaining_seconds = if s.mode == PomodoroMode::Work {
                s.work_seconds
            } else {
                s.break_seconds
            };
            toggle_btn_reset.set_label("Start");
            timer_label_reset.set_text(&format_time(s.remaining_seconds));
        });

        // Switch mode action (Focus <-> Break)
        let state_switch = Rc::clone(&state);
        let mode_label_switch = mode_label.clone();
        let timer_label_switch = timer_label.clone();
        let toggle_btn_switch = toggle_btn.clone();
        let switch_btn_label = switch_mode_btn.clone();
        let refresh_presets_switch = Rc::clone(&refresh_presets);

        switch_mode_btn.connect_clicked(move |_| {
            {
                let mut s = state_switch.borrow_mut();
                s.is_running = false;
                toggle_btn_switch.set_label("Start");
                if s.mode == PomodoroMode::Work {
                    s.mode = PomodoroMode::Break;
                    s.remaining_seconds = s.break_seconds;
                    mode_label_switch.set_text("☕ Short Break");
                    switch_btn_label.set_label("Switch to Focus");
                } else {
                    s.mode = PomodoroMode::Work;
                    s.remaining_seconds = s.work_seconds;
                    mode_label_switch.set_text("⚡ Focus Session");
                    switch_btn_label.set_label("Switch to Break");
                }
                timer_label_switch.set_text(&format_time(s.remaining_seconds));
            } // RefCell borrow_mut is explicitly dropped here before calling UI refresh
            refresh_presets_switch();
        });

        Self { container }
    }
}

fn update_preset_highlights(presets_box: &GtkBox, state: &PomodoroState) {
    let current_val = match state.mode {
        PomodoroMode::Work => state.work_seconds / 60,
        PomodoroMode::Break => state.break_seconds / 60,
    };
    let target_label = format!("{}m", current_val);

    let mut next_child = presets_box.first_child();
    while let Some(child) = next_child {
        if let Some(btn) = child.downcast_ref::<Button>() {
            if btn.label().as_deref() == Some(&target_label) {
                btn.add_css_class("pomodoro-preset-active");
            } else {
                btn.remove_css_class("pomodoro-preset-active");
            }
        }
        next_child = child.next_sibling();
    }
}

fn format_time(total_seconds: u32) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
