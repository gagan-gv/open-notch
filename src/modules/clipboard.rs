use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, GestureClick, Image, Label, Orientation, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardItem {
    pub id: u64,
    pub text: String,
    pub timestamp: SystemTime,
    pub is_pinned: bool,
    pub is_url: bool,
    pub is_code: bool,
}

static CLIPBOARD_HISTORY: OnceLock<Mutex<Vec<ClipboardItem>>> = OnceLock::new();
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static LAST_COPIED_FLASH: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

pub fn get_clipboard_history() -> Vec<ClipboardItem> {
    CLIPBOARD_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .clone()
}

pub fn get_last_copied_flash() -> bool {
    if let Ok(lock) = LAST_COPIED_FLASH.get_or_init(|| Mutex::new(None)).lock() {
        if let Some(instant) = *lock {
            return instant.elapsed().as_millis() < 2000;
        }
    }
    false
}

pub fn trigger_copied_flash() {
    if let Ok(mut lock) = LAST_COPIED_FLASH.get_or_init(|| Mutex::new(None)).lock() {
        *lock = Some(Instant::now());
    }
}

pub fn add_clipboard_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut history = CLIPBOARD_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();

    // Deduplicate: if top item already has the exact same text, skip
    if let Some(first) = history.first() {
        if first.text == trimmed {
            return false;
        }
    }

    let is_pinned = history
        .iter()
        .find(|item| item.text == trimmed)
        .map(|item| item.is_pinned)
        .unwrap_or(false);

    history.retain(|item| item.text != trimmed);

    let is_url = trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("ftp://")
        || (trimmed.starts_with("www.") && trimmed.contains('.'));

    let is_code = trimmed.contains('{')
        || trimmed.contains('}')
        || trimmed.contains("fn ")
        || trimmed.contains("let ")
        || trimmed.contains("const ")
        || trimmed.contains("def ")
        || trimmed.contains("import ")
        || trimmed.contains("curl ")
        || trimmed.contains("git ")
        || trimmed.contains("cargo ")
        || trimmed.contains("sudo ")
        || trimmed.contains("chmod ")
        || trimmed.lines().count() > 1;

    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let item = ClipboardItem {
        id,
        text: trimmed.to_string(),
        timestamp: SystemTime::now(),
        is_pinned,
        is_url,
        is_code,
    };

    if is_pinned {
        history.insert(0, item);
    } else {
        // Insert right after pinned items
        let first_unpinned = history.iter().position(|i| !i.is_pinned).unwrap_or(history.len());
        history.insert(first_unpinned, item);
    }

    // Limit unpinned items to 30
    let pinned_count = history.iter().filter(|i| i.is_pinned).count();
    let max_total = pinned_count + 30;
    if history.len() > max_total {
        if let Some(pos) = history.iter().rposition(|i| !i.is_pinned) {
            history.remove(pos);
        }
    }

    trigger_copied_flash();
    true
}

pub fn toggle_pin_clipboard_item(id: u64) {
    let mut history = CLIPBOARD_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();

    if let Some(item) = history.iter_mut().find(|i| i.id == id) {
        item.is_pinned = !item.is_pinned;
    }

    history.sort_by(|a, b| match (a.is_pinned, b.is_pinned) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.timestamp.cmp(&a.timestamp),
    });
}

pub fn remove_clipboard_item(id: u64) {
    let mut history = CLIPBOARD_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();
    history.retain(|i| i.id != id);
}

pub fn clear_unpinned_clipboard() {
    let mut history = CLIPBOARD_HISTORY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap();
    history.retain(|i| i.is_pinned);
}

pub fn format_time_ago(time: SystemTime) -> String {
    let elapsed = time.elapsed().unwrap_or(Duration::from_secs(0)).as_secs();
    if elapsed < 60 {
        "Just now".to_string()
    } else if elapsed < 3600 {
        format!("{}m ago", elapsed / 60)
    } else if elapsed < 86400 {
        format!("{}h ago", elapsed / 3600)
    } else {
        format!("{}d ago", elapsed / 86400)
    }
}

