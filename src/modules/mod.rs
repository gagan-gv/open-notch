pub mod calendar;
pub mod calendar_sync;
pub mod dashboard;
pub mod mpris;
pub mod notes;
pub mod pomodoro;

use calendar::CalendarWidget;
use dashboard::DashboardWidget;
use mpris::MprisWidget;
use notes::NotesWidget;
use pomodoro::PomodoroWidget;

use crate::config::Config;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Stack};
use std::time::Duration;

pub struct ModulesContainer {
    pub container: GtkBox,
    #[allow(dead_code)]
    pub mpris: MprisWidget,
}

impl ModulesContainer {
    pub fn new(config: &Config) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.add_css_class("modules-container");

        let mpris_widget = MprisWidget::new();

        // ------------------------------------------------------------------
        // Top Navigation & Status Bar (Matching Screenshot)
        // ------------------------------------------------------------------
        let top_bar = GtkBox::new(Orientation::Horizontal, 8);
        top_bar.add_css_class("notch-top-bar");

        // Top Left: Quick View Switchers [ 🏠 Dashboard ] [ ⏳ Focus ] [ 📝 Notes ]
        let nav_box = GtkBox::new(Orientation::Horizontal, 6);
        nav_box.set_halign(Align::Start);
        nav_box.set_valign(Align::Center);

        let home_btn = Button::with_label("🏠");
        home_btn.add_css_class("top-nav-btn");
        home_btn.add_css_class("top-nav-active");
        home_btn.set_tooltip_text(Some("Dashboard (Media & Schedule)"));

        let focus_btn = Button::with_label("⏳");
        focus_btn.add_css_class("top-nav-btn");
        focus_btn.set_tooltip_text(Some("Focus Timer"));

        let notes_btn = Button::with_label("📝");
        notes_btn.add_css_class("top-nav-btn");
        notes_btn.set_tooltip_text(Some("Quick Notes"));

        nav_box.append(&home_btn);
        nav_box.append(&focus_btn);
        nav_box.append(&notes_btn);

        // Top Right: Live Battery Indicator (e.g. 80% 🔋)
        let right_box = GtkBox::new(Orientation::Horizontal, 8);
        right_box.set_halign(Align::End);
        right_box.set_valign(Align::Center);
        right_box.set_hexpand(true);

        let battery_label = Label::new(Some("100% 🔋"));
        battery_label.add_css_class("top-battery-badge");

        // Read real battery info immediately
        if let Some((pct, charging)) = dashboard::get_battery_info() {
            let icon = if charging { "⚡" } else { "🔋" };
            battery_label.set_text(&format!("{}% {}", pct, icon));
        }

        // Live battery poller every 15 seconds
        let bat_update = battery_label.clone();
        glib::timeout_add_local(Duration::from_secs(15), move || {
            if let Some((pct, charging)) = dashboard::get_battery_info() {
                let icon = if charging { "⚡" } else { "🔋" };
                bat_update.set_text(&format!("{}% {}", pct, icon));
            }
            glib::ControlFlow::Continue
        });

        right_box.append(&battery_label);

        top_bar.append(&nav_box);
        top_bar.append(&right_box);

        // ------------------------------------------------------------------
        // Main Stack (Dashboard view by default)
        // ------------------------------------------------------------------
        let stack = Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(200);

        // 1. Dashboard View (The exact UI from the screenshot: Media + Week Strip + Events)
        let dashboard_widget = DashboardWidget::new(&mpris_widget);
        stack.add_named(&dashboard_widget.container, Some("dashboard"));

        // 2. Focus / Pomodoro View
        let pomodoro_widget = PomodoroWidget::new(
            config.pomodoro.work_duration_min,
            config.pomodoro.break_duration_min,
        );
        stack.add_named(&pomodoro_widget.container, Some("pomodoro"));

        // 3. Notes View
        let notes_widget = NotesWidget::new();
        stack.add_named(&notes_widget.container, Some("notes"));

        // 4. Standalone Full Calendar view (optional)
        let calendar_widget = CalendarWidget::new();
        stack.add_named(&calendar_widget.container, Some("calendar"));

        // Set default screen to dashboard (matching screenshot)
        stack.set_visible_child_name("dashboard");

        // Nav button click handlers
        let stack_home = stack.clone();
        let hb = home_btn.clone();
        let fb = focus_btn.clone();
        let nb = notes_btn.clone();
        home_btn.connect_clicked(move |_| {
            stack_home.set_visible_child_name("dashboard");
            hb.add_css_class("top-nav-active");
            fb.remove_css_class("top-nav-active");
            nb.remove_css_class("top-nav-active");
        });

        let stack_focus = stack.clone();
        let hb2 = home_btn.clone();
        let fb2 = focus_btn.clone();
        let nb2 = notes_btn.clone();
        focus_btn.connect_clicked(move |_| {
            stack_focus.set_visible_child_name("pomodoro");
            fb2.add_css_class("top-nav-active");
            hb2.remove_css_class("top-nav-active");
            nb2.remove_css_class("top-nav-active");
        });

        let stack_notes = stack.clone();
        let hb3 = home_btn.clone();
        let fb3 = focus_btn.clone();
        let nb3 = notes_btn.clone();
        notes_btn.connect_clicked(move |_| {
            stack_notes.set_visible_child_name("notes");
            nb3.add_css_class("top-nav-active");
            hb3.remove_css_class("top-nav-active");
            fb3.remove_css_class("top-nav-active");
        });

        container.append(&top_bar);
        container.append(&stack);

        Self {
            container,
            mpris: mpris_widget,
        }
    }
}
