use chrono::Local;
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Calendar, Label, Orientation};
use std::time::Duration;

pub struct CalendarWidget {
    pub container: GtkBox,
}

impl CalendarWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 18);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("calendar-widget");

        let now = Local::now();

        // Left Column: Today's Glance Card
        let glance_card = GtkBox::new(Orientation::Vertical, 4);
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

        // Right Column: Interactive Monthly Calendar
        let calendar_box = GtkBox::new(Orientation::Vertical, 4);
        calendar_box.set_valign(Align::Center);
        calendar_box.set_halign(Align::Center);
        calendar_box.add_css_class("calendar-grid-box");

        let calendar = Calendar::new();
        calendar.add_css_class("mini-calendar");
        calendar_box.append(&calendar);

        container.append(&glance_card);
        container.append(&calendar_box);

        Self { container }
    }
}