pub fn start_clipboard_monitor<F: Fn() + 'static>(on_change: F) {
    if let Some(display) = gtk4::gdk::Display::default() {
        let clipboard = display.clipboard();

        // Initial read of clipboard
        clipboard.read_text_async(gtk4::gio::Cancellable::NONE, move |result| {
            if let Ok(Some(text)) = result {
                add_clipboard_text(&text);
            }
        });

        let cb_fn = Rc::new(on_change);
        clipboard.connect_changed(move |cb| {
            let cb_fn_clone = Rc::clone(&cb_fn);
            cb.read_text_async(gtk4::gio::Cancellable::NONE, move |result| {
                if let Ok(Some(text)) = result {
                    if add_clipboard_text(&text) {
                        cb_fn_clone();
                    }
                }
            });
        });
    }
}

pub struct ClipboardWidget {
    pub container: GtkBox,
    pub refresh_ui_fn: Rc<dyn Fn()>,
}

impl ClipboardWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 6);
        container.set_valign(Align::Center);
        container.set_halign(Align::Fill);
        container.set_hexpand(true);
        container.add_css_class("clipboard-widget");

        // Header Row [ Icon + Title ] [ Badge ] [ Clear ]
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.set_valign(Align::Center);
        header.add_css_class("clipboard-header");

        let title_box = GtkBox::new(Orientation::Horizontal, 6);
        title_box.set_valign(Align::Center);
        let title_icon = Image::from_icon_name("edit-copy-symbolic");
        title_icon.set_pixel_size(14);
        let title_label = Label::new(Some("Clipboard History"));
        title_label.add_css_class("clipboard-title");
        title_box.append(&title_icon);
        title_box.append(&title_label);

        let badge = Label::new(Some("0 items"));
        badge.add_css_class("clipboard-badge");

        let header_actions = GtkBox::new(Orientation::Horizontal, 6);
        header_actions.set_halign(Align::End);
        header_actions.set_hexpand(true);

        let clear_btn = Button::with_label("Clear");
        clear_btn.add_css_class("clipboard-action-btn");
        clear_btn.set_tooltip_text(Some("Clear all unpinned clips"));

        header_actions.append(&clear_btn);
        header.append(&title_box);
        header.append(&badge);
        header.append(&header_actions);

        // Search Input Bar
        let search_entry = Entry::new();
        search_entry.set_placeholder_text(Some("Search clipboard history..."));
        search_entry.add_css_class("clipboard-search-entry");

        // List Container (Scrolled)
        let scrolled = ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scrolled.set_hexpand(true);
        scrolled.set_min_content_height(130);
        scrolled.set_max_content_height(140);
        scrolled.add_css_class("clipboard-scrolled");

        let items_box = GtkBox::new(Orientation::Vertical, 4);
        items_box.set_valign(Align::Start);
        items_box.set_halign(Align::Fill);
        items_box.set_hexpand(true);
        scrolled.set_child(Some(&items_box));

        container.append(&header);
        container.append(&search_entry);
        container.append(&scrolled);

        // UI Refresh Logic
        let items_box_ref = items_box.clone();
        let badge_ref = badge.clone();
        let clear_ref = clear_btn.clone();
        let search_ref = search_entry.clone();
        let refresh_ptr = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));
        let refresh_ptr_outer = Rc::clone(&refresh_ptr);

        let refresh_ui: Rc<dyn Fn()> = Rc::new(move || {
            let filter_query = search_ref.text().to_lowercase();
            let all_items = get_clipboard_history();
            let pinned_count = all_items.iter().filter(|i| i.is_pinned).count();

            // Filter items by search query
            let filtered_items: Vec<ClipboardItem> = all_items
                .into_iter()
                .filter(|item| {
                    if filter_query.is_empty() {
                        true
                    } else {
                        item.text.to_lowercase().contains(&filter_query)
                    }
                })
                .collect();

            // Update badge
            let total = filtered_items.len();
            if total == 0 {
                badge_ref.set_text("Empty");
                clear_ref.set_sensitive(false);
            } else {
                if pinned_count > 0 {
                    badge_ref.set_text(&format!("{} items • {} pinned", total, pinned_count));
                } else {
                    badge_ref.set_text(&format!("{} item{}", total, if total > 1 { "s" } else { "" }));
                }
                clear_ref.set_sensitive(true);
            }

            // Clear current list
            while let Some(child) = items_box_ref.first_child() {
                items_box_ref.remove(&child);
            }

            if filtered_items.is_empty() {
                let empty_box = GtkBox::new(Orientation::Vertical, 6);
                empty_box.set_valign(Align::Center);
                empty_box.set_halign(Align::Center);
                empty_box.set_margin_top(20);
                empty_box.set_margin_bottom(20);

                let empty_icon = Image::from_icon_name("edit-copy-symbolic");
                empty_icon.set_pixel_size(28);
                empty_icon.add_css_class("clip-empty-icon");

                let empty_txt = Label::new(Some(if filter_query.is_empty() {
                    "No clipboard items yet"
                } else {
                    "No matching clips found"
                }));
                empty_txt.add_css_class("clip-empty-text");

                empty_box.append(&empty_icon);
                empty_box.append(&empty_txt);
                items_box_ref.append(&empty_box);
            } else {
                for item in filtered_items {
                    let row = GtkBox::new(Orientation::Horizontal, 8);
                    row.set_valign(Align::Center);
                    row.set_halign(Align::Fill);
                    row.set_hexpand(true);
                    row.add_css_class("clip-row");
                    if item.is_pinned {
                        row.add_css_class("pinned");
                    }

                    // 1. Icon Tag (URL / Code / Text)
                    let icon_name = if item.is_url {
                        "insert-link-symbolic"
                    } else if item.is_code {
                        "utilities-terminal-symbolic"
                    } else {
                        "x-office-document-symbolic"
                    };
                    let icon_img = Image::from_icon_name(icon_name);
                    icon_img.set_pixel_size(14);
                    icon_img.add_css_class("clip-type-icon");

                    // 2. Text Snippet & Meta
                    let text_box = GtkBox::new(Orientation::Vertical, 1);
                    text_box.set_halign(Align::Start);
                    text_box.set_valign(Align::Center);
                    text_box.set_hexpand(true);

                    // First line or full snippet
                    let first_line = item.text.lines().next().unwrap_or(&item.text);
                    let snippet_lbl = Label::new(Some(first_line));
                    snippet_lbl.set_halign(Align::Start);
                    snippet_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                    snippet_lbl.set_max_width_chars(44);
                    snippet_lbl.add_css_class("clip-snippet");
                    if item.is_code {
                        snippet_lbl.add_css_class("code-text");
                    }

                    // Tooltip shows up to first 300 characters
                    let tooltip_preview = if item.text.len() > 300 {
                        format!("{}...", &item.text[..300])
                    } else {
                        item.text.clone()
                    };
                    snippet_lbl.set_tooltip_text(Some(&tooltip_preview));

                    let char_count = item.text.chars().count();
                    let line_count = item.text.lines().count();
                    let meta_info = if line_count > 1 {
                        format!("{} • {} lines", format_time_ago(item.timestamp), line_count)
                    } else {
                        format!("{} • {} chars", format_time_ago(item.timestamp), char_count)
                    };
                    let meta_lbl = Label::new(Some(&meta_info));
                    meta_lbl.set_halign(Align::Start);
                    meta_lbl.add_css_class("clip-meta");

                    text_box.append(&snippet_lbl);
                    text_box.append(&meta_lbl);

                    // 3. Action Buttons [ Copy ] [ Pin ] [ Del ]
                    let actions = GtkBox::new(Orientation::Horizontal, 2);
                    actions.set_halign(Align::End);
                    actions.set_valign(Align::Center);

                    let copy_btn = Button::new();
                    let copy_ico = Image::from_icon_name("edit-copy-symbolic");
                    copy_ico.set_pixel_size(12);
                    copy_btn.set_child(Some(&copy_ico));
                    copy_btn.add_css_class("clip-btn");
                    copy_btn.set_tooltip_text(Some("Copy to clipboard"));
                    let text_for_btn = item.text.clone();
                    let copy_btn_ref = copy_btn.clone();
                    copy_btn.connect_clicked(move |_| {
                        if let Some(display) = gtk4::gdk::Display::default() {
                            display.clipboard().set_text(&text_for_btn);
                            trigger_copied_flash();
                            copy_btn_ref.set_tooltip_text(Some("Copied!"));
                        }
                    });

                    let pin_btn = Button::new();
                    let pin_ico = Image::from_icon_name("view-pin-symbolic");
                    pin_ico.set_pixel_size(12);
                    pin_btn.set_child(Some(&pin_ico));
                    pin_btn.add_css_class("clip-btn");
                    if item.is_pinned {
                        pin_btn.add_css_class("pin-active");
                        pin_btn.set_tooltip_text(Some("Unpin snippet"));
                    } else {
                        pin_btn.set_tooltip_text(Some("Pin to top"));
                    }
                    let item_id_pin = item.id;
                    let refresh_pin = Rc::clone(&refresh_ptr);
                    pin_btn.connect_clicked(move |_| {
                        toggle_pin_clipboard_item(item_id_pin);
                        if let Some(ref cb) = *refresh_pin.borrow() {
                            cb();
                        }
                    });

                    let del_btn = Button::new();
                    let del_ico = Image::from_icon_name("window-close-symbolic");
                    del_ico.set_pixel_size(12);
                    del_btn.set_child(Some(&del_ico));
                    del_btn.add_css_class("clip-btn");
                    del_btn.add_css_class("clip-btn-del");
                    del_btn.set_tooltip_text(Some("Remove snippet"));
                    let item_id_del = item.id;
                    let refresh_del = Rc::clone(&refresh_ptr);
                    del_btn.connect_clicked(move |_| {
                        remove_clipboard_item(item_id_del);
                        if let Some(ref cb) = *refresh_del.borrow() {
                            cb();
                        }
                    });

                    actions.append(&copy_btn);
                    actions.append(&pin_btn);
                    actions.append(&del_btn);

                    // Entire row click gesture copies snippet
                    let row_click = GestureClick::new();
                    let text_for_click = item.text.clone();
                    let row_btn_ref = copy_btn.clone();
                    row_click.connect_released(move |_, _, _, _| {
                        if let Some(display) = gtk4::gdk::Display::default() {
                            display.clipboard().set_text(&text_for_click);
                            trigger_copied_flash();
                            row_btn_ref.set_tooltip_text(Some("Copied!"));
                        }
                    });
                    row.add_controller(row_click);

                    row.append(&icon_img);
                    row.append(&text_box);
                    row.append(&actions);

                    items_box_ref.append(&row);
                }
            }
        });

        // Store back reference
        *refresh_ptr_outer.borrow_mut() = Some(Rc::clone(&refresh_ui));

        // Connect Search input change
        let refresh_search = Rc::clone(&refresh_ui);
        search_entry.connect_changed(move |_| {
            refresh_search();
        });

        // Connect Clear button
        let refresh_clear = Rc::clone(&refresh_ui);
        clear_btn.connect_clicked(move |_| {
            clear_unpinned_clipboard();
            refresh_clear();
        });

        // Initial render
        refresh_ui();

        Self {
            container,
            refresh_ui_fn: refresh_ui,
        }
    }

    #[allow(dead_code)]
    pub fn refresh_ui(&self) {
        (self.refresh_ui_fn)();
    }
}
