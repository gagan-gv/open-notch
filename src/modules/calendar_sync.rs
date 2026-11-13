use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub title: String,
    pub time: String, // e.g. "10:00 AM" or "All Day"
    pub date: String, // "YYYY-MM-DD" or "today"
    #[serde(default)]
    pub uid: Option<String>,
    #[serde(default = "default_source")]
    pub source: String, // "local" or "system"
}

fn default_source() -> String {
    "local".to_string()
}

/// Unfolds RFC 5545 iCalendar lines (lines starting with space/tab are continuations)
fn unfold_ics(content: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in content.lines() {
        let trimmed_end = raw.trim_end_matches('\r');
        if (trimmed_end.starts_with(' ') || trimmed_end.starts_with('\t')) && !lines.is_empty() {
            let last: &mut String = lines.last_mut().unwrap();
            last.push_str(&trimmed_end[1..]);
        } else if !trimmed_end.is_empty() {
            lines.push(trimmed_end.to_string());
        }
    }
    lines
}

fn unescape_ics(val: &str) -> String {
    val.replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\n", " ")
        .replace("\\N", " ")
        .replace("\\\\", "\\")
}

/// Parses DTSTART string into (YYYY-MM-DD, HH:MM AM/PM)
pub fn parse_ics_dtstart(val: &str) -> (String, String) {
    let val = val.trim();
    if val.len() < 8 {
        return (String::new(), String::new());
    }

    let year = &val[0..4];
    let month = &val[4..6];
    let day = &val[6..8];
    let date = format!("{}-{}-{}", year, month, day);

    let time = if val.contains('T') {
        let after_t = val.split('T').nth(1).unwrap_or("");
        if after_t.len() >= 4 {
            let h: u32 = after_t[0..2].parse().unwrap_or(0);
            let m: u32 = after_t[2..4].parse().unwrap_or(0);
            let period = if h >= 12 { "PM" } else { "AM" };
            let display_h = if h == 0 {
                12
            } else if h > 12 {
                h - 12
            } else {
                h
            };
            format!("{:02}:{:02} {}", display_h, m, period)
        } else {
            "All Day".to_string()
        }
    } else {
        "All Day".to_string()
    };

    (date, time)
}

/// Formats date and user-typed time string into RFC 5545 DTSTART and DTEND parameters
pub fn format_time_to_ics(date_str: &str, time_str: &str) -> (String, String) {
    let clean_date = date_str.replace('-', "");
    let time_lower = time_str.trim().to_lowercase();

    if time_lower == "all day" || time_lower.is_empty() {
        return (
            format!(";VALUE=DATE:{}", clean_date),
            format!(";VALUE=DATE:{}", clean_date),
        );
    }

    let is_pm = time_lower.contains("pm");
    let is_am = time_lower.contains("am");

    let nums_only: String = time_lower
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ':')
        .collect();

    let mut hour = 10u32;
    let mut min = 0u32;
    let mut parsed = false;

    if let Some((h_str, m_str)) = nums_only.split_once(':') {
        if let (Ok(h), Ok(m)) = (h_str.parse::<u32>(), m_str.parse::<u32>()) {
            hour = h;
            min = m;
            parsed = true;
        }
    } else if let Ok(h) = nums_only.parse::<u32>() {
        hour = h;
        min = 0;
        parsed = true;
    }

    if parsed {
        if is_pm && hour < 12 {
            hour += 12;
        } else if is_am && hour == 12 {
            hour = 0;
        }
        let dtstart = format!(":{}{:02}{:02}00", clean_date, hour % 24, min % 60);
        let end_h = (hour + 1) % 24;
        let dtend = format!(":{}{:02}{:02}00", clean_date, end_h, min % 60);
        (dtstart, dtend)
    } else {
        (
            format!(";VALUE=DATE:{}", clean_date),
            format!(";VALUE=DATE:{}", clean_date),
        )
    }
}

/// Parses an .ics file and appends events matching `target_date`
fn parse_ics_file(path: &PathBuf, target_date: &str, events: &mut Vec<CalendarEvent>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines = unfold_ics(&content);
    let mut in_vevent = false;
    let mut cur_summary = String::new();
    let mut cur_date = String::new();
    let mut cur_time = String::new();
    let mut cur_uid = String::new();

    for line in lines {
        if line.starts_with("BEGIN:VEVENT") {
            in_vevent = true;
            cur_summary.clear();
            cur_date.clear();
            cur_time.clear();
            cur_uid.clear();
        } else if line.starts_with("END:VEVENT") {
            if in_vevent && !cur_summary.is_empty() {
                if cur_date == target_date || cur_date.is_empty() {
                    let display_time = if cur_time.is_empty() {
                        "All Day".to_string()
                    } else {
                        cur_time.clone()
                    };
                    events.push(CalendarEvent {
                        title: cur_summary.clone(),
                        time: display_time,
                        date: target_date.to_string(),
                        uid: Some(cur_uid.clone()),
                        source: "system".to_string(),
                    });
                }
            }
            in_vevent = false;
        } else if in_vevent {
            if line.starts_with("SUMMARY") {
                if let Some((_, val)) = line.split_once(':') {
                    cur_summary = unescape_ics(val.trim());
                }
            } else if line.starts_with("UID") {
                if let Some((_, val)) = line.split_once(':') {
                    cur_uid = val.trim().to_string();
                }
            } else if line.starts_with("DTSTART") {
                if let Some((_, val)) = line.split_once(':') {
                    let (parsed_d, parsed_t) = parse_ics_dtstart(val);
                    cur_date = parsed_d;
                    cur_time = parsed_t;
                }
            }
        }
    }
}

