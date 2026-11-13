use super::calendar_sync::{self, CalendarEvent};
use chrono::Local;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Calendar, Label, Orientation, PolicyType, ScrolledWindow,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct EventsFile {
    #[serde(default)]
    pub events: Vec<CalendarEvent>,
}

pub struct CalendarWidget {
    pub container: GtkBox,
}

impl CalendarWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 14);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("calendar-widget");

        let now = Local::now();

        // ------------------------------------------------------------------
        // Left Column: Glance Header + Today's Agenda Card
        // ------------------------------------------------------------------
        let left_col = GtkBox::new(Orientation::Vertical, 6);
        left_col.set_valign(Align::Center);
        left_col.set_halign(Align::Center);
        left_col.set_size_request(230, -1);

        // Compact Apple-style Glance Header
        let glance_card = GtkBox::new(Orientation::Horizontal, 12);
        glance_card.set_valign(Align::Center);
        glance_card.set_hexpand(true);
        glance_card.add_css_class("calendar-glance-card");

        let glance_text_box = GtkBox::new(Orientation::Vertical, 2);
        glance_text_box.set_hexpand(true);
        glance_text_box.set_valign(Align::Center);

        let weekday_label = Label::new(Some(&now.format("%A").to_string().to_uppercase()));
        weekday_label.add_css_class("glance-weekday");
        weekday_label.set_halign(Align::Start);

        let month_year_label = Label::new(Some(&now.format("%B %Y").to_string()));
        month_year_label.add_css_class("glance-month-year");
        month_year_label.set_halign(Align::Start);

        let time_label = Label::new(Some(&now.format("%I:%M %p").to_string()));
        time_label.add_css_class("glance-live-clock");
        time_label.set_halign(Align::Start);

        glance_text_box.append(&weekday_label);
        glance_text_box.append(&month_year_label);
        glance_text_box.append(&time_label);

        let day_label = Label::new(Some(&now.format("%d").to_string()));
        day_label.add_css_class("glance-day-number");
        day_label.set_valign(Align::Center);
        day_label.set_halign(Align::End);

        glance_card.append(&glance_text_box);
        glance_card.append(&day_label);

        // Update live clock every second
        let time_update = time_label.clone();
        let weekday_update = weekday_label.clone();
        let day_update = day_label.clone();
        let month_year_update = month_year_label.clone();

        glib::timeout_add_local(Duration::from_secs(1), move || {
            let current = Local::now();
            time_update.set_text(&current.format("%I:%M %p").to_string());
            weekday_update.set_text(&current.format("%A").to_string().to_uppercase());
            day_update.set_text(&current.format("%d").to_string());
            month_year_update.set_text(&current.format("%B %Y").to_string());
            glib::ControlFlow::Continue
        });

        // Agenda / Events Card (Displays Today's Synced Events)
        let agenda_card = GtkBox::new(Orientation::Vertical, 6);
        agenda_card.add_css_class("agenda-card");
        agenda_card.set_hexpand(true);

        // Agenda Header with Title + [📅 Open Calendar] button
        let agenda_header = GtkBox::new(Orientation::Horizontal, 6);
        agenda_header.set_valign(Align::Center);

        let agenda_title = Label::new(Some("TODAY'S SCHEDULE"));
        agenda_title.add_css_class("agenda-header-label");
        agenda_title.set_halign(Align::Start);
        agenda_title.set_hexpand(true);

        // Button to launch system calendar app
        let sys_app_btn = Button::with_label("📅 Open Calendar");
        sys_app_btn.add_css_class("agenda-sync-btn");
        sys_app_btn.set_tooltip_text(Some("Open in system Calendar app"));
        sys_app_btn.connect_clicked(|_| {
            calendar_sync::launch_system_calendar();
        });

        agenda_header.append(&agenda_title);
        agenda_header.append(&sys_app_btn);

        // Events list container wrapped in a ScrolledWindow
        let events_list_box = GtkBox::new(Orientation::Vertical, 4);
        events_list_box.add_css_class("agenda-events-list");

        let events_scroll = ScrolledWindow::new();
        events_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        events_scroll.set_has_frame(false);
        events_scroll.set_propagate_natural_height(true);
        events_scroll.set_min_content_height(75);
        events_scroll.set_max_content_height(120);
        events_scroll.set_child(Some(&events_list_box));

        agenda_card.append(&agenda_header);
        agenda_card.append(&events_scroll);

        left_col.append(&glance_card);
        left_col.append(&agenda_card);

        // ------------------------------------------------------------------
        // Right Column: Monthly Calendar View
        // ------------------------------------------------------------------
        let right_col = GtkBox::new(Orientation::Vertical, 4);
        right_col.set_valign(Align::Center);
        right_col.set_halign(Align::Center);

        // Mini Monthly Calendar
        let calendar_box = GtkBox::new(Orientation::Vertical, 2);
        calendar_box.add_css_class("calendar-grid-box");
        let calendar = Calendar::new();
        calendar.add_css_class("mini-calendar");
        calendar_box.append(&calendar);
        right_col.append(&calendar_box);

        container.append(&left_col);
        container.append(&right_col);

        // Initial agenda render (combining local + system calendar events)
        let events_list_box_clone = events_list_box.clone();
        render_agenda_items(&events_list_box_clone);

        // Auto-refresh schedule events every 30 seconds to pick up new system events
        let events_refresh = events_list_box.clone();
        glib::timeout_add_local(Duration::from_secs(30), move || {
            render_agenda_items(&events_refresh);
            glib::ControlFlow::Continue
        });

        Self { container }
    }
}

