use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Label, Orientation, PolicyType, ScrolledWindow, TextView};
use std::fs;
use std::path::PathBuf;

pub struct NotesWidget {
    pub container: GtkBox,
}

impl NotesWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.add_css_class("notes-widget");

        let header = Label::new(Some("📝 Quick Scratchpad"));
        header.add_css_class("notes-header");

        let text_view = TextView::new();
        text_view.set_wrap_mode(gtk4::WrapMode::Word);
        text_view.add_css_class("notes-editor");

        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(PolicyType::Never, PolicyType::Automatic);
        scrolled.set_min_content_height(110);
        scrolled.set_child(Some(&text_view));

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
            buffer.connect_changed(move |buf| {
                let start = buf.start_iter();
                let end = buf.end_iter();
                let text = buf.text(&start, &end, false);
                let _ = fs::write(&path, text.as_str());
            });
        }

        container.append(&header);
        container.append(&scrolled);

        Self { container }
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
