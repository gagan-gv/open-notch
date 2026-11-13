use super::calendar_sync::{self, CalendarEvent};
use super::mpris::{ActiveMediaInfo, MprisWidget};
use chrono::{Duration as ChronoDuration, Local};
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Label, Orientation, PolicyType, ProgressBar, Revealer,
    RevealerTransitionType, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

pub struct DashboardWidget {
    pub container: GtkBox,
}

impl DashboardWidget {
    pub fn new(mpris_widget: &MprisWidget) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 14);
        container.set_valign(Align::Center);
        container.set_halign(Align::Fill);
        container.set_hexpand(true);
        container.add_css_class("dashboard-container");

        // ------------------------------------------------------------------
        // Left Side: Media Player + Divider (Revealed ONLY when real music is playing)
        // ------------------------------------------------------------------
        let (media_panel, media_updater) = build_media_panel();
        media_panel.set_size_request(340, -1);

        let divider = GtkBox::new(Orientation::Vertical, 0);
        divider.add_css_class("dashboard-divider");
        divider.set_size_request(1, 140);
        divider.set_valign(Align::Center);

        let media_wrapper = GtkBox::new(Orientation::Horizontal, 12);
        media_wrapper.append(&media_panel);
        media_wrapper.append(&divider);

        let media_revealer = Revealer::new();
        media_revealer.set_transition_type(RevealerTransitionType::SlideRight);
        media_revealer.set_transition_duration(250);
        media_revealer.set_reveal_child(false);
        media_revealer.set_child(Some(&media_wrapper));

        // ------------------------------------------------------------------
        // Right Side: Calendar Week Strip + Today's Agenda (Expands when idle)
        // ------------------------------------------------------------------
        let calendar_panel = build_calendar_panel();
        calendar_panel.set_hexpand(true);
        calendar_panel.set_halign(Align::Fill);

        container.append(&media_revealer);
        container.append(&calendar_panel);

        // Connect live MPRIS updates: reveal media when music plays, collapse when idle
        let is_active_check = Arc::clone(&mpris_widget.is_active);
        let updater_clone = Rc::clone(&media_updater);
        let media_rev = media_revealer.clone();

        // Immediate initial check on startup
        let initial_active = is_active_check.load(Ordering::Relaxed);
        let initial_info = super::mpris::get_current_active_media();
        let initial_has_playback = initial_active
            && initial_info
                .as_ref()
                .map(|m| m.is_playing && !m.title.is_empty() && m.title != "No Media Playing")
                .unwrap_or(false);

        if initial_has_playback {
            media_revealer.set_visible(true);
            media_revealer.set_reveal_child(true);
            calendar_panel.set_halign(Align::Fill);
            calendar_panel.set_size_request(-1, -1);
        } else {
            media_revealer.set_visible(false);
            media_revealer.set_reveal_child(false);
            calendar_panel.set_halign(Align::Center);
            calendar_panel.set_size_request(400, -1);
        }
        updater_clone(initial_has_playback, initial_info);

        let calendar_ref = calendar_panel.clone();
        glib::timeout_add_local(Duration::from_millis(500), move || {
            let active = is_active_check.load(Ordering::Relaxed);
            let active_info = super::mpris::get_current_active_media();

            let has_active_playback = active
                && active_info
                    .as_ref()
                    .map(|m| m.is_playing && !m.title.is_empty() && m.title != "No Media Playing")
                    .unwrap_or(false);

            if has_active_playback {
                media_rev.set_visible(true);
                media_rev.set_reveal_child(true);
                calendar_ref.set_halign(Align::Fill);
                calendar_ref.set_size_request(-1, -1);
            } else {
                media_rev.set_reveal_child(false);
                calendar_ref.set_halign(Align::Center);
                calendar_ref.set_size_request(400, -1);
                let m_rev = media_rev.clone();
                glib::timeout_add_local_once(Duration::from_millis(260), move || {
                    if !m_rev.reveals_child() {
                        m_rev.set_visible(false);
                    }
                });
            }

            updater_clone(has_active_playback, active_info);
            glib::ControlFlow::Continue
        });

        Self { container }
    }
}

/// Helper type for updating media UI widgets live
type MediaUpdaterFn = Rc<dyn Fn(bool, Option<ActiveMediaInfo>)>;

