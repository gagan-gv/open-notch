use chrono::Local;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Calendar, Entry, Label, Orientation, PolicyType, Revealer,
    RevealerTransitionType, ScrolledWindow,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub title: String,
    pub time: String, // e.g. "10:00 AM"
    pub date: String, // "YYYY-MM-DD" or "today"
}

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
        let container = GtkBox::new(Orientation::Horizontal, 16);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("calendar-widget");

        let now = Local::now();

        // ------------------------------------------------------------------
        // Left Column: Today's Glance Card (Date, Weekday, Live Clock)
        // ------------------------------------------------------------------
        let glance_card = GtkBox::new(Orientation::Vertical, 6);
        glance_card.set_valign(Align::Center);
        glance_card.set_halign(Align::Center);
        glance_card.add_css_class("calendar-glance-card");

        let weekday_label = Label::new(Some(&now.format("%A").to_string().to_uppercase()));
        weekday_label.add_css_class("glance-weekday");

        let day_label = Label::new(Some(&now.format("%d").to_string()));
        day_label.add_css_class("glance-day-number");

        let month_year_label = Label::new(Some(&now.format("%B %Y").to_string()));
        month_year_label.add_css_class("glance-month-year");

        let time_label = Label::new(Some(&now.format("%I:%M %p").to_string()));
        time_label.add_css_class("glance-live-clock");

        glance_card.append(&weekday_label);
        glance_card.append(&day_label);
        glance_card.append(&month_year_label);
        glance_card.append(&time_label);

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

        // ------------------------------------------------------------------
        // Right Column: Monthly Calendar + Today's Agenda Card
        // ------------------------------------------------------------------
        let right_col = GtkBox::new(Orientation::Vertical, 8);
        right_col.set_valign(Align::Center);
        right_col.set_halign(Align::Center);

        // Mini Monthly Calendar
        let calendar_box = GtkBox::new(Orientation::Vertical, 2);
        calendar_box.add_css_class("calendar-grid-box");
        let calendar = Calendar::new();
        calendar.add_css_class("mini-calendar");
        calendar_box.append(&calendar);

        // Agenda / Events Card
        let agenda_card = GtkBox::new(Orientation::Vertical, 6);
        agenda_card.add_css_class("agenda-card");

        // Agenda Header with Title + [+] Quick Add button
        let agenda_header = GtkBox::new(Orientation::Horizontal, 8);
        agenda_header.set_valign(Align::Center);

        let agenda_title = Label::new(Some("TODAY'S SCHEDULE"));
        agenda_title.add_css_class("agenda-header-label");
        agenda_title.set_halign(Align::Start);
        agenda_title.set_hexpand(true);

        let add_btn = Button::with_label("+ Add");
        add_btn.add_css_class("agenda-add-btn");
        add_btn.set_halign(Align::End);

        agenda_header.append(&agenda_title);
        agenda_header.append(&add_btn);

        // Events list container wrapped in a ScrolledWindow
        let events_list_box = GtkBox::new(Orientation::Vertical, 4);
        events_list_box.add_css_class("agenda-events-list");

        let events_scroll = ScrolledWindow::new();
        events_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        events_scroll.set_min_content_height(55);
        events_scroll.set_max_content_height(90);
        events_scroll.set_child(Some(&events_list_box));

        // Inline Add Event Form (revealed when clicking "+ Add")
        let add_revealer = Revealer::new();
        add_revealer.set_transition_type(RevealerTransitionType::SlideDown);
        add_revealer.set_reveal_child(false);

        let add_form_box = GtkBox::new(Orientation::Horizontal, 6);
        add_form_box.add_css_class("agenda-add-row");

        let title_entry = Entry::new();
        title_entry.set_placeholder_text(Some("Event title..."));
        title_entry.add_css_class("agenda-input");
        title_entry.set_hexpand(true);

        let time_entry = Entry::new();
        time_entry.set_placeholder_text(Some("Time (e.g. 2:00 PM)"));
        time_entry.add_css_class("agenda-input");
        time_entry.set_width_chars(14);

        let save_event_btn = Button::with_label("Add");
        save_event_btn.add_css_class("agenda-submit-btn");

        add_form_box.append(&title_entry);
        add_form_box.append(&time_entry);
        add_form_box.append(&save_event_btn);
        add_revealer.set_child(Some(&add_form_box));

        agenda_card.append(&agenda_header);
        agenda_card.append(&add_revealer);
        agenda_card.append(&events_scroll);

        right_col.append(&calendar_box);
        right_col.append(&agenda_card);

        container.append(&glance_card);
        container.append(&right_col);

        // Toggle inline Add form
        let add_revealer_toggle = add_revealer.clone();
        add_btn.connect_clicked(move |_| {
            let is_open = add_revealer_toggle.reveals_child();
            add_revealer_toggle.set_reveal_child(!is_open);
        });

        // Initial agenda render
        let events_list_box_clone = events_list_box.clone();
        render_agenda_items(&events_list_box_clone);

        // Handle Add Event button click
        let events_box_on_add = events_list_box.clone();
        let add_revealer_hide = add_revealer.clone();
        let title_entry_ref = title_entry.clone();
        let time_entry_ref = time_entry.clone();

        save_event_btn.connect_clicked(move |_| {
            let title = title_entry_ref.text().to_string();
            let mut time = time_entry_ref.text().to_string();
            if time.trim().is_empty() {
                time = Local::now().format("%I:%M %p").to_string();
            }

            if !title.trim().is_empty() {
                let today_str = Local::now().format("%Y-%m-%d").to_string();
                add_event_to_disk(&title, &time, &today_str);
                title_entry_ref.set_text("");
                time_entry_ref.set_text("");
                add_revealer_hide.set_reveal_child(false);
                render_agenda_items(&events_box_on_add);
            }
        });

        Self { container }
    }
}

