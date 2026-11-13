use chrono::{Datelike, Local};
use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Calendar, Label, Orientation};

pub struct CalendarWidget {
    pub container: GtkBox,
}

impl CalendarWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("calendar-widget");

        let now = Local::now();
        let date_str = now.format("%A, %B %d, %Y").to_string();

        let header_label = Label::new(Some(&date_str));
        header_label.add_css_class("calendar-header");

        let calendar = Calendar::new();
        calendar.add_css_class("mini-calendar");

        container.append(&header_label);
        container.append(&calendar);

        Self { container }
    }
}
