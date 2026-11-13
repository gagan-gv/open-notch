use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation, ProgressBar};
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
    pub player_display_name: String,
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
        let container = GtkBox::new(Orientation::Vertical, 10);
        container.set_valign(Align::Center);
        container.set_halign(Align::Center);
        container.add_css_class("mpris-widget");

        // Top Row: Album Cover + Track Metadata
        let top_row = GtkBox::new(Orientation::Horizontal, 14);
        top_row.set_valign(Align::Center);
        top_row.set_halign(Align::Center);

        // Album Art / Vinyl Disc placeholder (BoringNotch style)
        let album_art_box = GtkBox::new(Orientation::Vertical, 0);
        album_art_box.set_size_request(60, 60);
        album_art_box.set_valign(Align::Center);
        album_art_box.add_css_class("mpris-album-art");

        let vinyl_icon = Label::new(Some("🎵"));
        vinyl_icon.set_valign(Align::Center);
        vinyl_icon.set_halign(Align::Center);
        vinyl_icon.add_css_class("mpris-vinyl-icon");
        album_art_box.append(&vinyl_icon);

        // Track Text Info
        let track_info = GtkBox::new(Orientation::Vertical, 3);
        track_info.set_valign(Align::Center);
        track_info.set_halign(Align::Start);

        let app_badge = Label::new(Some("MEDIA"));
        app_badge.set_halign(Align::Start);
        app_badge.add_css_class("mpris-app-badge");

        let title_label = Label::new(Some("No Media Playing"));
        title_label.set_halign(Align::Start);
        title_label.add_css_class("mpris-title");
        title_label.set_wrap(true);
        title_label.set_max_width_chars(32);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        let artist_label = Label::new(Some("Waiting for playback"));
        artist_label.set_halign(Align::Start);
        artist_label.add_css_class("mpris-artist");
        artist_label.set_wrap(true);
        artist_label.set_max_width_chars(32);
        artist_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);

        track_info.append(&app_badge);
        track_info.append(&title_label);
        track_info.append(&artist_label);

        top_row.append(&album_art_box);
        top_row.append(&track_info);

        // Progress line (BoringNotch aesthetic)
        let progress_bar = ProgressBar::new();
        progress_bar.set_fraction(0.35);
        progress_bar.add_css_class("mpris-progress-bar");

        // Media Controls (Apple glass circle buttons)
        let controls = GtkBox::new(Orientation::Horizontal, 18);
        controls.set_halign(Align::Center);
        controls.set_valign(Align::Center);
        controls.add_css_class("mpris-controls-row");

        let prev_btn = Button::with_label("⏮");
        prev_btn.add_css_class("mpris-ctrl-btn");

        let play_btn = Button::with_label("▶");
        play_btn.add_css_class("mpris-play-btn");

        let next_btn = Button::with_label("⏭");
        next_btn.add_css_class("mpris-ctrl-btn");

        controls.append(&prev_btn);
        controls.append(&play_btn);
        controls.append(&next_btn);

        container.append(&top_row);
        container.append(&progress_bar);
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

        // Background worker thread: continuously polls D-Bus MPRIS every 800ms
        let shared_worker = Arc::clone(&shared_media);
        std::thread::spawn(move || loop {
            let info = query_active_mpris_player();
            if let Ok(mut lock) = shared_worker.lock() {
                *lock = info;
            }
            std::thread::sleep(Duration::from_millis(800));
        });

        // GTK main thread timer: syncs UI from shared_media
        let title_update = title_label.clone();
        let artist_update = artist_label.clone();
        let badge_update = app_badge.clone();
        let play_btn_update = play_btn.clone();
        let album_box_update = album_art_box.clone();
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
                    badge_update.set_text(&info.player_display_name.to_uppercase());
                    if info.is_playing {
                        artist_update.set_text(&info.artist);
                        play_btn_update.set_label("⏸");
                        album_box_update.add_css_class("playing");
                    } else {
                        artist_update.set_text(&format!("{} (Paused)", info.artist));
                        play_btn_update.set_label("▶");
                        album_box_update.remove_css_class("playing");
                    }
                    *bus_update.borrow_mut() = Some(info.bus_name);
                    is_active_flag.store(true, Ordering::Relaxed);
                } else {
                    title_update.set_text("No Media Playing");
                    artist_update.set_text("Waiting for playback");
                    badge_update.set_text("IDLE");
                    play_btn_update.set_label("▶");
                    album_box_update.remove_css_class("playing");
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

                        // App badge name (Firefox, Brave, Spotify, Chrome)
                        let clean_bus_name = name.strip_prefix("org.mpris.MediaPlayer2.").unwrap_or(&name);
                        let app_name = clean_bus_name.split('.').next().unwrap_or(clean_bus_name);
                        let player_display_name = match app_name.to_lowercase().as_str() {
                            "firefox" => "🦊 Firefox".to_string(),
                            "chromium" => "🌐 Chromium".to_string(),
                            "chrome" | "google-chrome" => "🌐 Chrome".to_string(),
                            "brave" => "🦁 Brave".to_string(),
                            "spotify" => "🟢 Spotify".to_string(),
                            "vlc" => "🍊 VLC".to_string(),
                            other => capitalize_first(other),
                        };

                        if title.trim().is_empty() {
                            title = format!("{} Playback", capitalize_first(app_name));
                        }

                        if artist.trim().is_empty() {
                            artist = "Streaming Audio/Video".to_string();
                        }

                        let is_playing = status.eq_ignore_ascii_case("Playing");

                        discovered.push(ActiveMediaInfo {
                            bus_name: name.to_string(),
                            player_display_name,
                            title,
                            artist,
                            status,
                            is_playing,
                        });
                    }
                }
            }
        }

        // 1. Highest priority: Return player actively "Playing"
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
