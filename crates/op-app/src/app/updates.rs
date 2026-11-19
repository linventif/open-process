//! Desktop update notices; one background check per launch, with a manual retry.

use crate::platform::updates::{self, AvailableUpdate, Check};
use op_core::i18n::{tr, trf};

#[derive(Default)]
pub(super) struct Updates {
    check: Option<Check>,
    manual: bool,
    notice: Option<Notice>,
}

enum Notice {
    Available(AvailableUpdate),
    Current,
    Failed,
}

impl Updates {
    pub(super) fn start(&mut self, ctx: &egui::Context, manual: bool) {
        if self.check.is_some() {
            return;
        }
        let installed = match updates::installed_release() {
            Some(tag) => tag,
            None if !manual => return,
            None => concat!("v", env!("CARGO_PKG_VERSION")),
        };
        self.manual = manual;
        self.notice = None;
        match Check::start(ctx.clone(), installed) {
            Ok(check) => self.check = Some(check),
            Err(error) => self.complete(Err(error.to_string())),
        }
    }

    pub(super) fn menu(&mut self, ui: &mut egui::Ui) {
        if self.check.is_some() {
            ui.add_enabled(false, egui::Button::new(tr("updates.checking")));
        } else if ui.button(tr("updates.check")).clicked() {
            self.start(ui.ctx(), true);
            ui.close();
        }
    }

    pub(super) fn poll(&mut self) {
        if let Some(result) = self.check.as_ref().and_then(Check::poll) {
            self.check = None;
            self.complete(result);
        }
    }

    fn complete(&mut self, result: Result<Option<AvailableUpdate>, String>) {
        self.notice = match result {
            Ok(Some(update)) => Some(Notice::Available(update)),
            Ok(None) if self.manual => Some(Notice::Current),
            Err(error) => {
                log::debug!("Release check failed: {error}");
                self.manual.then_some(Notice::Failed)
            }
            Ok(None) => None,
        };
    }

    pub(super) fn is_open(&self) -> bool {
        self.notice.is_some()
    }