fn build_media_panel() -> (GtkBox, MediaUpdaterFn) {
    let panel = GtkBox::new(Orientation::Horizontal, 12);
    panel.set_valign(Align::Center);
    panel.add_css_class("dashboard-media-panel");

    // Album Art (100x100 rounded card with soft gradient and speaker icon)
    let art_box = GtkBox::new(Orientation::Vertical, 0);
    art_box.set_size_request(96, 96);
    art_box.set_valign(Align::Center);
    art_box.add_css_class("dashboard-album-art");

    let speaker_icon = Label::new(Some("🔊"));
    speaker_icon.set_valign(Align::Center);
    speaker_icon.set_halign(Align::Center);
    speaker_icon.add_css_class("dashboard-speaker-icon");
    art_box.append(&speaker_icon);

    // Spotify/App badge in the corner of album art
    let app_badge = Label::new(Some("🟢"));
    app_badge.set_valign(Align::End);
    app_badge.set_halign(Align::End);
    app_badge.add_css_class("dashboard-app-badge");
    art_box.append(&app_badge);

    // Track Details & Controls Column
    let details_col = GtkBox::new(Orientation::Vertical, 3);
    details_col.set_valign(Align::Center);
    details_col.set_hexpand(true);

    let title_label = Label::new(Some(""));
    title_label.add_css_class("dashboard-track-title");
    title_label.set_halign(Align::Start);
    title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

    let artist_label = Label::new(Some(""));
    artist_label.add_css_class("dashboard-track-artist");
    artist_label.set_halign(Align::Start);
    artist_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

    let subtitle_label = Label::new(Some(""));
    subtitle_label.add_css_class("dashboard-track-sub");
    subtitle_label.set_halign(Align::Start);

    // Amber Progress Bar + Timestamps
    let progress_bar = ProgressBar::new();
    progress_bar.add_css_class("dashboard-progress-bar");
    progress_bar.set_fraction(0.0);

    let time_row = GtkBox::new(Orientation::Horizontal, 0);
    time_row.add_css_class("dashboard-time-row");

    let time_curr = Label::new(Some("0:00"));
    time_curr.add_css_class("dashboard-time-label");
    time_curr.set_halign(Align::Start);
    time_curr.set_hexpand(true);

    let time_total = Label::new(Some("--:--"));
    time_total.add_css_class("dashboard-time-label");
    time_total.set_halign(Align::End);

    time_row.append(&time_curr);
    time_row.append(&time_total);

    // Playback Controls Row: [ ⏮ ] [ ⏸ ] [ ⏭ ] ... [ ♡ ]
    let controls_row = GtkBox::new(Orientation::Horizontal, 10);
    controls_row.set_valign(Align::Center);
    controls_row.add_css_class("dashboard-controls-row");

    let prev_btn = Button::with_label("⏮");
    prev_btn.add_css_class("dashboard-ctrl-btn");
    prev_btn.connect_clicked(|_| {
        super::mpris::send_mpris_action("previous");
    });

    let play_pause_btn = Button::with_label("⏸");
    play_pause_btn.add_css_class("dashboard-play-btn");
    play_pause_btn.connect_clicked(|btn| {
        if btn.label().as_deref() == Some("⏸") {
            btn.set_label("▶");
        } else {
            btn.set_label("⏸");
        }
        super::mpris::send_mpris_action("play_pause");
    });
    let play_btn_update = play_pause_btn.clone();

    let next_btn = Button::with_label("⏭");
    next_btn.add_css_class("dashboard-ctrl-btn");
    next_btn.connect_clicked(|_| {
        super::mpris::send_mpris_action("next");
    });

    // Heart / Favorite toggle button
    let heart_btn = Button::with_label("♡");
    heart_btn.add_css_class("dashboard-heart-btn");
    heart_btn.set_halign(Align::End);
    heart_btn.set_hexpand(true);
    let is_liked = Rc::new(RefCell::new(false));
    let heart_toggle = heart_btn.clone();
    heart_btn.connect_clicked(move |_| {
        let mut liked = is_liked.borrow_mut();
        *liked = !*liked;
        if *liked {
            heart_toggle.set_label("♥");
            heart_toggle.add_css_class("heart-active");
        } else {
            heart_toggle.set_label("♡");
            heart_toggle.remove_css_class("heart-active");
        }
    });

    controls_row.append(&prev_btn);
    controls_row.append(&play_pause_btn);
    controls_row.append(&next_btn);
    controls_row.append(&heart_btn);

    details_col.append(&title_label);
    details_col.append(&artist_label);
    details_col.append(&subtitle_label);
    details_col.append(&progress_bar);
    details_col.append(&time_row);
    details_col.append(&controls_row);

    panel.append(&art_box);
    panel.append(&details_col);

    let updater = Rc::new(move |has_active: bool, info: Option<ActiveMediaInfo>| {
        if has_active {
            if let Some(media) = info {
                title_label.set_text(&media.title);
                artist_label.set_text(&media.artist);
                subtitle_label.set_text(&format!("via {}", media.player_display_name));
                progress_bar.set_fraction(media.progress_fraction);

                let pos_m = media.position_sec / 60;
                let pos_s = media.position_sec % 60;
                time_curr.set_text(&format!("{}:{:02}", pos_m, pos_s));

                if media.length_sec > 0 {
                    let len_m = media.length_sec / 60;
                    let len_s = media.length_sec % 60;
                    time_total.set_text(&format!("{}:{:02}", len_m, len_s));
                } else {
                    time_total.set_text("--:--");
                }

                if media.is_playing {
                    play_btn_update.set_label("⏸");
                } else {
                    play_btn_update.set_label("▶");
                }

                let p_name = media.player_display_name.to_lowercase();
                if p_name.contains("spotify") {
                    app_badge.set_text("🟢");
                } else if p_name.contains("firefox") || p_name.contains("chrome") || p_name.contains("brave") {
                    app_badge.set_text("🌐");
                } else {
                    app_badge.set_text("🎵");
                }
            }
        }
    });

    (panel, updater)
}

