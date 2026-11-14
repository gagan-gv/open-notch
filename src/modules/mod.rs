pub mod calendar;
pub mod calendar_sync;
pub mod clipboard;
pub mod dashboard;
pub mod drop_shelf;
pub mod mpris;
pub mod notes;
pub mod pomodoro;

use calendar::CalendarWidget;
use clipboard::ClipboardWidget;
use dashboard::DashboardWidget;
use drop_shelf::DropShelfWidget;
use mpris::MprisWidget;
use notes::NotesWidget;
use pomodoro::PomodoroWidget;

use crate::config::Config;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, Stack};
use std::rc::Rc;
use std::time::Duration;

pub struct ModulesContainer {
    pub container: GtkBox,
    #[allow(dead_code)]
    pub mpris: MprisWidget,
    pub drop_shelf: DropShelfWidget,
    #[allow(dead_code)]
    pub clipboard: ClipboardWidget,
    #[allow(dead_code)]
    pub stack: Stack,
    switch_to_drops_fn: Rc<dyn Fn()>,
}

impl ModulesContainer {
    pub fn new(config: &Config) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.add_css_class("modules-container");

        let mpris_widget = MprisWidget::new();
        let drop_shelf_widget = DropShelfWidget::new();
        let clipboard_widget = ClipboardWidget::new();

        // Start system clipboard monitor
        let clip_refresh_bg = clipboard_widget.refresh_ui_fn.clone();
        clipboard::start_clipboard_monitor(move || {
            clip_refresh_bg();
        });

        // ------------------------------------------------------------------
        // Top Navigation & Status Bar (Matching Screenshot)
        // ------------------------------------------------------------------
        let top_bar = GtkBox::new(Orientation::Horizontal, 8);
        top_bar.add_css_class("notch-top-bar");

        // Top Left: Quick View Switchers [ 🏠 Dashboard ] [ 📥 Drops ] [ 📋 Clipboard ] [ ⏳ Focus ] [ 📝 Notes ]
        let nav_box = GtkBox::new(Orientation::Horizontal, 6);
        nav_box.set_halign(Align::Start);
        nav_box.set_valign(Align::Center);

        let home_btn = Button::with_label("🏠");
        home_btn.add_css_class("top-nav-btn");
        home_btn.add_css_class("top-nav-active");
        home_btn.set_tooltip_text(Some("Dashboard (Media & Schedule)"));

        let drops_btn = Button::with_label("📥");
        drops_btn.add_css_class("top-nav-btn");
        drops_btn.set_tooltip_text(Some("Drop Shelf (File Transit Tray)"));

        let clip_btn = Button::with_label("📋");
        clip_btn.add_css_class("top-nav-btn");
        clip_btn.set_tooltip_text(Some("Clipboard History"));

        let focus_btn = Button::with_label("⏳");
        focus_btn.add_css_class("top-nav-btn");
        focus_btn.set_tooltip_text(Some("Focus Timer"));

        let notes_btn = Button::with_label("📝");
        notes_btn.add_css_class("top-nav-btn");
        notes_btn.set_tooltip_text(Some("Quick Notes"));

        nav_box.append(&home_btn);
        nav_box.append(&drops_btn);
        nav_box.append(&clip_btn);
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