/// Loads all system calendar events matching `target_date` (e.g. "2026-11-14")
pub fn load_system_calendar_events(target_date: &str) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return events,
    };

    // Scan standard Evolution / GNOME Calendar stores
    let base_dirs = [
        home.join(".local/share/evolution/calendar"),
        home.join(".cache/evolution/calendar"),
    ];

    for base in &base_dirs {
        if !base.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let sub_path = entry.path();
                if sub_path.is_dir() {
                    let ics_file = sub_path.join("calendar.ics");
                    if ics_file.exists() {
                        parse_ics_file(&ics_file, target_date, &mut events);
                    }
                } else if sub_path.extension().map_or(false, |ext| ext == "ics") {
                    parse_ics_file(&sub_path, target_date, &mut events);
                }
            }
        }
    }

    events
}

/// Syncs an event by adding it to the system's Evolution / GNOME Calendar store
pub fn add_event_to_system_calendar(title: &str, time: &str, date_str: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    let system_ics_path = home.join(".local/share/evolution/calendar/system/calendar.ics");

    if let Some(parent) = system_ics_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let (dtstart, dtend) = format_time_to_ics(date_str, time);
    let now_utc = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let uid = format!(
        "opn-{}-{}@open-notch",
        Utc::now().timestamp_micros(),
        std::process::id()
    );

    let vevent = format!(
        "BEGIN:VEVENT\nUID:{uid}\nDTSTAMP:{now_utc}\nDTSTART{dtstart}\nDTEND{dtend}\nSUMMARY:{title}\nDESCRIPTION:Added via OpenNotch\nCATEGORIES:OpenNotch\nEND:VEVENT\n"
    );

    let existing = fs::read_to_string(&system_ics_path).unwrap_or_default();
    let updated = if existing.contains("END:VCALENDAR") {
        if let Some(pos) = existing.rfind("END:VCALENDAR") {
            let mut s = existing[..pos].trim_end().to_string();
            s.push('\n');
            s.push_str(&vevent);
            s.push_str("END:VCALENDAR\n");
            s
        } else {
            format!(
                "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//OpenNotch//EN\nCALSCALE:GREGORIAN\n{}END:VCALENDAR\n",
                vevent
            )
        }
    } else {
        format!(
            "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//OpenNotch//EN\nCALSCALE:GREGORIAN\n{}END:VCALENDAR\n",
            vevent
        )
    };

    let _ = fs::write(&system_ics_path, updated);
    Some(uid)
}

/// Removes an event from the system calendar by UID or title match
pub fn remove_event_from_system_calendar(uid: Option<&str>, title: &str) {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };
    let system_ics_path = home.join(".local/share/evolution/calendar/system/calendar.ics");
    if !system_ics_path.exists() {
        return;
    }

    let content = match fs::read_to_string(&system_ics_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut filtered_lines = Vec::new();
    let mut in_block = false;
    let mut block_lines = Vec::new();
    let mut block_matches = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "BEGIN:VEVENT" {
            in_block = true;
            block_lines.clear();
            block_matches = false;
            block_lines.push(line);
        } else if trimmed == "END:VEVENT" {
            block_lines.push(line);
            in_block = false;
            if !block_matches {
                filtered_lines.extend(block_lines.drain(..));
            } else {
                block_lines.clear();
            }
        } else if in_block {
            block_lines.push(line);
            if let Some(target_uid) = uid {
                if trimmed.starts_with("UID:") && trimmed.contains(target_uid) {
                    block_matches = true;
                }
            }
            if trimmed.starts_with("SUMMARY:") && trimmed.contains(title) {
                block_matches = true;
            }
        } else {
            filtered_lines.push(line);
        }
    }

    let mut new_content = filtered_lines.join("\n");
    if !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    let _ = fs::write(&system_ics_path, new_content);
}

/// Launches the system's installed calendar app (Evolution or GNOME Calendar)
pub fn launch_system_calendar() {
    let _ = std::process::Command::new("evolution")
        .arg("-c")
        .arg("calendar")
        .spawn()
        .or_else(|_| std::process::Command::new("gnome-calendar").spawn());
}