fn build_calendar_panel() -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 8);
    panel.set_valign(Align::Center);
    panel.set_hexpand(true);
    panel.add_css_class("dashboard-calendar-panel");

    let now = Local::now();

    // ------------------------------------------------------------------
    // Week Strip Row: Month & Year (Stacked) + 7-Day Horizontal Strip
    // ------------------------------------------------------------------
    let header_strip = GtkBox::new(Orientation::Horizontal, 10);
    header_strip.set_valign(Align::Center);
    header_strip.add_css_class("dashboard-header-strip");

    // Month & Year (Left of the strip)
    let month_box = GtkBox::new(Orientation::Vertical, 1);
    month_box.set_valign(Align::Center);
    month_box.add_css_class("strip-month-box");

    let month_label = Label::new(Some(&now.format("%b").to_string()));
    month_label.add_css_class("strip-month-label");
    month_label.set_halign(Align::Start);

    let year_label = Label::new(Some(&now.format("%Y").to_string()));
    year_label.add_css_class("strip-year-label");
    year_label.set_halign(Align::Start);

    month_box.append(&month_label);
    month_box.append(&year_label);
    header_strip.append(&month_box);

    // 7-Day Horizontal Strip (Centered around today)
    let days_box = GtkBox::new(Orientation::Horizontal, 8);
    days_box.set_valign(Align::Center);
    days_box.set_halign(Align::End);
    days_box.set_hexpand(true);

    for offset in -2..=4 {
        let day = now + ChronoDuration::days(offset);
        let col = GtkBox::new(Orientation::Vertical, 2);
        col.set_halign(Align::Center);
        col.set_valign(Align::Center);

        let is_today = offset == 0;

        let weekday_name = day.format("%a").to_string(); // Thu, Fri, Sat...
        let day_num = day.format("%e").to_string();      // 12, 13, 14...

        let weekday_label = Label::new(Some(&weekday_name));
        weekday_label.add_css_class("strip-weekday-label");

        let day_label = Label::new(Some(day_num.trim()));
        if is_today {
            day_label.add_css_class("strip-day-today");
            weekday_label.add_css_class("strip-weekday-today");
        } else {
            day_label.add_css_class("strip-day-label");
        }

        col.append(&weekday_label);
        col.append(&day_label);
        days_box.append(&col);
    }

    header_strip.append(&days_box);
    panel.append(&header_strip);

    // ------------------------------------------------------------------
    // Events List with Vertical Amber Accent Bars (BoringNotch style)
    // ------------------------------------------------------------------
    let events_list_box = GtkBox::new(Orientation::Vertical, 5);
    events_list_box.add_css_class("dashboard-events-list");

    let events_scroll = ScrolledWindow::new();
    events_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    events_scroll.set_has_frame(false);
    events_scroll.set_propagate_natural_height(true);
    events_scroll.set_min_content_height(75);
    events_scroll.set_max_content_height(115);
    events_scroll.set_child(Some(&events_list_box));

    panel.append(&events_scroll);

    // Render events
    let events_box_clone = events_list_box.clone();
    render_dashboard_events(&events_box_clone);

    // Auto-refresh events every 30 seconds
    let events_refresh = events_list_box.clone();
    glib::timeout_add_local(Duration::from_secs(30), move || {
        render_dashboard_events(&events_refresh);
        glib::ControlFlow::Continue
    });

    panel
}

