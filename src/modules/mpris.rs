use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[zbus::proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer2Player {
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>;

    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    fn previous(&self) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveMediaInfo {
    pub bus_name: String,
    pub title: String,
    pub artist: String,
    pub status: String,
    pub is_playing: bool,
}

pub struct MprisWidget {
    pub container: GtkBox,
    #[allow(dead_code)]
    pub is_active: Arc<AtomicBool>,
}

impl MprisWidget {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("mpris-widget");

        let track_info = GtkBox::new(Orientation::Vertical, 2);
        track_info.set_halign(Align::Center);

        let title_label = Label::new(Some("No Media Playing"));
        title_label.add_css_class("mpris-title");
        title_label.set_wrap(true);
        title_label.set_max_width_chars(38);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let artist_label = Label::new(Some("Idle"));
        artist_label.add_css_class("mpris-artist");
        artist_label.set_wrap(true);
        artist_label.set_max_width_chars(38);
        artist_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        track_info.append(&title_label);
        track_info.append(&artist_label);

        let controls = GtkBox::new(Orientation::Horizontal, 16);
        controls.set_halign(Align::Center);

        let prev_btn = Button::with_label("⏮");
        prev_btn.add_css_class("mpris-ctrl-btn");

        let play_btn = Button::with_label("▶");
        play_btn.add_css_class("mpris-play-btn");

        let next_btn = Button::with_label("⏭");
        next_btn.add_css_class("mpris-ctrl-btn");

        controls.append(&prev_btn);
        controls.append(&play_btn);
        controls.append(&next_btn);

        container.append(&track_info);
        container.append(&controls);

        let is_active = Arc::new(AtomicBool::new(false));
        let active_bus_name = Rc::new(RefCell::new(None::<String>));
        let shared_media = Arc::new(Mutex::new(None::<ActiveMediaInfo>));

        // Button Controls
        let bus_for_play = Rc::clone(&active_bus_name);
        play_btn.connect_clicked(move |_| {
            let target_bus = bus_for_play.borrow().clone();
            std::thread::spawn(move || {
                let _ = execute_mpris_action(target_bus, "PlayPause");
            });
        });

        let bus_for_prev = Rc::clone(&active_bus_name);
        prev_btn.connect_clicked(move |_| {
            let target_bus = bus_for_prev.borrow().clone();
            std::thread::spawn(move || {
                let _ = execute_mpris_action(target_bus, "Previous");
            });
        });

        let bus_for_next = Rc::clone(&active_bus_name);
        next_btn.connect_clicked(move |_| {
            let target_bus = bus_for_next.borrow().clone();
            std::thread::spawn(move || {
                let _ = execute_mpris_action(target_bus, "Next");
            });
        });

        // 1. Background worker thread: continuously polls D-Bus MPRIS every 800ms
        let shared_worker = Arc::clone(&shared_media);
        std::thread::spawn(move || loop {
            let info = query_active_mpris_player();
            if let Ok(mut lock) = shared_worker.lock() {
                *lock = info;
            }
            std::thread::sleep(Duration::from_millis(800));
        });

        // 2. GTK main thread timer: syncs UI from shared_media
        let title_update = title_label.clone();
        let artist_update = artist_label.clone();
        let play_btn_update = play_btn.clone();
        let bus_update = Rc::clone(&active_bus_name);
        let is_active_flag = Arc::clone(&is_active);
        let shared_ui = Arc::clone(&shared_media);
        let last_rendered = Rc::new(RefCell::new(None::<ActiveMediaInfo>));

        glib::timeout_add_local(Duration::from_millis(500), move || {
            let current = shared_ui.lock().ok().and_then(|g| g.clone());
            let mut last = last_rendered.borrow_mut();

            if *last != current {
                *last = current.clone();
                if let Some(info) = current {
                    title_update.set_text(&info.title);
                    if info.is_playing {
                        artist_update.set_text(&info.artist);
                        play_btn_update.set_label("⏸");
                    } else {
                        artist_update.set_text(&format!("{} (Paused)", info.artist));
                        play_btn_update.set_label("▶");
                    }
                    *bus_update.borrow_mut() = Some(info.bus_name);
                    is_active_flag.store(true, Ordering::Relaxed);
                } else {
                    title_update.set_text("No Media Playing");
                    artist_update.set_text("Idle");
                    play_btn_update.set_label("▶");
                    *bus_update.borrow_mut() = None;
                    is_active_flag.store(false, Ordering::Relaxed);
                }
            }

            glib::ControlFlow::Continue
        });

        Self {
            container,
            is_active,
        }
    }
}

/// Dispatches an action (PlayPause, Next, Previous) to the target MPRIS player
fn execute_mpris_action(bus_name: Option<String>, method: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let connection = zbus::Connection::session().await?;

        let target = if let Some(name) = bus_name {
            name
        } else {
            // If bus name not cached, discover any available player
            let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await?;
            let names = dbus_proxy.list_names().await?;
            names
                .into_iter()
                .find(|n| n.starts_with("org.mpris.MediaPlayer2."))
                .map(|n| n.to_string())
                .unwrap_or_default()
        };

        if target.is_empty() {
            return Ok(());
        }

        if let Ok(builder) = MediaPlayer2PlayerProxy::builder(&connection).destination(target.as_str()) {
            if let Ok(player) = builder.build().await {
                match method {
                    "PlayPause" => { let _ = player.play_pause().await; }
                    "Next" => { let _ = player.next().await; }
                    "Previous" => { let _ = player.previous().await; }
                    _ => {}
                }
            }
        }

        Ok(())
    })
}