        // Drop Shelf file count badge updater on drops_btn (every 500ms)
        let drops_btn_badge = drops_btn.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            let count = drop_shelf::get_shelf_files().len();
            if count > 0 {
                drops_btn_badge.set_label(&format!("📥 {}", count));
                drops_btn_badge.add_css_class("has-drops");
            } else {
                drops_btn_badge.set_label("📥");
                drops_btn_badge.remove_css_class("has-drops");
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

        // 1. Dashboard View
        let dashboard_widget = DashboardWidget::new(&mpris_widget);
        stack.add_named(&dashboard_widget.container, Some("dashboard"));

        // 2. Drop Shelf View (File Transit Tray)
        stack.add_named(&drop_shelf_widget.container, Some("drops"));

        // 3. Clipboard History View
        stack.add_named(&clipboard_widget.container, Some("clipboard"));

        // 4. Focus / Pomodoro View
        let pomodoro_widget = PomodoroWidget::new(
            config.pomodoro.work_duration_min,
            config.pomodoro.break_duration_min,
        );
        stack.add_named(&pomodoro_widget.container, Some("pomodoro"));

        // 5. Notes View
        let notes_widget = NotesWidget::new();
        stack.add_named(&notes_widget.container, Some("notes"));

        // 6. Standalone Full Calendar view (optional)
        let calendar_widget = CalendarWidget::new();
        stack.add_named(&calendar_widget.container, Some("calendar"));

        // Set default screen to dashboard (matching screenshot)
        stack.set_visible_child_name("dashboard");

        // Nav button click handlers
        let stack_home = stack.clone();
        let hb = home_btn.clone();
        let db = drops_btn.clone();
        let cb = clip_btn.clone();
        let fb = focus_btn.clone();
        let nb = notes_btn.clone();
        home_btn.connect_clicked(move |_| {
            stack_home.set_visible_child_name("dashboard");
            hb.add_css_class("top-nav-active");
            db.remove_css_class("top-nav-active");
            cb.remove_css_class("top-nav-active");
            fb.remove_css_class("top-nav-active");
            nb.remove_css_class("top-nav-active");
        });

        let stack_drops = stack.clone();
        let hb_d = home_btn.clone();
        let db_d = drops_btn.clone();
        let cb_d = clip_btn.clone();
        let fb_d = focus_btn.clone();
        let nb_d = notes_btn.clone();
        let drop_shelf_refresh = drop_shelf_widget.refresh_ui_fn.clone();
        drops_btn.connect_clicked(move |_| {
            stack_drops.set_visible_child_name("drops");
            db_d.add_css_class("top-nav-active");
            hb_d.remove_css_class("top-nav-active");
            cb_d.remove_css_class("top-nav-active");
            fb_d.remove_css_class("top-nav-active");
            nb_d.remove_css_class("top-nav-active");
            drop_shelf_refresh();
        });

        let stack_clip = stack.clone();
        let hb_c = home_btn.clone();
        let db_c = drops_btn.clone();
        let cb_c = clip_btn.clone();
        let fb_c = focus_btn.clone();
        let nb_c = notes_btn.clone();
        let clip_refresh_click = clipboard_widget.refresh_ui_fn.clone();
        clip_btn.connect_clicked(move |_| {
            stack_clip.set_visible_child_name("clipboard");
            cb_c.add_css_class("top-nav-active");
            hb_c.remove_css_class("top-nav-active");
            db_c.remove_css_class("top-nav-active");
            fb_c.remove_css_class("top-nav-active");
            nb_c.remove_css_class("top-nav-active");
            clip_refresh_click();
        });

        let stack_focus = stack.clone();
        let hb2 = home_btn.clone();
        let db2 = drops_btn.clone();
        let cb2 = clip_btn.clone();
        let fb2 = focus_btn.clone();
        let nb2 = notes_btn.clone();
        focus_btn.connect_clicked(move |_| {
            stack_focus.set_visible_child_name("pomodoro");
            fb2.add_css_class("top-nav-active");
            hb2.remove_css_class("top-nav-active");
            db2.remove_css_class("top-nav-active");
            cb2.remove_css_class("top-nav-active");
            nb2.remove_css_class("top-nav-active");
        });

        let stack_notes = stack.clone();
        let hb3 = home_btn.clone();
        let db3 = drops_btn.clone();
        let cb3 = clip_btn.clone();
        let fb3 = focus_btn.clone();
        let nb3 = notes_btn.clone();
        let notes_tv = notes_widget.text_view.clone();
        notes_btn.connect_clicked(move |_| {
            stack_notes.set_visible_child_name("notes");
            nb3.add_css_class("top-nav-active");
            hb3.remove_css_class("top-nav-active");
            db3.remove_css_class("top-nav-active");
            cb3.remove_css_class("top-nav-active");
            fb3.remove_css_class("top-nav-active");
            notes_tv.grab_focus();
            crate::window::position::grab_x11_input_focus();
        });

        // Switch to drops closure for external caller (e.g. pill drag-over)
        let stack_ext = stack.clone();
        let hb_ext = home_btn.clone();
        let db_ext = drops_btn.clone();
        let cb_ext = clip_btn.clone();
        let fb_ext = focus_btn.clone();
        let nb_ext = notes_btn.clone();
        let switch_to_drops_fn: Rc<dyn Fn()> = Rc::new(move || {
            stack_ext.set_visible_child_name("drops");
            db_ext.add_css_class("top-nav-active");
            hb_ext.remove_css_class("top-nav-active");
            cb_ext.remove_css_class("top-nav-active");
            fb_ext.remove_css_class("top-nav-active");
            nb_ext.remove_css_class("top-nav-active");
        });

        container.append(&top_bar);
        container.append(&stack);

        Self {
            container,
            mpris: mpris_widget,
            drop_shelf: drop_shelf_widget,
            clipboard: clipboard_widget,
            stack,
            switch_to_drops_fn,
        }
    }

    pub fn switch_to_drops(&self) {
        (self.switch_to_drops_fn)();
    }
}
