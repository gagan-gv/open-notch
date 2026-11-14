use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Image, Label, Orientation, Picture, ScrolledWindow,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, PartialEq)]
pub struct DroppedFile {
    pub path: PathBuf,
    pub name: String,
    pub size_str: String,
    pub is_image: bool,
}

static SHELF_FILES: OnceLock<Mutex<Vec<DroppedFile>>> = OnceLock::new();

pub fn get_shelf_files() -> Vec<DroppedFile> {
    SHELF_FILES
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .clone()
}

pub fn add_shelf_file(path: PathBuf) -> bool {
    let mut files = SHELF_FILES.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    if files.iter().any(|f| f.path == path) {
        return false;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let size_str = if let Ok(metadata) = std::fs::metadata(&path) {
        format_file_size(metadata.len())
    } else {
        "".to_string()
    };

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let is_image = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp");

    files.push(DroppedFile {
        path,
        name,
        size_str,
        is_image,
    });
    true
}

pub fn remove_shelf_file(path: &Path) {
    let mut files = SHELF_FILES.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    files.retain(|f| f.path != path);
}

pub fn clear_shelf_files() {
    let mut files = SHELF_FILES.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap();
    files.clear();
}

fn format_file_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn get_file_icon_name(ext: &str) -> &'static str {
    match ext {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "bmp" => "image-x-generic-symbolic",
        "pdf" => "x-office-document-symbolic",
        "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "html" | "css" | "sh" | "json" | "toml" | "yaml" | "yml" => "text-x-generic-symbolic",
        "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => "package-x-generic-symbolic",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => "audio-x-generic-symbolic",
        "mp4" | "mkv" | "mov" | "avi" | "webm" => "video-x-generic-symbolic",
        "doc" | "docx" | "odt" | "rtf" | "txt" | "md" => "x-office-document-symbolic",
        "xls" | "xlsx" | "csv" | "ods" => "x-office-spreadsheet-symbolic",
        "ppt" | "pptx" | "key" | "odp" => "x-office-presentation-symbolic",
        _ => "text-x-generic-symbolic",
    }
}

pub struct DropShelfWidget {
    pub container: GtkBox,
    pub refresh_ui_fn: Rc<dyn Fn()>,
}