fn render_agenda_items(events_list_box: &GtkBox) {
    while let Some(child) = events_list_box.first_child() {
        events_list_box.remove(&child);
    }

    let today_str = Local::now().format("%Y-%m-%d").to_string();

    // 1. Load local events
    let local_events = load_events_from_disk();

    // 2. Load system calendar events
    let system_events = calendar_sync::load_system_calendar_events(&today_str);

    // 3. Merge & deduplicate
    let mut combined_events: Vec<CalendarEvent> = Vec::new();

    for ev in local_events {
        if ev.date == today_str || ev.date.eq_ignore_ascii_case("today") {
            combined_events.push(ev);
        }
    }

    for sys_ev in system_events {
        let is_dup = combined_events.iter().any(|e| {
            e.title.trim().eq_ignore_ascii_case(sys_ev.title.trim())
                && (e.time.trim().eq_ignore_ascii_case(sys_ev.time.trim()) || sys_ev.time == "All Day")
        });
        if !is_dup {
            combined_events.push(sys_ev);
        }
    }

    if combined_events.is_empty() {
        let empty_box = GtkBox::new(Orientation::Vertical, 2);
        empty_box.set_halign(Align::Center);
        empty_box.set_valign(Align::Center);
        empty_box.add_css_class("agenda-empty-box");

        let empty_title = Label::new(Some("🎉 You're done for the day!"));
        empty_title.add_css_class("agenda-empty-title");

        let empty_sub = Label::new(Some("All clear — no upcoming events scheduled"));
        empty_sub.add_css_class("agenda-empty-sub");

        empty_box.append(&empty_title);
        empty_box.append(&empty_sub);
        events_list_box.append(&empty_box);
    } else {
        for (idx, ev) in combined_events.into_iter().enumerate() {
            let row = GtkBox::new(Orientation::Horizontal, 6);
            row.set_valign(Align::Center);
            row.add_css_class("agenda-item-row");

            let time_badge = Label::new(Some(&ev.time));
            time_badge.add_css_class("agenda-item-time");

            let title_label = Label::new(Some(&ev.title));
            title_label.add_css_class("agenda-item-title");
            title_label.set_halign(Align::Start);
            title_label.set_hexpand(true);
            title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

            // If synced from system calendar, show a badge
            if ev.source == "system" {
                let sys_badge = Label::new(Some("📅"));
                sys_badge.add_css_class("agenda-sys-badge");
                sys_badge.set_tooltip_text(Some("Synced from system calendar"));
                row.append(&sys_badge);
            }

            let del_btn = Button::with_label("✕");
            del_btn.add_css_class("agenda-item-del-btn");
            del_btn.set_tooltip_text(Some("Dismiss / remove"));

            let events_list_del = events_list_box.clone();
            let ev_uid = ev.uid.clone();
            let ev_title = ev.title.clone();
            let ev_source = ev.source.clone();

            del_btn.connect_clicked(move |_| {
                if ev_source == "system" {
                    calendar_sync::remove_event_from_system_calendar(ev_uid.as_deref(), &ev_title);
                } else {
                    remove_event_by_index(idx);
                }
                render_agenda_items(&events_list_del);
            });

            row.append(&time_badge);
            row.append(&title_label);
            row.append(&del_btn);
            events_list_box.append(&row);
        }
    }
}

fn get_events_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("open-notch");
        let _ = fs::create_dir_all(&p);
        p.push("events.toml");
        p
    })
}

pub fn load_events_from_disk() -> Vec<CalendarEvent> {
    if let Some(path) = get_events_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(file_data) = toml::from_str::<EventsFile>(&content) {
                    return file_data.events;
                }
            }
        } else {
            let default_data = EventsFile {
                events: vec![
                    CalendarEvent {
                        title: "Explore OpenNotch & customize timers".to_string(),
                        time: "09:00 AM".to_string(),
                        date: "today".to_string(),
                        uid: None,
                        source: "local".to_string(),
                    },
                ],
            };
            if let Ok(serialized) = toml::to_string_pretty(&default_data) {
                let _ = fs::write(&path, serialized);
            }
            return default_data.events;
        }
    }
    Vec::new()
}

fn remove_event_by_index(index: usize) {
    if let Some(path) = get_events_path() {
        let mut events = load_events_from_disk();
        if index < events.len() {
            events.remove(index);
            let data = EventsFile { events };
            if let Ok(serialized) = toml::to_string_pretty(&data) {
                let _ = fs::write(&path, serialized);
            }
        }
    }
}
