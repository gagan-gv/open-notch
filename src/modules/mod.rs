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
use gtk4::{Align, Box as GtkBox, Button, Image, Label, Orientation, Separator, Stack, Switch};
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

        // Top Left: Quick View Switchers [ Home ] [ Drops ] [ Clipboard ] [ Focus ] [ Notes ]
        let nav_box = GtkBox::new(Orientation::Horizontal, 6);
        nav_box.set_halign(Align::Start);
        nav_box.set_valign(Align::Center);

        let home_icon = Image::from_icon_name("go-home-symbolic");
        home_icon.set_pixel_size(15);
        let home_btn = Button::new();
        home_btn.set_child(Some(&home_icon));
        home_btn.add_css_class("top-nav-btn");
        home_btn.add_css_class("top-nav-active");
        home_btn.set_tooltip_text(Some("Dashboard (Media & Schedule)"));

        let drops_box = GtkBox::new(Orientation::Horizontal, 4);
        drops_box.set_valign(Align::Center);
        let drops_icon = Image::from_icon_name("folder-download-symbolic");
        drops_icon.set_pixel_size(15);
        let drops_badge = Label::new(None);
        drops_badge.add_css_class("top-nav-badge");
        drops_badge.set_visible(false);
        drops_box.append(&drops_icon);
        drops_box.append(&drops_badge);
        let drops_btn = Button::new();
        drops_btn.set_child(Some(&drops_box));
        drops_btn.add_css_class("top-nav-btn");
        drops_btn.set_tooltip_text(Some("Drop Shelf (File Transit Tray)"));

        let clip_icon = Image::from_icon_name("edit-copy-symbolic");
        clip_icon.set_pixel_size(15);
        let clip_btn = Button::new();
        clip_btn.set_child(Some(&clip_icon));
        clip_btn.add_css_class("top-nav-btn");
        clip_btn.set_tooltip_text(Some("Clipboard History"));

        let focus_icon = Image::from_icon_name("alarm-symbolic");
        focus_icon.set_pixel_size(15);
        let focus_btn = Button::new();
        focus_btn.set_child(Some(&focus_icon));
        focus_btn.add_css_class("top-nav-btn");
        focus_btn.set_tooltip_text(Some("Focus Timer"));

        let notes_icon = Image::from_icon_name("document-edit-symbolic");
        notes_icon.set_pixel_size(15);
        let notes_btn = Button::new();
        notes_btn.set_child(Some(&notes_icon));
        notes_btn.add_css_class("top-nav-btn");
        notes_btn.set_tooltip_text(Some("Quick Notes"));

        nav_box.append(&home_btn);
        nav_box.append(&drops_btn);
        nav_box.append(&clip_btn);
        nav_box.append(&focus_btn);
        nav_box.append(&notes_btn);

        // Top Right: Live Battery Indicator with Symbolic Battery Icon
        let right_box = GtkBox::new(Orientation::Horizontal, 8);
        right_box.set_halign(Align::End);
        right_box.set_valign(Align::Center);
        right_box.set_hexpand(true);

        let battery_box = GtkBox::new(Orientation::Horizontal, 4);
        battery_box.set_halign(Align::End);
        battery_box.set_valign(Align::Center);
        battery_box.add_css_class("top-battery-badge");

        let battery_icon = Image::from_icon_name("battery-level-80-symbolic");
        battery_icon.set_pixel_size(14);
        battery_icon.add_css_class("battery-icon");

        let battery_label = Label::new(Some("100%"));
        battery_label.add_css_class("battery-text");

        battery_box.append(&battery_icon);
        battery_box.append(&battery_label);

        // Read real battery info immediately
        if let Some((pct, charging)) = dashboard::get_battery_info() {
            battery_label.set_text(&format!("{}%", pct));
            battery_icon.set_icon_name(Some(get_battery_icon_name(pct, charging)));
        }

        // Live battery poller every 15 seconds
        let bat_update_lbl = battery_label.clone();
        let bat_update_ico = battery_icon.clone();
        glib::timeout_add_local(Duration::from_secs(15), move || {
            if let Some((pct, charging)) = dashboard::get_battery_info() {
                bat_update_lbl.set_text(&format!("{}%", pct));
                bat_update_ico.set_icon_name(Some(get_battery_icon_name(pct, charging)));
            }
            glib::ControlFlow::Continue
        });

        // Drop Shelf file count badge updater on drops_btn (every 500ms)
        let drops_badge_poller = drops_badge.clone();
        let drops_btn_poller = drops_btn.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            let count = drop_shelf::get_shelf_files().len();
            if count > 0 {
                drops_badge_poller.set_text(&count.to_string());
                drops_badge_poller.set_visible(true);
                drops_btn_poller.add_css_class("has-drops");
            } else {
                drops_badge_poller.set_visible(false);
                drops_btn_poller.remove_css_class("has-drops");
            }
            glib::ControlFlow::Continue
        });

        right_box.append(&battery_box);

        // Settings Button near battery icon
        let settings_icon = Image::from_icon_name("preferences-system-symbolic");
        settings_icon.set_pixel_size(14);
        let settings_btn = Button::new();
        settings_btn.set_child(Some(&settings_icon));
        settings_btn.add_css_class("top-settings-btn");
        settings_btn.set_tooltip_text(Some("Settings & Preferences"));

        right_box.append(&settings_btn);

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

        // 7. Settings View (In-shelf Preferences)
        let settings_view = build_settings_view(&settings_btn);
        stack.add_named(&settings_view, Some("settings"));

        // Set default screen to dashboard (matching screenshot)
        stack.set_visible_child_name("dashboard");

        // Nav button click handlers
        let stack_home = stack.clone();
        let hb = home_btn.clone();
        let db = drops_btn.clone();
        let cb = clip_btn.clone();
        let fb = focus_btn.clone();
        let nb = notes_btn.clone();
        let sb = settings_btn.clone();
        home_btn.connect_clicked(move |_| {
            stack_home.set_visible_child_name("dashboard");
            hb.add_css_class("top-nav-active");
            db.remove_css_class("top-nav-active");
            cb.remove_css_class("top-nav-active");
            fb.remove_css_class("top-nav-active");
            nb.remove_css_class("top-nav-active");
            sb.remove_css_class("top-nav-active");
        });

        let stack_drops = stack.clone();
        let hb_d = home_btn.clone();
        let db_d = drops_btn.clone();
        let cb_d = clip_btn.clone();
        let fb_d = focus_btn.clone();
        let nb_d = notes_btn.clone();
        let sb_d = settings_btn.clone();
        let drop_shelf_refresh = drop_shelf_widget.refresh_ui_fn.clone();
        drops_btn.connect_clicked(move |_| {
            stack_drops.set_visible_child_name("drops");
            db_d.add_css_class("top-nav-active");
            hb_d.remove_css_class("top-nav-active");
            cb_d.remove_css_class("top-nav-active");
            fb_d.remove_css_class("top-nav-active");
            nb_d.remove_css_class("top-nav-active");
            sb_d.remove_css_class("top-nav-active");
            drop_shelf_refresh();
        });

        let stack_clip = stack.clone();
        let hb_c = home_btn.clone();
        let db_c = drops_btn.clone();
        let cb_c = clip_btn.clone();
        let fb_c = focus_btn.clone();
        let nb_c = notes_btn.clone();
        let sb_c = settings_btn.clone();
        let clip_refresh_click = clipboard_widget.refresh_ui_fn.clone();
        clip_btn.connect_clicked(move |_| {
            stack_clip.set_visible_child_name("clipboard");
            cb_c.add_css_class("top-nav-active");
            hb_c.remove_css_class("top-nav-active");
            db_c.remove_css_class("top-nav-active");
            fb_c.remove_css_class("top-nav-active");
            nb_c.remove_css_class("top-nav-active");
            sb_c.remove_css_class("top-nav-active");
            clip_refresh_click();
        });

        let stack_focus = stack.clone();
        let hb2 = home_btn.clone();
        let db2 = drops_btn.clone();
        let cb2 = clip_btn.clone();
        let fb2 = focus_btn.clone();
        let nb2 = notes_btn.clone();
        let sb2 = settings_btn.clone();
        focus_btn.connect_clicked(move |_| {
            stack_focus.set_visible_child_name("pomodoro");
            fb2.add_css_class("top-nav-active");
            hb2.remove_css_class("top-nav-active");
            db2.remove_css_class("top-nav-active");
            cb2.remove_css_class("top-nav-active");
            nb2.remove_css_class("top-nav-active");
            sb2.remove_css_class("top-nav-active");
        });

        let stack_notes = stack.clone();
        let hb3 = home_btn.clone();
        let db3 = drops_btn.clone();
        let cb3 = clip_btn.clone();
        let fb3 = focus_btn.clone();
        let nb3 = notes_btn.clone();
        let sb3 = settings_btn.clone();
        let notes_tv = notes_widget.text_view.clone();
        notes_btn.connect_clicked(move |_| {
            stack_notes.set_visible_child_name("notes");
            nb3.add_css_class("top-nav-active");
            hb3.remove_css_class("top-nav-active");
            db3.remove_css_class("top-nav-active");
            cb3.remove_css_class("top-nav-active");
            fb3.remove_css_class("top-nav-active");
            sb3.remove_css_class("top-nav-active");
            notes_tv.grab_focus();
            crate::window::position::grab_x11_input_focus();
        });

        // Settings button click handler
        let stack_set = stack.clone();
        let hb_s = home_btn.clone();
        let db_s = drops_btn.clone();
        let cb_s = clip_btn.clone();
        let fb_s = focus_btn.clone();
        let nb_s = notes_btn.clone();
        let sb_s = settings_btn.clone();
        settings_btn.connect_clicked(move |_| {
            if stack_set.visible_child_name().as_deref() == Some("settings") {
                stack_set.set_visible_child_name("dashboard");
                sb_s.remove_css_class("top-nav-active");
                hb_s.add_css_class("top-nav-active");
            } else {
                stack_set.set_visible_child_name("settings");
                sb_s.add_css_class("top-nav-active");
                hb_s.remove_css_class("top-nav-active");
                db_s.remove_css_class("top-nav-active");
                cb_s.remove_css_class("top-nav-active");
                fb_s.remove_css_class("top-nav-active");
                nb_s.remove_css_class("top-nav-active");
            }
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

fn get_battery_icon_name(pct: u32, charging: bool) -> &'static str {
    if charging {
        if pct >= 90 {
            "battery-level-100-charging-symbolic"
        } else if pct >= 70 {
            "battery-level-80-charging-symbolic"
        } else if pct >= 50 {
            "battery-level-60-charging-symbolic"
        } else if pct >= 30 {
            "battery-level-30-charging-symbolic"
        } else {
            "battery-level-10-charging-symbolic"
        }
    } else {
        if pct >= 90 {
            "battery-level-100-symbolic"
        } else if pct >= 70 {
            "battery-level-80-symbolic"
        } else if pct >= 50 {
            "battery-level-60-symbolic"
        } else if pct >= 30 {
            "battery-level-30-symbolic"
        } else {
            "battery-level-10-symbolic"
        }
    }
}

fn build_settings_view(settings_btn: &Button) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 10);
    container.set_valign(Align::Center);
    container.set_halign(Align::Center);
    container.set_hexpand(true);
    container.set_vexpand(true);
    container.set_size_request(450, -1);
    container.add_css_class("settings-view-card");

    // 1. Header: [ Preferences ] [ v0.2.0 ]
    let header_row = GtkBox::new(Orientation::Horizontal, 8);
    header_row.set_valign(Align::Center);
    header_row.set_halign(Align::Fill);

    let title_icon = Image::from_icon_name("preferences-system-symbolic");
    title_icon.set_pixel_size(16);
    title_icon.add_css_class("settings-header-icon");

    let title_lbl = Label::new(Some("OpenNotch Preferences"));
    title_lbl.add_css_class("settings-header-title");
    title_lbl.set_halign(Align::Start);
    title_lbl.set_hexpand(true);

    let cur_ver_str = format!("v{}", crate::updater::CURRENT_VERSION);
    let version_badge = Label::new(Some(&cur_ver_str));
    version_badge.add_css_class("settings-version-badge");

    header_row.append(&title_icon);
    header_row.append(&title_lbl);
    header_row.append(&version_badge);

    // Separator
    let sep1 = Separator::new(Orientation::Horizontal);
    sep1.add_css_class("settings-separator");

    // 2. Autostart Setting Row (Enabled by default)
    let autostart_row = GtkBox::new(Orientation::Horizontal, 12);
    autostart_row.set_valign(Align::Center);
    autostart_row.set_halign(Align::Fill);
    autostart_row.add_css_class("settings-item-row");

    let auto_icon = Image::from_icon_name("system-run-symbolic");
    auto_icon.set_pixel_size(16);
    auto_icon.add_css_class("settings-row-icon");

    let text_col = GtkBox::new(Orientation::Vertical, 2);
    text_col.set_halign(Align::Start);
    text_col.set_hexpand(true);

    let auto_title = Label::new(Some("Start at Login"));
    auto_title.add_css_class("settings-row-label");
    auto_title.set_halign(Align::Start);

    let auto_desc = Label::new(Some("Launch OpenNotch automatically when you log in"));
    auto_desc.add_css_class("settings-row-sub");
    auto_desc.set_halign(Align::Start);

    text_col.append(&auto_title);
    text_col.append(&auto_desc);

    let autostart_switch = Switch::new();
    autostart_switch.set_valign(Align::Center);
    autostart_switch.add_css_class("settings-switch");
    let is_on = crate::config::is_autostart_enabled();
    autostart_switch.set_active(is_on);

    autostart_switch.connect_state_set(|_, active| {
        crate::config::set_autostart_enabled(active);
        glib::Propagation::Proceed
    });

    autostart_row.append(&auto_icon);
    autostart_row.append(&text_col);
    autostart_row.append(&autostart_switch);

    // Separator
    let sep2 = Separator::new(Orientation::Horizontal);
    sep2.add_css_class("settings-separator");

    // 3. Updates Row
    let update_row = GtkBox::new(Orientation::Horizontal, 12);
    update_row.set_valign(Align::Center);
    update_row.set_halign(Align::Fill);
    update_row.add_css_class("settings-item-row");

    let update_icon = Image::from_icon_name("software-update-available-symbolic");
    update_icon.set_pixel_size(16);
    update_icon.add_css_class("settings-row-icon");

    let update_text_col = GtkBox::new(Orientation::Vertical, 2);
    update_text_col.set_halign(Align::Start);
    update_text_col.set_hexpand(true);

    let update_title = Label::new(Some("Software Updates"));
    update_title.add_css_class("settings-row-label");
    update_title.set_halign(Align::Start);

    let default_status = format!("v{} • Up to date", crate::updater::CURRENT_VERSION);
    let update_sub = Label::new(Some(&default_status));
    update_sub.add_css_class("settings-row-sub");
    update_sub.set_halign(Align::Start);

    update_text_col.append(&update_title);
    update_text_col.append(&update_sub);

    let check_btn = Button::with_label("Check for Updates");
    check_btn.set_valign(Align::Center);
    check_btn.add_css_class("settings-update-btn");

    update_row.append(&update_icon);
    update_row.append(&update_text_col);
    update_row.append(&check_btn);

    // Action button row (dynamically populated when update found)
    let details_row = GtkBox::new(Orientation::Horizontal, 8);
    details_row.set_halign(Align::End);
    details_row.set_visible(false);

    // Check updates callback
    let s_lbl = update_sub.clone();
    let c_btn = check_btn.clone();
    let d_row = details_row.clone();
    let top_btn = settings_btn.clone();

    let trigger_check = std::rc::Rc::new(move || {
        c_btn.set_sensitive(false);
        c_btn.set_label("Checking...");
        s_lbl.set_text("Checking GitHub for updates...");
        s_lbl.remove_css_class("settings-update-highlight");

        let s_lbl_clone = s_lbl.clone();
        let c_btn_clone = c_btn.clone();
        let d_row_clone = d_row.clone();
        let top_btn_clone = top_btn.clone();

        let (tx, rx) = std::sync::mpsc::channel::<crate::updater::UpdateStatus>();

        std::thread::spawn(move || {
            let status = crate::updater::check_updates_sync();
            let _ = tx.send(status);
        });

        glib::timeout_add_local(std::time::Duration::from_millis(80), move || {
            match rx.try_recv() {
                Ok(status) => {
                    c_btn_clone.set_sensitive(true);
                    c_btn_clone.set_label("Check for Updates");

                    match status {
                        crate::updater::UpdateStatus::UpToDate(ver) => {
                            s_lbl_clone.set_text(&format!("v{} • You're on the latest version", ver));
                            s_lbl_clone.remove_css_class("settings-update-highlight");
                            top_btn_clone.remove_css_class("has-update");
                            d_row_clone.set_visible(false);
                        }
                        crate::updater::UpdateStatus::UpdateAvailable(info) => {
                            s_lbl_clone.set_text(&format!("🌟 v{} is available!", info.tag_name));
                            s_lbl_clone.add_css_class("settings-update-highlight");
                            top_btn_clone.add_css_class("has-update");

                            while let Some(child) = d_row_clone.first_child() {
                                d_row_clone.remove(&child);
                            }

                            if let Some(deb_url) = info.deb_url.clone() {
                                let deb_btn = Button::with_label("Download .deb");
                                deb_btn.add_css_class("settings-deb-btn");
                                deb_btn.connect_clicked(move |_| {
                                    crate::updater::open_url(&deb_url);
                                });
                                d_row_clone.append(&deb_btn);
                            }

                            let view_btn = Button::with_label("View Release Notes");
                            view_btn.add_css_class("settings-release-btn");
                            let url = info.html_url.clone();
                            view_btn.connect_clicked(move |_| {
                                crate::updater::open_url(&url);
                            });
                            d_row_clone.append(&view_btn);

                            d_row_clone.set_visible(true);
                        }
                        crate::updater::UpdateStatus::Error(_) => {
                            s_lbl_clone.set_text("Offline / Check failed");
                            s_lbl_clone.remove_css_class("settings-update-highlight");
                        }
                        _ => {}
                    }
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    c_btn_clone.set_sensitive(true);
                    c_btn_clone.set_label("Check for Updates");
                    s_lbl_clone.set_text("Offline / Check failed");
                    glib::ControlFlow::Break
                }
            }
        });
    });

    let tc = trigger_check.clone();
    check_btn.connect_clicked(move |_| {
        tc();
    });

    // Auto-check once daily on startup
    if crate::updater::should_auto_check(24) {
        let tc_auto = trigger_check.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            tc_auto();
        });
    }

    // Separator
    let sep3 = Separator::new(Orientation::Horizontal);
    sep3.add_css_class("settings-separator");

    // 4. Actions & Info Row: [ Linux Dynamic Island ] [ Quit OpenNotch ]
    let actions_row = GtkBox::new(Orientation::Horizontal, 10);
    actions_row.set_valign(Align::Center);
    actions_row.set_halign(Align::Fill);

    let info_box = GtkBox::new(Orientation::Horizontal, 6);
    info_box.set_valign(Align::Center);
    info_box.set_hexpand(true);

    let info_icon = Image::from_icon_name("dialog-information-symbolic");
    info_icon.set_pixel_size(14);
    info_icon.add_css_class("settings-info-icon");

    let info_lbl = Label::new(Some("Linux Dynamic Island"));
    info_lbl.add_css_class("settings-info-text");

    info_box.append(&info_icon);
    info_box.append(&info_lbl);

    let quit_btn = Button::new();
    let quit_box = GtkBox::new(Orientation::Horizontal, 6);
    quit_box.set_halign(Align::Center);
    quit_box.set_valign(Align::Center);

    let quit_icon = Image::from_icon_name("application-exit-symbolic");
    quit_icon.set_pixel_size(13);
    let quit_lbl = Label::new(Some("Quit OpenNotch"));

    quit_box.append(&quit_icon);
    quit_box.append(&quit_lbl);
    quit_btn.set_child(Some(&quit_box));
    quit_btn.add_css_class("settings-quit-btn");
    quit_btn.set_tooltip_text(Some("Close OpenNotch application"));
    quit_btn.connect_clicked(|_| {
        std::process::exit(0);
    });

    actions_row.append(&info_box);
    actions_row.append(&quit_btn);

    container.append(&header_row);
    container.append(&sep1);
    container.append(&autostart_row);
    container.append(&sep2);
    container.append(&update_row);
    container.append(&details_row);
    container.append(&sep3);
    container.append(&actions_row);

    container
}
