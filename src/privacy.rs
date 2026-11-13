use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivacyState {
    Idle,
    MicrophoneActive,
    CameraActive,
}

/// Detects camera and microphone usage on Linux.
/// Camera has higher priority: if both are active, CameraActive is returned.
pub fn check_privacy_status() -> PrivacyState {
    if is_camera_active() {
        return PrivacyState::CameraActive;
    }

    if is_microphone_active() {
        return PrivacyState::MicrophoneActive;
    }

    PrivacyState::Idle
}

/// Checks if any video capture device (/dev/video*) is currently open.
fn is_camera_active() -> bool {
    // 1. Check using fuser command
    if let Ok(output) = Command::new("fuser")
        .args(["/dev/video0", "/dev/video1", "/dev/video2", "/dev/video3"])
        .output()
    {
        if output.status.success() {
            return true;
        }
    }

    // 2. Fallback: inspect /proc/*/fd for open /dev/video handles
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            if file_name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                let mut fd_path = entry.path();
                fd_path.push("fd");
                if let Ok(fds) = fs::read_dir(fd_path) {
                    for fd in fds.flatten() {
                        if let Ok(target) = fs::read_link(fd.path()) {
                            if target.to_string_lossy().starts_with("/dev/video") {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

/// Checks if any audio capture device / microphone is currently streaming.
fn is_microphone_active() -> bool {
    // 1. Check ALSA kernel capture status (/proc/asound/card*/pcm*c/sub*/status)
    if let Ok(cards) = fs::read_dir("/proc/asound") {
        for card in cards.flatten() {
            let card_path = card.path();
            if card_path.is_dir() && card.file_name().to_string_lossy().starts_with("card") {
                if let Ok(pcms) = fs::read_dir(&card_path) {
                    for pcm in pcms.flatten() {
                        let pcm_name = pcm.file_name().to_string_lossy().to_string();
                        // Suffix 'c' denotes Capture (microphone)
                        if pcm_name.starts_with("pcm") && pcm_name.ends_with('c') {
                            let mut sub_path = pcm.path();
                            sub_path.push("sub0");
                            sub_path.push("status");
                            if let Ok(status) = fs::read_to_string(sub_path) {
                                if status.trim().to_uppercase().contains("RUNNING") {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. PulseAudio / PipeWire check via pactl
    if let Ok(output) = Command::new("pactl")
        .args(["list", "short", "source-outputs"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.trim().is_empty() {
                return true;
            }
        }
    }

    false
}