    pub(super) fn dialog(&mut self, ctx: &egui::Context) {
        let Some(notice) = &self.notice else { return };
        let mut close = false;
        let response = egui::Modal::new(egui::Id::unique("software_update")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 48.0).clamp(160.0, 420.0));
            match notice {
                Notice::Available(update) => {
                    ui.heading(tr("updates.available"));
                    let installed = updates::installed_release().unwrap_or(env!("CARGO_PKG_VERSION"));
                    ui.label(trf("updates.installed", &[&installed]));
                    ui.label(trf("updates.latest", &[&update.tag]));
                    ui.add_space(8.0);
                    ui.label(tr("updates.manual_install"));
                    ui.hyperlink_to(tr("updates.download"), &update.download_url);
                    ui.hyperlink_to(tr("updates.release_notes"), &update.release_url);
                }
                Notice::Current => {
                    ui.heading(tr("updates.title"));
                    ui.label(tr("updates.current"));
                    ui.hyperlink_to(tr("updates.releases"), updates::RELEASES_URL);
                }
                Notice::Failed => {
                    ui.heading(tr("updates.title"));
                    ui.label(tr("updates.failed"));
                    ui.hyperlink_to(tr("updates.releases"), updates::RELEASES_URL);
                }
            }
            ui.separator();
            let label = if matches!(notice, Notice::Available(_)) {
                tr("updates.later")
            } else {
                tr("common.close")
            };
            close = ui.button(label).clicked();
        });
        if close || response.should_close() {
            self.notice = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::OpenProcessApp;
    use egui_kittest::{Harness, kittest::Queryable};
    use op_core::i18n::{Lang, set_lang};

    fn update() -> AvailableUpdate {
        AvailableUpdate {
            tag: "v9.0.0".into(),
            release_url: format!("{}/tag/v9.0.0", updates::RELEASES_URL),
            download_url: format!("{}/download/v9.0.0/open-process-x86_64-pc-windows-msvc.exe", updates::RELEASES_URL),
        }
    }

    fn harness() -> Harness<'static, OpenProcessApp> {
        let mut h = Harness::builder()
            .with_size([1000.0, 800.0])
            .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), crate::composition::registry()));
        h.run_steps(3);
        assert!(h.state().updates.check.is_none(), "UI tests never start network requests");
        h
    }

    #[test]
    fn popup_opens_download_and_release_links_and_stays_dismissed() {
        let mut h = harness();
        h.state_mut().open_example("bpmn.order");
        let before = h.state().docs[0].snapshot();
        h.state_mut().updates.complete(Ok(Some(update())));
        h.run_steps(4);
        assert!(h.query_by_label("A new version is available").is_some());
        let installed = updates::installed_release().unwrap_or(env!("CARGO_PKG_VERSION"));
        assert!(h.query_by_label(&trf("updates.installed", &[&installed])).is_some());
        assert!(h.query_by_label("Available version: v9.0.0").is_some());
        assert!(h.state().thumbnail_capture_blocked(&h.ctx));
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
        h.run_steps(3);
        assert!(h.state().new_picker.is_none(), "the update dialog owns keyboard input");
        crate::app::tests::shot(&mut h, "desktop_update_available");
        for (label, expected) in [
            ("Download for this computer", update().download_url),
            ("Release notes and other downloads", update().release_url),
        ] {
            h.get_by_label(label).click();
            let mut opened = Vec::new();
            for _ in 0..5 {
                h.step();
                for command in &h.output().platform_output.commands {
                    if let egui::OutputCommand::OpenUrl(url) = command {
                        opened.push(url.url.clone());
                    }
                }
            }
            assert_eq!(opened, [expected]);
            assert!(h.state().updates.is_open());
        }
        h.get_by_label("Later").click();
        h.run_steps(8);
        assert!(!h.state().updates.is_open());
        assert_eq!(h.state().docs[0].snapshot(), before);
        assert!(!h.state().show_home);
        assert!(h.query_by_label("A new version is available").is_none());
    }

    #[test]
    fn automatic_checks_are_quiet_without_updates_and_manual_checks_explain_failures() {
        let mut h = harness();
        h.state_mut().updates.complete(Err("offline".into()));
        h.run_steps(3);
        assert!(!h.state().updates.is_open());
        h.state_mut().updates.complete(Ok(None));
        h.run_steps(3);
        assert!(!h.state().updates.is_open());
        h.state_mut().updates.manual = true;
        h.state_mut().updates.complete(Err("offline".into()));
        h.run_steps(3);
        assert!(h.query_by_label(tr("updates.failed")).is_some());
        assert!(h.query_by_label(tr("updates.releases")).is_some());
        h.get_by_label("Close").click();
        h.run_steps(3);
        assert!(!h.state().updates.is_open());
        h.state_mut().updates.complete(Ok(None));
        h.run_steps(3);
        assert!(h.query_by_label(tr("updates.current")).is_some());
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        assert!(!h.state().updates.is_open());
        assert!(h.state().show_home);
        assert!(h.state().docs.is_empty());
    }

    #[test]
    fn popup_waits_for_document_picker_and_uses_the_saved_interface_language() {
        for language in Lang::ALL {
            let mut h = harness();
            set_lang(language);
            h.state_mut().open_new_picker(None);
            h.state_mut().updates.complete(Ok(Some(update())));
            h.run_steps(3);
            assert!(h.state().updates.is_open());
            assert!(h.query_by_label(tr("updates.available")).is_none());
            h.get_by_label(tr("common.cancel")).click();
            h.run_steps(5);
            assert!(h.query_by_label(tr("updates.available")).is_some());
            assert!(h.query_by_label(tr("updates.download")).is_some());
            h.get_by_label(tr("updates.later")).click();
            h.run_steps(4);
            assert!(!h.state().updates.is_open());
        }
        set_lang(Lang::En);
    }
}
