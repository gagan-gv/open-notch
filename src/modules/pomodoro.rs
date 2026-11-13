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
        let container = GtkBox::new(Orientation::Vertical, 10);
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
        container.append(&controls);

        // Timer Tick
        let state_tick = Rc::clone(&state);
        let timer_label_tick = timer_label.clone();
        let mode_label_tick = mode_label.clone();
        let toggle_btn_tick = toggle_btn.clone();
        let switch_mode_btn_tick = switch_mode_btn.clone();

        glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
            let mut s = state_tick.borrow_mut();
            if s.is_running {
                if s.remaining_seconds > 0 {
                    s.remaining_seconds -= 1;
                    timer_label_tick.set_text(&format_time(s.remaining_seconds));
                } else {
                    // Time up! Auto switch mode
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
                }
            }
            glib::ControlFlow::Continue
        });

        // Toggle action
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

        // Switch mode action
        let state_switch = Rc::clone(&state);
        let mode_label_switch = mode_label.clone();
        let timer_label_switch = timer_label.clone();
        let toggle_btn_switch = toggle_btn.clone();
        let switch_btn_label = switch_mode_btn.clone();
        switch_mode_btn.connect_clicked(move |_| {
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
        });

        Self { container }
    }
}

fn format_time(total_seconds: u32) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}
