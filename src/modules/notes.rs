use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, GestureClick, Label, Orientation, PolicyType, PropagationPhase,
    ScrolledWindow, TextView,
};
use std::fs;
use std::path::PathBuf;

pub struct NotesWidget {
    pub container: GtkBox,
    pub text_view: TextView,
}

impl NotesWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.add_css_class("notes-widget");

        // Header Row: Title + Auto-save indicator
        let header_row = GtkBox::new(Orientation::Horizontal, 8);
        header_row.set_valign(Align::Center);

        let header = Label::new(Some("📝 Scratchpad"));
        header.add_css_class("notes-header");
        header.set_halign(Align::Start);
        header.set_hexpand(true);

        let status_badge = Label::new(Some("✓ Saved locally"));
        status_badge.add_css_class("notes-status-badge");
        status_badge.set_halign(Align::End);

        header_row.append(&header);
        header_row.append(&status_badge);

        let text_view = TextView::new();
        text_view.set_editable(true);
        text_view.set_cursor_visible(true);
        text_view.set_focusable(true);
        text_view.set_wrap_mode(gtk4::WrapMode::Word);
        text_view.add_css_class("notes-editor");
        text_view.set_left_margin(12);
        text_view.set_right_margin(12);
        text_view.set_top_margin(10);
        text_view.set_bottom_margin(10);

        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled.set_min_content_height(140);
        scrolled.set_child(Some(&text_view));
        scrolled.add_css_class("notes-scrolled-window");

        // Ensure clicking anywhere in the text view grabs GTK and X11 focus
        let tv_click = GestureClick::new();
        tv_click.set_propagation_phase(PropagationPhase::Capture);
        let tv_focus = text_view.clone();
        tv_click.connect_pressed(move |_, _, _, _| {
            tv_focus.grab_focus();
            crate::window::position::grab_x11_input_focus();
        });
        text_view.add_controller(tv_click);

        // Also ensure clicking empty space in the scrolled window focuses the editor
        let scrolled_click = GestureClick::new();
        scrolled_click.set_propagation_phase(PropagationPhase::Capture);
        let tv_scrolled_focus = text_view.clone();
        scrolled_click.connect_pressed(move |_, _, _, _| {
            tv_scrolled_focus.grab_focus();
            crate::window::position::grab_x11_input_focus();
        });
        scrolled.add_controller(scrolled_click);

        let notes_path = Self::get_notes_path();

        // Load saved notes
        if let Some(ref path) = notes_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    text_view.buffer().set_text(&content);
                }
            }
        }

        // Auto-save on buffer changes
        if let Some(path) = notes_path {
            let buffer = text_view.buffer();
            let status_label = status_badge.clone();
            buffer.connect_changed(move |buf| {
                let start = buf.start_iter();
                let end = buf.end_iter();
                let text = buf.text(&start, &end, false);
                let _ = fs::write(&path, text.as_str());
                status_label.set_text("✓ Auto-saved");
            });
        }

        container.append(&header_row);
        container.append(&scrolled);

        Self {
            container,
            text_view,
        }
    }

    fn get_notes_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("open-notch");
            let _ = fs::create_dir_all(&p);
            p.push("notes.txt");
            p
        })
    }
}