impl DropShelfWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_valign(Align::Center);
        container.set_halign(Align::Fill);
        container.set_hexpand(true);
        container.add_css_class("drop-shelf-widget");

        // Header Row [ Icon + Title ] [ Items badge ] [ + Add ] [ Clear All ]
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.set_valign(Align::Center);
        header.add_css_class("drop-shelf-header");

        let title_box = GtkBox::new(Orientation::Horizontal, 6);
        title_box.set_valign(Align::Center);
        let title_icon = Image::from_icon_name("folder-download-symbolic");
        title_icon.set_pixel_size(14);
        let title_label = Label::new(Some("File Transit Shelf"));
        title_label.add_css_class("drop-shelf-title");
        title_box.append(&title_icon);
        title_box.append(&title_label);

        let count_badge = Label::new(Some("0 items"));
        count_badge.add_css_class("drop-shelf-badge");

        let header_actions = GtkBox::new(Orientation::Horizontal, 6);
        header_actions.set_halign(Align::End);
        header_actions.set_hexpand(true);

        let add_btn = Button::with_label("+ Add");
        add_btn.add_css_class("drop-shelf-action-btn");
        add_btn.set_tooltip_text(Some("Pick files from computer"));

        let clear_btn = Button::with_label("Clear");
        clear_btn.add_css_class("drop-shelf-action-btn");
        clear_btn.set_tooltip_text(Some("Clear all files from shelf"));

        header_actions.append(&add_btn);
        header_actions.append(&clear_btn);

        header.append(&title_box);
        header.append(&count_badge);
        header.append(&header_actions);

        // Body area (switches between empty state drop zone and file list cards)
        let body_box = GtkBox::new(Orientation::Vertical, 0);
        body_box.set_valign(Align::Center);
        body_box.set_halign(Align::Fill);
        body_box.set_hexpand(true);

        container.append(&header);
        container.append(&body_box);

        // File Dialog for "+ Add"
        let add_refresh = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));
        let add_refresh_btn = Rc::clone(&add_refresh);

        add_btn.connect_clicked(move |_| {
            let dialog = gtk4::FileDialog::new();
            dialog.set_title("Choose files to hold in transit");
            let add_refresh_cb = Rc::clone(&add_refresh_btn);

            dialog.open_multiple(
                gtk4::Window::NONE,
                gtk4::gio::Cancellable::NONE,
                move |result| {
                    if let Ok(files) = result {
                        let mut added = false;
                        for i in 0..files.n_items() {
                            if let Some(file) = files.item(i).and_downcast::<gtk4::gio::File>() {
                                if let Some(path) = file.path() {
                                    if add_shelf_file(path) {
                                        added = true;
                                    }
                                }
                            }
                        }
                        if added {
                            if let Some(ref cb) = *add_refresh_cb.borrow() {
                                cb();
                            }
                        }
                    }
                },
            );
        });

        // Clear All action
        let clear_refresh = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));
        let clear_refresh_btn = Rc::clone(&clear_refresh);
        clear_btn.connect_clicked(move |_| {
            clear_shelf_files();
            if let Some(ref cb) = *clear_refresh_btn.borrow() {
                cb();
            }
        });

        // UI Builder Closure
        let body_clone = body_box.clone();
        let badge_clone = count_badge.clone();
        let clear_clone = clear_btn.clone();
        let refresh_ptr = Rc::new(RefCell::new(None::<Rc<dyn Fn()>>));
        let refresh_ptr_outer = Rc::clone(&refresh_ptr);

        let refresh_ui: Rc<dyn Fn()> = Rc::new(move || {
            let files = get_shelf_files();
            let count = files.len();

            // Update badge & clear button
            if count == 0 {
                badge_clone.set_text("Empty");
                clear_clone.set_sensitive(false);
            } else {
                badge_clone.set_text(&format!("{} item{}", count, if count > 1 { "s" } else { "" }));
                clear_clone.set_sensitive(true);
            }

            // Clear body
            while let Some(child) = body_clone.first_child() {
                body_clone.remove(&child);
            }

            if files.is_empty() {
                // Empty state drop zone
                let empty_zone = GtkBox::new(Orientation::Vertical, 6);
                empty_zone.set_valign(Align::Center);
                empty_zone.set_halign(Align::Fill);
                empty_zone.set_hexpand(true);
                empty_zone.add_css_class("drop-shelf-empty");

                let icon = Image::from_icon_name("folder-download-symbolic");
                icon.set_pixel_size(28);
                icon.add_css_class("drop-empty-icon");

                let main_text = Label::new(Some("Drop files here to hold in transit"));
                main_text.add_css_class("drop-empty-text");

                let sub_text = Label::new(Some("Drag screenshots, downloads, or documents here. Drag out anywhere."));
                sub_text.add_css_class("drop-empty-subtext");

                empty_zone.append(&icon);
                empty_zone.append(&main_text);
                empty_zone.append(&sub_text);

                body_clone.append(&empty_zone);
            } else {
                // Horizontal scrolling card list
                let scrolled = ScrolledWindow::new();
                scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
                scrolled.set_hexpand(true);
                scrolled.set_min_content_height(125);
                scrolled.add_css_class("drop-shelf-scrolled");

                let cards_box = GtkBox::new(Orientation::Horizontal, 10);
                cards_box.set_valign(Align::Center);
                cards_box.set_halign(Align::Start);
                cards_box.set_hexpand(true);
                cards_box.set_margin_top(4);
                cards_box.set_margin_bottom(4);
                cards_box.set_margin_start(4);
                cards_box.set_margin_end(4);

                for item in files {
                    let card = GtkBox::new(Orientation::Vertical, 4);
                    card.set_size_request(105, 115);
                    card.set_valign(Align::Center);
                    card.set_halign(Align::Center);
                    card.add_css_class("drop-card");

                    // 1. Thumbnail or Icon
                    let ext = item.path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    let icon_area = GtkBox::new(Orientation::Vertical, 0);
                    icon_area.set_valign(Align::Center);
                    icon_area.set_halign(Align::Center);
                    icon_area.set_size_request(42, 42);

                    if item.is_image && item.path.exists() {
                        let pic = Picture::for_filename(&item.path);
                        pic.set_content_fit(gtk4::ContentFit::Cover);
                        pic.set_size_request(40, 40);
                        pic.add_css_class("drop-card-picture");
                        icon_area.append(&pic);
                    } else {
                        let icon = Image::from_icon_name(get_file_icon_name(&ext));
                        icon.set_pixel_size(24);
                        icon.add_css_class("drop-card-icon");
                        icon_area.append(&icon);
                    }

                    // 2. Name
                    let name_lbl = Label::new(Some(&item.name));
                    name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                    name_lbl.set_max_width_chars(11);
                    name_lbl.set_tooltip_text(Some(&format!("{}\nPath: {}", item.name, item.path.display())));
                    name_lbl.add_css_class("drop-card-name");

                    // 3. Size
                    let size_lbl = Label::new(Some(&item.size_str));
                    size_lbl.add_css_class("drop-card-size");

                    // 4. Action bar [ Copy ] [ Reveal ] [ Remove ]
                    let action_bar = GtkBox::new(Orientation::Horizontal, 2);
                    action_bar.set_halign(Align::Center);
                    action_bar.add_css_class("drop-card-actions");

                    let copy_btn = Button::new();
                    let copy_ico = Image::from_icon_name("edit-copy-symbolic");
                    copy_ico.set_pixel_size(12);
                    copy_btn.set_child(Some(&copy_ico));
                    copy_btn.add_css_class("drop-card-btn");
                    copy_btn.set_tooltip_text(Some("Copy file path"));
                    let path_for_copy = item.path.clone();
                    let copy_btn_ref = copy_btn.clone();
                    copy_btn.connect_clicked(move |_| {
                        if let Some(display) = gtk4::gdk::Display::default() {
                            display.clipboard().set_text(&path_for_copy.to_string_lossy());
                            copy_btn_ref.set_tooltip_text(Some("Path copied!"));
                        }
                    });

                    let reveal_btn = Button::new();
                    let reveal_ico = Image::from_icon_name("folder-open-symbolic");
                    reveal_ico.set_pixel_size(12);
                    reveal_btn.set_child(Some(&reveal_ico));
                    reveal_btn.add_css_class("drop-card-btn");
                    reveal_btn.set_tooltip_text(Some("Reveal in file manager"));
                    let path_for_reveal = item.path.clone();
                    reveal_btn.connect_clicked(move |_| {
                        let parent = path_for_reveal.parent().unwrap_or(&path_for_reveal);
                        let _ = std::process::Command::new("xdg-open")
                            .arg(parent)
                            .spawn();
                    });

                    let remove_btn = Button::new();
                    let del_ico = Image::from_icon_name("window-close-symbolic");
                    del_ico.set_pixel_size(12);
                    remove_btn.set_child(Some(&del_ico));
                    remove_btn.add_css_class("drop-card-btn");
                    remove_btn.add_css_class("drop-card-btn-remove");
                    remove_btn.set_tooltip_text(Some("Remove from tray"));
                    let path_for_remove = item.path.clone();
                    let refresh_for_remove = Rc::clone(&refresh_ptr);
                    remove_btn.connect_clicked(move |_| {
                        remove_shelf_file(&path_for_remove);
                        if let Some(ref cb) = *refresh_for_remove.borrow() {
                            cb();
                        }
                    });

                    action_bar.append(&copy_btn);
                    action_bar.append(&reveal_btn);
                    action_bar.append(&remove_btn);

                    // DragSource on card: Dragging from card copies file to any app!
                    let drag_source = gtk4::DragSource::new();
                    drag_source.set_actions(gtk4::gdk::DragAction::COPY);
                    let path_for_drag = item.path.clone();
                    drag_source.connect_prepare(move |_source, _x, _y| {
                        let gio_file = gtk4::gio::File::for_path(&path_for_drag);
                        let file_list = gtk4::gdk::FileList::from_array(&[gio_file]);
                        Some(gtk4::gdk::ContentProvider::for_value(&file_list.to_value()))
                    });
                    card.add_controller(drag_source);

                    card.append(&icon_area);
                    card.append(&name_lbl);
                    card.append(&size_lbl);
                    card.append(&action_bar);

                    cards_box.append(&card);
                }

                scrolled.set_child(Some(&cards_box));
                body_clone.append(&scrolled);
            }
        });

        // Store back references for UI callbacks
        *refresh_ptr_outer.borrow_mut() = Some(Rc::clone(&refresh_ui));
        *add_refresh.borrow_mut() = Some(Rc::clone(&refresh_ui));
        *clear_refresh.borrow_mut() = Some(Rc::clone(&refresh_ui));

        // Drag & Drop Target on the container
        let drop_target = gtk4::DropTarget::new(gtk4::gdk::FileList::static_type(), gtk4::gdk::DragAction::COPY);
        let container_drop_ref = container.clone();
        let refresh_on_drop = Rc::clone(&refresh_ui);

        drop_target.connect_enter(move |_target, _x, _y| {
            container_drop_ref.add_css_class("drop-active");
            gtk4::gdk::DragAction::COPY
        });

        let container_leave_ref = container.clone();
        drop_target.connect_leave(move |_target| {
            container_leave_ref.remove_css_class("drop-active");
        });

        let container_end_ref = container.clone();
        drop_target.connect_drop(move |_target, value, _x, _y| {
            container_end_ref.remove_css_class("drop-active");
            if let Ok(file_list) = value.get::<gtk4::gdk::FileList>() {
                let mut added = false;
                for file in file_list.files() {
                    if let Some(path) = file.path() {
                        if add_shelf_file(path) {
                            added = true;
                        }
                    }
                }
                if added {
                    refresh_on_drop();
                }
                true
            } else {
                false
            }
        });

        container.add_controller(drop_target);

        // Initial render
        refresh_ui();

        Self {
            container,
            refresh_ui_fn: refresh_ui,
        }
    }

    pub fn refresh_ui(&self) {
        (self.refresh_ui_fn)();
    }
}