/// Queries D-Bus session for active MPRIS players (Browsers, Spotify, VLC, MPV, etc.)
fn query_active_mpris_player() -> Option<ActiveMediaInfo> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;

    rt.block_on(async {
        let connection = zbus::Connection::session().await.ok()?;
        let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await.ok()?;
        let names = dbus_proxy.list_names().await.ok()?;

        let mut discovered = Vec::new();

        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                if let Ok(builder) = MediaPlayer2PlayerProxy::builder(&connection).destination(name.as_str()) {
                    if let Ok(proxy) = builder.build().await {
                        let status = proxy.playback_status().await.unwrap_or_else(|_| "Stopped".to_string());
                        let metadata = proxy.metadata().await.unwrap_or_default();

                        let mut title = metadata
                            .get("xesam:title")
                            .map(|v| extract_string_from_variant(v))
                            .unwrap_or_default();

                        let mut artist = metadata
                            .get("xesam:artist")
                            .map(|v| extract_string_from_variant(v))
                            .unwrap_or_default();

                        // Clean up generic/empty titles
                        let clean_bus_name = name.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(&name);
                        let app_name = clean_bus_name.split('.').next().unwrap_or(clean_bus_name);

                        if title.trim().is_empty() {
                            title = format!("{} Playback", capitalize_first(app_name));
                        }

                        if artist.trim().is_empty() {
                            artist = format!("{} (Active)", capitalize_first(app_name));
                        }

                        let is_playing = status.eq_ignore_ascii_case("Playing");

                        discovered.push(ActiveMediaInfo {
                            bus_name: name.to_string(),
                            title,
                            artist,
                            status,
                            is_playing,
                        });
                    }
                }
            }
        }

        // 1. Highest priority: Return player actively "Playing" (e.g. YouTube in Chrome/Firefox)
        if let Some(playing) = discovered.iter().find(|p| p.is_playing) {
            return Some(playing.clone());
        }

        // 2. Next priority: Return player that is "Paused"
        if let Some(paused) = discovered.iter().find(|p| p.status.eq_ignore_ascii_case("Paused")) {
            return Some(paused.clone());
        }

        None
    })
}

/// Recursively extracts a human-readable string from a D-Bus variant
fn extract_string_from_variant(val: &zbus::zvariant::OwnedValue) -> String {
    extract_from_value(val)
}

fn extract_from_value(val: &zbus::zvariant::Value) -> String {
    match val {
        zbus::zvariant::Value::Str(s) => s.to_string(),
        zbus::zvariant::Value::Value(inner) => extract_from_value(inner),
        zbus::zvariant::Value::Array(arr) => {
            let mut list = Vec::new();
            for item in arr.iter() {
                let s = extract_from_value(item);
                if !s.is_empty() {
                    list.push(s);
                }
            }
            list.join(", ")
        }
        _ => String::new(),
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
