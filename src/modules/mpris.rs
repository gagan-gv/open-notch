use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct MprisWidget {
    pub container: GtkBox,
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

        let artist_label = Label::new(Some("Idle"));
        artist_label.add_css_class("mpris-artist");

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

        // Connect button signals (D-Bus MPRIS command calls)
        let is_playing = Rc::new(RefCell::new(false));
        let play_btn_clone = play_btn.clone();
        let is_playing_clone = Rc::clone(&is_playing);

        play_btn.connect_clicked(move |_| {
            let mut playing = is_playing_clone.borrow_mut();
            *playing = !*playing;
            if *playing {
                play_btn_clone.set_label("⏸");
            } else {
                play_btn_clone.set_label("▶");
            }
            // Trigger async D-Bus PlayPause command
            std::thread::spawn(|| {
                let _ = send_mpris_command("PlayPause");
            });
        });

        prev_btn.connect_clicked(|_| {
            std::thread::spawn(|| {
                let _ = send_mpris_command("Previous");
            });
        });

        next_btn.connect_clicked(|_| {
            std::thread::spawn(|| {
                let _ = send_mpris_command("Next");
            });
        });

        Self {
            container,
            is_active,
        }
    }
}

/// Dispatches an MPRIS player command via D-Bus session
fn send_mpris_command(method: &str) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let connection = zbus::Connection::session().await?;
        // Find players on the bus
        let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await?;
        let names = dbus_proxy.list_names().await?;

        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                let _: () = connection
                    .call_method(
                        Some(name.as_str()),
                        "/org/mpris/MediaPlayer2",
                        Some("org.mpris.MediaPlayer2.Player"),
                        method,
                        &(),
                    )
                    .await?
                    .body()?;
                break;
            }
        }
        Ok(())
    })
}
