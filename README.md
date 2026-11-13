# OpenNotch

An interactive desktop notch overlay built for Linux environments. OpenNotch sits anchored at the top-center of your display, remaining discreet while idle and expanding on cursor interaction into an interactive control shelf.

---

## Key Features

- **Top-Center Floating Anchor**: Stays persistently positioned at the top-center edge of the screen across desktop sessions.
- **Hover-Driven Expansion**: Unfolds automatically when hovered to reveal controls, widgets, and status information.
- **Automatic Collapse**: Smoothly reverts to its compact resting pill dimensions once the cursor moves outside the notch area.
- **Universal Media Player Controls**: Communicates via standard Linux D-Bus MPRIS protocols to display playback status, track metadata, album art, and controls (play/pause, seek, skip) for browsers, Spotify, and local players.
- **Idle Calendar View**: Automatically displays a calendar and date overview whenever no active media stream is detected.
- **Integrated Pomodoro Timer**: Dedicated interval countdown utility with work and break intervals to support focused working sessions.
- **Quick Notes Scratchpad**: Rapid note-taking area for drafting thoughts, to-do items, and temporary clipboard clippings.

---

## Interaction Model

```
   [ Resting State (Top-Center Pill) ]
                 │
           (Cursor Enters)
                 ▼
   ┌──────────────────────────────────────────────┐
   │             [ Expanded Shelf ]               │
   │                                              │
   │  [ Active Mode: Media / Calendar Glance ]    │
   │  [ Quick Tools: Pomodoro Timer | Notes ]     │
   └──────────────────────────────────────────────┘
                 │
           (Cursor Leaves)
                 ▼
   [ Returns to Resting State ]
```

---

## Window Layering & Display Protocol Strategy

Linux environments operate across multiple window managers and display protocols. OpenNotch accounts for both modern Wayland compositors and standard X11 setups:

| Display Protocol | Layering Mechanism | Target Environments |
| :--- | :--- | :--- |
| **Wayland** | `wlr-layer-shell` protocol (`top` / `overlay` layer) | Hyprland, Sway, KDE Plasma (Wayland), River |
| **X11** | Extended Window Manager Hints (EWMH) `_NET_WM_WINDOW_TYPE_DOCK`, `_NET_WM_STATE_ABOVE` | XFCE, i3, bspwm, Openbox, GNOME (X11) |

---

## Tech Stack & Architecture

- **Language & Framework**: Rust with [GTK4](https://gtk-rs.org/)
- **Window Layering**: [`gtk4-layer-shell`](https://crates.io/crates/gtk4-layer-shell) for Wayland overlays + EWMH dock fallback
- **Media Control**: D-Bus MPRIS client via [`zbus`](https://crates.io/crates/zbus)
- **Styling**: GTK4 CSS with glassmorphic transparency and easing animations
- **Configuration**: TOML via `serde` (`~/.config/open-notch/config.toml`)

---

## Getting Started

### System Prerequisites (Ubuntu / Debian / Zorin OS)

```bash
# Install GTK4 and build dependencies
sudo apt update && sudo apt install -y build-essential libgtk-4-dev pkg-config

# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Build & Run

```bash
# Clone the repository
git clone https://github.com/your-username/open-notch.git
cd open-notch

# Run in debug mode
cargo run

# Build optimized release binary
cargo build --release
```

---

## Configuration Example

The application uses a plain-text configuration file located at `~/.config/open-notch/config.toml`:

```toml
[notch]
monitor = "primary"
collapsed_width = 180
collapsed_height = 30
expanded_width = 520
expanded_height = 210
animation_duration_ms = 200
leave_debounce_ms = 250

[behavior]
default_idle_view = "calendar"
mpris_auto_expand = false

[pomodoro]
focus_duration = 25
break_duration = 5
sound_alerts = true
```
