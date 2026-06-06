//! Daily-popup scheduling helpers.
//!
//! Two concerns, both kept pure so they unit-test on any platform:
//! 1. the once-per-day gate ([`should_show`]) — the in-app source of truth that
//!    prevents the popup appearing more than once per local calendar day.
//! 2. launchd plist text generation ([`launchd_plist`]) — emitted to
//!    `~/Library/LaunchAgents` on macOS so the app can be launched at a set hour.
//!
//! The actual filesystem/launchctl side-effects live in the macOS `platform`
//! layer of the Tauri crate; here we only compute strings.

use chrono::Local;

/// Today's local date as "YYYY-MM-DD".
pub fn today_local() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Whether the daily popup should be shown, given the last date it was shown
/// (from settings) and today's date. Shows once per new calendar day.
pub fn should_show(last_shown: Option<&str>, today: &str) -> bool {
    match last_shown {
        Some(d) => d != today,
        None => true,
    }
}

/// Generate a launchd LaunchAgent plist that launches `program_path` every day
/// at `hour:00` (and at load). Write this to
/// `~/Library/LaunchAgents/<label>.plist` and `launchctl load` it.
pub fn launchd_plist(label: &str, program_path: &str, hour: u8) -> String {
    let hour = hour.min(23);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{program_path}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>{hour}</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
</dict>
</plist>
"#,
        label = label,
        program_path = program_path,
        hour = hour,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_when_never_shown() {
        assert!(should_show(None, "2026-06-06"));
    }

    #[test]
    fn shows_once_per_day() {
        assert!(!should_show(Some("2026-06-06"), "2026-06-06"));
        assert!(should_show(Some("2026-06-05"), "2026-06-06"));
    }

    #[test]
    fn plist_contains_schedule_and_clamps_hour() {
        let p = launchd_plist("com.knowledgeapp.app", "/Applications/Knowledge.app/Contents/MacOS/knowledge-app", 30);
        assert!(p.contains("<key>Hour</key>"));
        assert!(p.contains("<integer>23</integer>")); // clamped
        assert!(p.contains("com.knowledgeapp.app"));
        assert!(p.contains("StartCalendarInterval"));
    }
}