fn render_agenda_items(events_list_box: &GtkBox) {
    while let Some(child) = events_list_box.first_child() {
        events_list_box.remove(&child);
    }

    let today_str = Local::now().format("%Y-%m-%d").to_string();
    let all_events = load_events_from_disk();
    let todays_events: Vec<(usize, CalendarEvent)> = all_events
        .iter()
        .enumerate()
        .filter(|(_, ev)| ev.date == today_str || ev.date.eq_ignore_ascii_case("today"))
        .map(|(idx, ev)| (idx, ev.clone()))
        .collect();

    if todays_events.is_empty() {
        // Empty state card
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
        for (orig_idx, ev) in todays_events {
            let row = GtkBox::new(Orientation::Horizontal, 8);
            row.set_valign(Align::Center);
            row.add_css_class("agenda-item-row");

            let time_badge = Label::new(Some(&ev.time));
            time_badge.add_css_class("agenda-item-time");

            let title_label = Label::new(Some(&ev.title));
            title_label.add_css_class("agenda-item-title");
            title_label.set_halign(Align::Start);
            title_label.set_hexpand(true);
            title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

            let del_btn = Button::with_label("✕");
            del_btn.add_css_class("agenda-item-del-btn");
            del_btn.set_tooltip_text(Some("Mark as done / remove"));

            let events_list_del = events_list_box.clone();
            del_btn.connect_clicked(move |_| {
                remove_event_by_index(orig_idx);
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

fn load_events_from_disk() -> Vec<CalendarEvent> {
    if let Some(path) = get_events_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(file_data) = toml::from_str::<EventsFile>(&content) {
                    return file_data.events;
                }
            }
        } else {
            // Write default initial events file with a starter event
            let default_data = EventsFile {
                events: vec![
                    CalendarEvent {
                        title: "Explore OpenNotch & customize timers".to_string(),
                        time: "09:00 AM".to_string(),
                        date: "today".to_string(),
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

fn add_event_to_disk(title: &str, time: &str, date: &str) {
    if let Some(path) = get_events_path() {
        let mut events = load_events_from_disk();
        events.push(CalendarEvent {
            title: title.trim().to_string(),
            time: time.trim().to_string(),
            date: date.trim().to_string(),
        });
        let data = EventsFile { events };
        if let Ok(serialized) = toml::to_string_pretty(&data) {
            let _ = fs::write(&path, serialized);
        }
    }
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
