use glib::translate::ToGlibPtr;
use gtk4::cairo::{RectangleInt, Region};
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::ApplicationWindow;
use std::ffi::{c_char, c_int, c_ulong, c_void, CString};

extern "C" {
    fn XMoveResizeWindow(
        display: *mut c_void,
        w: c_ulong,
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
    ) -> c_int;
    fn XFlush(display: *mut c_void) -> c_int;
    fn XInternAtom(
        display: *mut c_void,
        atom_name: *const c_char,
        only_if_exists: c_int,
    ) -> c_ulong;
    fn XChangeProperty(
        display: *mut c_void,
        w: c_ulong,
        property: c_ulong,
        prop_type: c_ulong,
        format: c_int,
        mode: c_int,
        data: *const u8,
        nelements: c_int,
    ) -> c_int;
    fn XSetInputFocus(
        display: *mut c_void,
        focus: c_ulong,
        revert_to: c_int,
        time: c_ulong,
    ) -> c_int;
    fn XSendEvent(
        display: *mut c_void,
        w: c_ulong,
        propagate: c_int,
        event_mask: std::ffi::c_long,
        event_send: *mut c_void,
    ) -> c_int;
    fn XDefaultRootWindow(display: *mut c_void) -> c_ulong;
}

#[repr(C)]
struct XClientMessageEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut c_void,
    window: c_ulong,
    message_type: c_ulong,
    format: c_int,
    data: [std::ffi::c_long; 5],
}

static CURRENT_XID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Applies EWMH Dock & Always-On-Top properties and anchors the window at top-center
pub fn position_x11_top_center(window: &ApplicationWindow, width: i32, height: i32) {
    let display = match Display::default() {
        Some(d) => d,
        None => return,
    };

    let x11_display = match display.downcast_ref::<gdk4_x11::X11Display>() {
        Some(d) => d,
        None => return,
    };

    let surface = match window.surface() {
        Some(s) => s,
        None => return,
    };

    let x11_surface = match surface.downcast_ref::<gdk4_x11::X11Surface>() {
        Some(s) => s,
        None => return,
    };

    unsafe {
        let xdisplay = gdk4_x11::ffi::gdk_x11_display_get_xdisplay(x11_display.to_glib_none().0);
        let xid = x11_surface.xid();
        CURRENT_XID.store(xid, std::sync::atomic::Ordering::Relaxed);

        // Query monitor width to calculate center
        let monitor = display
            .monitors()
            .item(0)
            .and_downcast::<gtk4::gdk::Monitor>();
        let screen_w = monitor.map(|m| m.geometry().width()).unwrap_or(1920);

        let effective_w = width.max(window.width());
        let target_x = (screen_w - effective_w) / 2;
        let target_y = 0;

        let atom_atom = XInternAtom(xdisplay, CString::new("ATOM").unwrap().as_ptr(), 0);

        // 1. _NET_WM_WINDOW_TYPE = _NET_WM_WINDOW_TYPE_DOCK (Keeps window above Mutter panel at y=0)
        let net_wm_window_type = CString::new("_NET_WM_WINDOW_TYPE").unwrap();
        let atom_window_type = XInternAtom(xdisplay, net_wm_window_type.as_ptr(), 0);

        let net_wm_type_dock = CString::new("_NET_WM_WINDOW_TYPE_DOCK").unwrap();
        let atom_type_dock = XInternAtom(xdisplay, net_wm_type_dock.as_ptr(), 0);

        let dock_atoms = [atom_type_dock];
        XChangeProperty(
            xdisplay,
            xid,
            atom_window_type,
            atom_atom,
            32,
            0, // PropModeReplace
            dock_atoms.as_ptr() as *const u8,
            1,
        );

        // 2. _NET_WM_STATE = _NET_WM_STATE_ABOVE
        let net_wm_state = CString::new("_NET_WM_STATE").unwrap();
        let atom_state = XInternAtom(xdisplay, net_wm_state.as_ptr(), 0);

        let net_wm_state_above = CString::new("_NET_WM_STATE_ABOVE").unwrap();
        let atom_state_above = XInternAtom(xdisplay, net_wm_state_above.as_ptr(), 0);

        let state_atoms = [atom_state_above];
        XChangeProperty(
            xdisplay,
            xid,
            atom_state,
            atom_atom,
            32,
            0, // PropModeReplace
            state_atoms.as_ptr() as *const u8,
            1,
        );

        // 3. Move and size window to top-center
        XMoveResizeWindow(xdisplay, xid, target_x, target_y, effective_w, height.max(450));
        XFlush(xdisplay);
    }
}

/// Gives direct X11 keyboard input focus to the notch window
pub fn grab_x11_input_focus() {
    let xid = CURRENT_XID.load(std::sync::atomic::Ordering::Relaxed);
    if xid == 0 {
        return;
    }

    let display = match Display::default() {
        Some(d) => d,
        None => return,
    };

    let x11_display = match display.downcast_ref::<gdk4_x11::X11Display>() {
        Some(d) => d,
        None => return,
    };

    unsafe {
        let xdisplay = gdk4_x11::ffi::gdk_x11_display_get_xdisplay(x11_display.to_glib_none().0);
        let root = XDefaultRootWindow(xdisplay);

        // Send _NET_ACTIVE_WINDOW ClientMessage to root window (notifies Mutter to route focus to dock)
        let net_active_window = CString::new("_NET_ACTIVE_WINDOW").unwrap();
        let atom_active = XInternAtom(xdisplay, net_active_window.as_ptr(), 0);

        let mut ev: XClientMessageEvent = std::mem::zeroed();
        ev.type_ = 33; // ClientMessage
        ev.window = xid as c_ulong;
        ev.message_type = atom_active;
        ev.format = 32;
        ev.data[0] = 2; // source indication: 2 = direct user interaction
        ev.data[1] = 0; // CurrentTime

        let mask = 0x0010_0000 | 0x0008_0000; // SubstructureNotifyMask | SubstructureRedirectMask
        XSendEvent(xdisplay, root, 0, mask, &mut ev as *mut _ as *mut c_void);

        // Also directly tell X server to focus this window
        XSetInputFocus(xdisplay, xid as c_ulong, 2, 0);
        XFlush(xdisplay);
    }
}

/// Dynamically updates the clickable input mask:
/// - When collapsed: only the notch pill receives mouse events; all empty areas pass clicks through.
/// - When expanded: the entire shelf receives mouse events.
pub fn update_notch_input_region(
    window: &ApplicationWindow,
    expanded: bool,
    collapsed_w: i32,
    collapsed_h: i32,
    expanded_w: i32,
    expanded_h: i32,
) {
    if let Some(surface) = window.surface() {
        let win_w = expanded_w.max(window.width());
        let rect = if expanded {
            // Generous bounding box so child widgets never clip click or scroll events
            RectangleInt::new(0, 0, win_w, expanded_h.max(480))
        } else {
            let offset_x = (win_w - collapsed_w) / 2;
            RectangleInt::new(offset_x, 0, collapsed_w, collapsed_h + 4)
        };
        let region = Region::create_rectangle(&rect);
        surface.set_input_region(&region);
    }
}