fn render_dashboard_events(events_list_box: &GtkBox) {
    while let Some(child) = events_list_box.first_child() {
        events_list_box.remove(&child);
    }

    let today_str = Local::now().format("%Y-%m-%d").to_string();

    // Load local + system calendar events
    let local_events = super::calendar::load_events_from_disk();
    let system_events = calendar_sync::load_system_calendar_events(&today_str);

    let mut combined: Vec<CalendarEvent> = Vec::new();
    for ev in local_events {
        if ev.date == today_str || ev.date.eq_ignore_ascii_case("today") {
            combined.push(ev);
        }
    }
    for sys_ev in system_events {
        let is_dup = combined.iter().any(|e| {
            e.title.trim().eq_ignore_ascii_case(sys_ev.title.trim())
                && (e.time.trim().eq_ignore_ascii_case(sys_ev.time.trim()) || sys_ev.time == "All Day")
        });
        if !is_dup {
            combined.push(sys_ev);
        }
    }

    // When there are no events scheduled, display celebratory empty state (zero dummy events)
    if combined.is_empty() {
        let empty_box = GtkBox::new(Orientation::Vertical, 4);
        empty_box.set_halign(Align::Center);
        empty_box.set_valign(Align::Center);
        empty_box.set_hexpand(true);
        empty_box.set_vexpand(true);
        empty_box.add_css_class("agenda-empty-box");

        let empty_title = Label::new(Some("🎉 You're done for the day!"));
        empty_title.add_css_class("agenda-empty-title");

        let empty_sub = Label::new(Some("All clear — no upcoming events scheduled"));
        empty_sub.add_css_class("agenda-empty-sub");

        empty_box.append(&empty_title);
        empty_box.append(&empty_sub);
        events_list_box.append(&empty_box);
        return;
    }

    for ev in combined {
        let row = GtkBox::new(Orientation::Horizontal, 8);
        row.set_valign(Align::Center);
        row.add_css_class("dashboard-event-item");

        // Amber vertical accent bar
        let amber_bar = GtkBox::new(Orientation::Vertical, 0);
        amber_bar.add_css_class("dashboard-amber-bar");
        amber_bar.set_size_request(3, 26);

        // Event Title & Time stacked vertically
        let text_col = GtkBox::new(Orientation::Vertical, 1);
        text_col.set_valign(Align::Center);
        text_col.set_hexpand(true);

        let title_label = Label::new(Some(&ev.title));
        title_label.add_css_class("dashboard-event-title");
        title_label.set_halign(Align::Start);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let time_label = Label::new(Some(&ev.time));
        time_label.add_css_class("dashboard-event-time");
        time_label.set_halign(Align::Start);

        text_col.append(&title_label);
        text_col.append(&time_label);

        row.append(&amber_bar);
        row.append(&text_col);
        events_list_box.append(&row);
    }
}

/// Reads Linux battery capacity & charging status from `/sys/class/power_supply/BAT*`
pub fn get_battery_info() -> Option<(u32, bool)> {
    for bat in &["BAT1", "BAT0"] {
        let cap_path = format!("/sys/class/power_supply/{}/capacity", bat);
        if let Ok(c) = std::fs::read_to_string(&cap_path) {
            if let Ok(val) = c.trim().parse::<u32>() {
                let stat_path = format!("/sys/class/power_supply/{}/status", bat);
                let is_charging = std::fs::read_to_string(&stat_path)
                    .map(|s| s.trim().to_lowercase().contains("charging"))
                    .unwrap_or(false);
                return Some((val, is_charging));
            }
        }
    }
    None
}
