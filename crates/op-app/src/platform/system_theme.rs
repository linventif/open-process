//! Linux desktops publish their light/dark preference through the freedesktop
//! settings portal, which winit does not report on X11 or Wayland. A background thread reads
//! it and follows changes; the result becomes egui's fallback theme, which egui
//! uses whenever the windowing system reports no theme of its own.
use std::sync::atomic::{AtomicU8, Ordering};
use zbus::zvariant::Value;

const UNKNOWN: u8 = 0;
const DARK: u8 = 1;
const LIGHT: u8 = 2;
static SCHEME: AtomicU8 = AtomicU8::new(UNKNOWN);

/// Starts following the portal; without a session bus or portal nothing changes.
pub fn watch(ctx: egui::Context) {
    let _ = std::thread::Builder::new().name("system-theme".into()).spawn(move || {
        if let Err(error) = follow(&ctx) {
            log::debug!("Desktop color scheme unavailable: {error}");
        }
    });
}

/// The desktop's preference, once the portal has answered.
pub fn current() -> Option<egui::Theme> {
    match SCHEME.load(Ordering::Relaxed) {
        DARK => Some(egui::Theme::Dark),
        LIGHT => Some(egui::Theme::Light),
        _ => None,
    }
}

fn follow(ctx: &egui::Context) -> zbus::Result<()> {
    let connection = zbus::blocking::Connection::session()?;
    let settings = zbus::blocking::Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    )?;
    // Subscribe first so that a change made while reading is not missed.
    let changes = settings.receive_signal("SettingChanged")?;
    let key = ("org.freedesktop.appearance", "color-scheme");
    let initial: zbus::zvariant::OwnedValue = settings.call("ReadOne", &key).or_else(|_| settings.call("Read", &key))?;
    store(&initial, ctx);
    for message in changes {
        if let Ok((namespace, name, value)) = message.body().deserialize::<(String, String, zbus::zvariant::OwnedValue)>()
            && (namespace.as_str(), name.as_str()) == key
        {
            store(&value, ctx);
        }
    }
    Ok(())
}

fn store(value: &Value<'_>, ctx: &egui::Context) {
    SCHEME.store(scheme(value), Ordering::Relaxed);
    ctx.request_repaint();
}

/// Portal values: 1 prefers dark, 2 prefers light, 0 has no preference (light).
/// The deprecated `Read` method wraps the value in one more variant.
fn scheme(value: &Value<'_>) -> u8 {
    match value {
        Value::U32(1) => DARK,
        Value::U32(_) => LIGHT,
        Value::Value(inner) => scheme(inner),
        _ => UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_values_map_to_themes() {
        assert_eq!(scheme(&Value::U32(1)), DARK);
        assert_eq!(scheme(&Value::U32(2)), LIGHT);
        assert_eq!(scheme(&Value::U32(0)), LIGHT, "no preference uses the light default");
        assert_eq!(scheme(&Value::Value(Box::new(Value::U32(1)))), DARK);
        assert_eq!(scheme(&Value::from("dark")), UNKNOWN);
    }
}
