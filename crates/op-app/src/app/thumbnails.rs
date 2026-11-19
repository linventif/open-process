//! Disposable PNG previews of rendered documents, separate from document/session data.
use crate::editor::{Doc, Revisions};
use crate::platform::autosave;
use egui::{ColorImage, Context, Rect, TextureHandle, TextureId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

const KEY: &str = "open-process-thumbnails-v1";
const SIZE: [u32; 2] = [320, 180];
const MAX_ENTRIES: usize = 64;
const MAX_CACHE_BYTES: usize = 512 * 1024;

pub(super) struct Source {
    pub uid: u64,
    pub rect: Rect,
    pub view: String,
}

#[derive(Clone, PartialEq)]
struct Stamp {
    revisions: Revisions,
    rect: Rect,
    view: String,
    pixels_per_point: f32,
    dark: bool,
}

struct Thumbnail {
    stamp: Option<Stamp>,
    digest: u64,
    png: Vec<u8>,
    texture: TextureHandle,
    sequence: u64,
    archive_revision: Option<String>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum Identity {
    Open(u64),
    Saved(String),
}

impl Identity {
    fn document(doc: &Doc) -> Self {
        doc.archive
            .as_ref()
            .map_or(Self::Open(doc.uid), |entry| Self::Saved(entry.id.clone()))
    }
}

#[derive(Clone)]
struct Request {
    id: u64,
    uid: u64,
    stamp: Stamp,
    digest: u64,
    started: f64,
}

#[derive(Default)]
struct Inbox {
    epoch: u64,
    request_id: u64,
    ready: Option<(Request, Option<Arc<ColorImage>>)>,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    uid: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    archive: Option<Archive>,
    digest: u64,
    png: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Archive {
    id: String,
    revision: String,
}

#[derive(Default)]
pub(super) struct Thumbnails {
    entries: HashMap<Identity, Thumbnail>,
    pending: Option<Request>,
    inbox: Arc<Mutex<Inbox>>,
    sequence: u64,
}

impl Thumbnails {
    fn document(&self, doc: &Doc) -> Option<&Thumbnail> {
        self.entries
            .get(&Identity::document(doc))
            .or_else(|| self.entries.get(&Identity::Open(doc.uid)))
    }

    pub fn texture(&self, doc: &Doc) -> Option<TextureId> {
        self.document(doc).map(|t| t.texture.id())
    }

    pub fn saved_texture(&self, entry: &autosave::Entry) -> Option<TextureId> {
        self.entries
            .get(&Identity::Saved(entry.id.clone()))
            .filter(|t| t.archive_revision.as_deref() == Some(entry.revision.as_str()))
            .map(|t| t.texture.id())
    }

    pub fn acknowledge(&mut self, uid: u64, entry: &autosave::Entry, state: &crate::editor::StoredDocument) {
        let identity = Identity::Saved(entry.id.clone());
        if let Some(thumbnail) = self.entries.remove(&Identity::Open(uid)) {
            self.entries.insert(identity.clone(), thumbnail);
        }
        if let Some(thumbnail) = self.entries.get_mut(&identity)
            && thumbnail.digest == digest(&state.bytes())
        {
            thumbnail.archive_revision = Some(entry.revision.clone());
        }
    }

    pub fn invalidate_scene(&mut self) {
        // The web renderer captures one frame later. Never accept an image of
        // Home, a dialog or another document under the original document's UID.
        self.inbox.lock().unwrap().epoch += 1;
    }

    pub fn poll(&mut self, ctx: &Context, docs: &[Doc], library: &[autosave::Entry]) {
        self.entries.retain(|identity, thumbnail| match identity {
            Identity::Open(uid) => docs.iter().any(|d| d.uid == *uid),
            Identity::Saved(id) => {
                docs.iter().any(|d| d.archive.as_ref().is_some_and(|entry| entry.id == *id))
                    || library
                        .iter()
                        .any(|entry| entry.id == *id && thumbnail.archive_revision.as_deref() == Some(entry.revision.as_str()))
            }
        });
        let ready = self.inbox.lock().unwrap().ready.take();
        let Some((request, image)) = ready else { return };
        if self.pending.as_ref().is_none_or(|p| p.id != request.id) {
            return;
        }
        self.pending = None;
        let Some(image) = image else { return };
        let Some(doc) = docs
            .iter()
            .find(|d| d.uid == request.uid && d.editor.revisions() == request.stamp.revisions)
        else {
            return;
        };
        let Ok((png, image)) = encode(&image, request.stamp.rect, request.stamp.pixels_per_point) else {
            return;
        };
        let texture = ctx.load_texture(format!("document-thumbnail-{}", request.uid), image, egui::TextureOptions::LINEAR);
        self.entries.remove(&Identity::Open(request.uid));
        self.entries.insert(
            Identity::document(doc),
            Thumbnail {
                stamp: Some(request.stamp),
                digest: request.digest,
                png,
                texture,
                sequence: request.id,
                archive_revision: doc
                    .archive
                    .as_ref()
                    .filter(|entry| !entry.revision.is_empty() && doc.autosaved_revision == Some(doc.editor.revisions()))
                    .map(|entry| entry.revision.clone()),
            },
        );
        if self.entries.len() > MAX_ENTRIES {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, t)| t.sequence)
                .map(|(identity, _)| identity.clone());
            if let Some(identity) = oldest {
                self.entries.remove(&identity);
            }
        }
    }

    pub fn capture(&mut self, ctx: &Context, docs: &[Doc], mut sources: Vec<Source>, blocked: bool) {
        sources.sort_by_key(|s| s.view != "editor");
        let mut seen = HashSet::new();
        sources.retain(|s| seen.insert(s.uid));
        let candidates: Vec<_> = sources
            .into_iter()
            .filter_map(|s| {
                let doc = docs.iter().find(|d| d.uid == s.uid)?;
                let rect = s.rect.intersect(ctx.content_rect());
                (rect.is_finite() && rect.width() > 1.0 && rect.height() > 1.0).then(|| {
                    (
                        doc,
                        Stamp {
                            revisions: doc.editor.revisions(),
                            rect,
                            view: s.view,
                            pixels_per_point: ctx.pixels_per_point(),
                            dark: ctx.global_style().visuals.dark_mode,
                        },
                    )
                })
            })
            .collect();
        if let Some(pending) = self.pending.clone() {
            if blocked || !candidates.iter().any(|(d, stamp)| d.uid == pending.uid && *stamp == pending.stamp) {
                self.invalidate_scene();
            }
            if ctx.input(|i| i.time) - pending.started < 5.0 {
                return;
            }
            self.pending = None;
            self.inbox.lock().unwrap().request_id += 1;
        }
        if blocked {
            self.invalidate_scene();
            return;
        }
        let Some((doc, stamp)) = candidates
            .into_iter()
            .find(|(d, s)| self.document(d).is_none_or(|t| t.stamp.as_ref() != Some(s)))
        else {
            return;
        };
        self.sequence += 1;
        let request = Request {
            id: self.sequence,
            uid: doc.uid,
            stamp,
            digest: digest(&doc.snapshot().bytes()),
            started: ctx.input(|i| i.time),
        };
        let inbox = self.inbox.clone();
        let epoch = {
            let mut inbox = inbox.lock().unwrap();
            inbox.request_id = request.id;
            inbox.epoch
        };
        self.pending = Some(request.clone());
        let repaint = ctx.clone();
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |image| {
            let mut inbox = inbox.lock().unwrap();
            if inbox.request_id == request.id {
                let image = (inbox.epoch == epoch).then_some(image);
                inbox.ready = Some((request, image));
                repaint.request_repaint();
            }
        })));
        ctx.request_repaint_after(std::time::Duration::from_secs(5));
    }

    pub fn restore(
        &mut self,
        storage: &dyn eframe::Storage,
        ctx: &Context,
        ids: &HashMap<u64, u64>,
        docs: &[Doc],
        library: &[autosave::Entry],
    ) {
        let Some(raw) = storage.get_string(KEY) else { return };
        if raw.len() > MAX_CACHE_BYTES * 4 + 16_384 {
            return;
        }
        let Ok(saved) = serde_json::from_str::<Vec<Saved>>(&raw) else {
            return;
        };
        let mut total = 0;
        for saved in saved.into_iter().take(MAX_ENTRIES) {
            total += saved.png.len();
            if total > MAX_CACHE_BYTES {
                break;
            }
            let (identity, archive_revision) = if let Some(archive) = saved.archive {
                if !library
                    .iter()
                    .any(|entry| entry.id == archive.id && entry.revision == archive.revision)
                {
                    continue;
                }
                if docs
                    .iter()
                    .find(|d| d.archive.as_ref().is_some_and(|entry| entry.id == archive.id))
                    .is_some_and(|d| digest(&d.snapshot().bytes()) != saved.digest)
                {
                    continue;
                }
                (Identity::Saved(archive.id), Some(archive.revision))
            } else {
                let Some(uid) = ids.get(&saved.uid).copied() else { continue };
                let Some(doc) = docs.iter().find(|d| d.uid == uid) else { continue };
                if digest(&doc.snapshot().bytes()) != saved.digest {
                    continue;
                }
                let revision = doc
                    .archive
                    .as_ref()
                    .filter(|_| doc.autosaved_revision == Some(doc.editor.revisions()))
                    .map(|entry| entry.revision.clone());
                (Identity::document(doc), revision)
            };
            let reader = image::ImageReader::with_format(std::io::Cursor::new(&saved.png), image::ImageFormat::Png);
            if reader.into_dimensions().ok() != Some((SIZE[0], SIZE[1])) {
                continue;
            }
            let Ok(image) = image::load_from_memory_with_format(&saved.png, image::ImageFormat::Png) else {
                continue;
            };
            let image = ColorImage::from_rgba_unmultiplied([SIZE[0] as usize, SIZE[1] as usize], image.to_rgba8().as_raw());
            let texture = ctx.load_texture("document-thumbnail", image, egui::TextureOptions::LINEAR);
            self.entries.insert(
                identity,
                Thumbnail {
                    stamp: None,
                    digest: saved.digest,
                    png: saved.png,
                    texture,
                    sequence: 0,
                    archive_revision,
                },
            );
        }
    }

    fn serialized(&self, docs: &[Doc], library: &[autosave::Entry]) -> String {
        let mut saved = Vec::new();
        let mut total = 0;
        let mut append = |uid, archive, t: &Thumbnail| {
            if total + t.png.len() <= MAX_CACHE_BYTES && saved.len() < MAX_ENTRIES {
                total += t.png.len();
                saved.push(Saved {
                    uid,
                    archive,
                    digest: t.digest,
                    png: t.png.clone(),
                });
            }
        };
        for doc in docs.iter().rev() {
            let Some(t) = self.document(doc) else { continue };
            // A stale image remains useful on Home, but is never restored for
            // a different version of the document on the next launch.
            if digest(&doc.snapshot().bytes()) != t.digest {
                continue;
            }
            let archive = doc
                .archive
                .as_ref()
                .filter(|entry| !entry.revision.is_empty() && doc.autosaved_revision == Some(doc.editor.revisions()))
                .map(|entry| Archive {
                    id: entry.id.clone(),
                    revision: entry.revision.clone(),
                });
            append(doc.uid, archive, t);
        }
        for entry in library {
            if docs.iter().any(|doc| doc.archive.as_ref().is_some_and(|open| open.id == entry.id)) {
                continue;
            }
            let Some(t) = self.entries.get(&Identity::Saved(entry.id.clone())) else {
                continue;
            };
            if t.archive_revision.as_deref() != Some(entry.revision.as_str()) {
                continue;
            }
            append(
                0,
                Some(Archive {
                    id: entry.id.clone(),
                    revision: entry.revision.clone(),
                }),
                t,
            );
        }
        serde_json::to_string(&saved).expect("thumbnail cache is serializable")
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage, docs: &[Doc], library: &[autosave::Entry]) {
        storage.set_string(KEY, self.serialized(docs, library));
    }

    #[cfg(target_arch = "wasm32")]
    pub fn save_browser(&self, storage: &web_sys::Storage, docs: &[Doc], library: &[autosave::Entry]) -> Result<(), String> {
        storage
            .set_item(KEY, &self.serialized(docs, library))
            .map_err(|error| format!("{error:?}"))
    }

    #[cfg(test)]
    pub fn png(&self, doc: &Doc) -> Option<&[u8]> {
        self.document(doc).map(|t| t.png.as_slice())
    }
}

fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn encode(image: &ColorImage, rect: Rect, ppp: f32) -> Result<(Vec<u8>, ColorImage), String> {
    if !rect.is_finite() || !ppp.is_finite() || ppp <= 0.0 {
        return Err("Invalid preview region".into());
    }
    let x = ((rect.min.x * ppp).max(0.0) as usize).min(image.width());
    let y = ((rect.min.y * ppp).max(0.0) as usize).min(image.height());
    let right = ((rect.max.x * ppp).max(0.0) as usize).min(image.width());
    let bottom = ((rect.max.y * ppp).max(0.0) as usize).min(image.height());
    if right <= x || bottom <= y {
        return Err("Empty preview region".into());
    }
    let region = image.region_by_pixels([x, y], [right - x, bottom - y]);
    let raw: Vec<u8> = region.pixels.iter().flat_map(|p| p.to_array()).collect();
    let rgba = image::RgbaImage::from_raw(region.width() as u32, region.height() as u32, raw).ok_or("Invalid preview image")?;
    let fitted = image::imageops::thumbnail(&rgba, SIZE[0], SIZE[1]);
    let mut target = image::RgbaImage::from_pixel(SIZE[0], SIZE[1], *rgba.get_pixel(0, 0));
    image::imageops::overlay(
        &mut target,
        &fitted,
        ((SIZE[0] - fitted.width()) / 2) as i64,
        ((SIZE[1] - fitted.height()) / 2) as i64,
    );
    let color = ColorImage::from_rgba_unmultiplied([SIZE[0] as usize, SIZE[1] as usize], target.as_raw());
    let mut output = std::io::Cursor::new(Vec::new());
    target.write_to(&mut output, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok((output.into_inner(), color))
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage as _;
    use egui::{Color32, pos2, vec2};

    #[derive(Default)]
    struct Storage(HashMap<String, String>);
    impl eframe::Storage for Storage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }

    fn document() -> Doc {
        let registry = crate::composition::registry();
        Doc::new(
            crate::editor::StoredDocument::Editable(registry.example("bpmn.order").unwrap()),
            None,
            true,
            registry,
        )
    }

    #[test]
    fn png_previews_crop_at_capture_dpi_and_clip_safely() {
        let mut image = ColorImage::filled([40, 20], Color32::BLUE);
        for y in 4..12 {
            for x in 8..24 {
                image[(x, y)] = Color32::RED;
            }
        }
        for y in 6..10 {
            for x in 14..18 {
                image[(x, y)] = Color32::GREEN;
            }
        }
        let (png, _) = encode(&image, Rect::from_min_size(pos2(4.0, 2.0), vec2(8.0, 4.0)), 2.0).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&png).unwrap().into_rgba8();
        assert_eq!(decoded.dimensions(), (320, 180));
        assert_eq!(decoded.get_pixel(160, 90).0, Color32::GREEN.to_array());
        assert!(!decoded.pixels().any(|p| p.0 == Color32::BLUE.to_array()));
        assert!(encode(&image, Rect::from_min_max(pos2(-50.0, -50.0), pos2(100.0, 100.0)), 2.0).is_ok());
        assert!(encode(&image, Rect::NOTHING, 1.0).is_err());
        assert!(encode(&image, Rect::from_min_size(pos2(100.0, 100.0), vec2(2.0, 2.0)), 1.0).is_err());
    }

    #[test]
    fn delayed_captures_are_discarded_after_leaving_the_document() {
        let ctx = Context::default();
        let docs = vec![document()];
        let mut cache = Thumbnails::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            cache.capture(
                ui.ctx(),
                &docs,
                vec![Source {
                    uid: docs[0].uid,
                    rect: ui.max_rect(),
                    view: "editor".into(),
                }],
                false,
            );
        });
        output.textures_delta.clear();
        let callback = output
            .viewport_output
            .values()
            .flat_map(|v| &v.commands)
            .find_map(|c| match c {
                egui::ViewportCommand::Screenshot(callback) => Some(callback.clone()),
                _ => None,
            })
            .expect("a rendered document requests a preview");
        cache.invalidate_scene();
        callback.complete(Arc::new(ColorImage::filled([320, 180], Color32::RED)));
        cache.poll(&ctx, &docs, &[]);
        assert!(cache.texture(&docs[0]).is_none(), "Home must not be captured as a diagram");
        assert!(cache.pending.is_none());
    }

    #[test]
    fn preview_cache_restores_remapped_uids_and_ignores_stale_or_corrupt_images() {
        let ctx = Context::default();
        let mut storage = Storage::default();
        let old = document();
        let original = old.snapshot();
        let (png, image) = encode(
            &ColorImage::filled([32, 18], Color32::RED),
            Rect::from_min_size(pos2(0.0, 0.0), vec2(32.0, 18.0)),
            1.0,
        )
        .unwrap();
        let mut cache = Thumbnails::default();
        cache.entries.insert(
            Identity::Open(old.uid),
            Thumbnail {
                stamp: None,
                digest: digest(&original.bytes()),
                png: png.clone(),
                texture: ctx.load_texture("preview-test", image, egui::TextureOptions::LINEAR),
                sequence: 1,
                archive_revision: None,
            },
        );
        let old_uid = old.uid;
        cache.save(&mut storage, &[old], &[]);
        assert!(
            storage.get_string(crate::platform::persistence::KEY).is_none(),
            "preview storage never changes the workspace schema"
        );
        let restored = document();
        let ids = HashMap::from([(old_uid, restored.uid)]);
        let mut loaded = Thumbnails::default();
        loaded.restore(&storage, &ctx, &ids, std::slice::from_ref(&restored), &[]);
        assert_eq!(loaded.png(&restored), Some(png.as_slice()));
        assert_eq!(restored.snapshot(), original);
        let mut changed = restored;
        changed.editor.rename("Changed since capture".into());
        let mut stale = Thumbnails::default();
        stale.restore(&storage, &ctx, &ids, std::slice::from_ref(&changed), &[]);
        assert!(stale.texture(&changed).is_none());
        storage.set_string(
            KEY,
            serde_json::to_string(&vec![Saved {
                uid: old_uid,
                archive: None,
                digest: digest(&changed.snapshot().bytes()),
                png: vec![1, 2, 3],
            }])
            .unwrap(),
        );
        stale.restore(&storage, &ctx, &ids, &[changed], &[]);
        assert!(stale.entries.is_empty());
    }

    #[test]
    fn saved_previews_require_the_same_library_identity_revision_and_content() {
        let ctx = Context::default();
        let mut doc = document();
        let mut entry = autosave::Entry::new(doc.name(), doc.editor.kind().into(), None);
        entry.revision = "saved-revision".into();
        let (png, image) = encode(
            &ColorImage::filled([32, 18], Color32::GREEN),
            Rect::from_min_size(pos2(0.0, 0.0), vec2(32.0, 18.0)),
            1.0,
        )
        .unwrap();
        let mut cache = Thumbnails::default();
        cache.entries.insert(
            Identity::Open(doc.uid),
            Thumbnail {
                stamp: None,
                digest: digest(&doc.snapshot().bytes()),
                png,
                texture: ctx.load_texture("saved-preview-test", image, egui::TextureOptions::LINEAR),
                sequence: 1,
                archive_revision: None,
            },
        );
        cache.acknowledge(doc.uid, &entry, &doc.snapshot());
        assert!(cache.saved_texture(&entry).is_some());
        let mut storage = Storage::default();
        cache.save(&mut storage, &[], std::slice::from_ref(&entry));

        // Closed library entries restore without any open workspace or UID mapping.
        let mut restored = Thumbnails::default();
        restored.restore(&storage, &ctx, &HashMap::new(), &[], std::slice::from_ref(&entry));
        restored.poll(&ctx, &[], std::slice::from_ref(&entry));
        assert!(restored.saved_texture(&entry).is_some());
        doc.archive = Some(entry.clone());
        assert!(restored.texture(&doc).is_some());

        let mut changed = entry.clone();
        changed.revision = "newer-revision".into();
        assert!(restored.saved_texture(&changed).is_none());
        let mut stale = Thumbnails::default();
        stale.restore(&storage, &ctx, &HashMap::new(), &[], &[changed]);
        assert!(stale.entries.is_empty());
        stale.restore(&storage, &ctx, &HashMap::new(), &[], &[]);
        assert!(stale.entries.is_empty(), "deleted records must not restore previews");

        doc.editor.rename("Unsaved content after capture".into());
        stale.restore(&storage, &ctx, &HashMap::new(), &[doc], std::slice::from_ref(&entry));
        assert!(
            stale.entries.is_empty(),
            "a saved preview must not stand in for a different restored draft"
        );
        restored.poll(&ctx, &[], &[]);
        assert!(restored.entries.is_empty());
    }
}
