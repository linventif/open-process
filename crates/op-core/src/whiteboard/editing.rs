//! Portable selections, fresh identities and stable drawing order.
use super::*;

const FORMAT: &str = "open-process-whiteboard-selection";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    format: String,
    version: u32,
    pub elements: Vec<Element>,
}

impl Fragment {
    pub fn bounds(&self) -> Option<Rect> {
        let first = self.elements.first()?.bounds;
        Some(self.elements.iter().skip(1).fold(first, |a, e| {
            let b = e.bounds;
            Rect::new(
                a.x.min(b.x),
                a.y.min(b.y),
                a.right().max(b.right()) - a.x.min(b.x),
                a.bottom().max(b.bottom()) - a.y.min(b.y),
            )
        }))
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT || self.version != 1 {
            return Err(tr("drawing.invalid_clipboard").into());
        }
        Board {
            name: String::new(),
            elements: self.elements.clone(),
            next_id: 1,
        }
        .validate()
    }

    pub fn to_clipboard(&self) -> Result<String, String> {
        self.validate()?;
        let text = serde_json::to_string(self).map_err(|_| tr("drawing.invalid_clipboard").to_string())?;
        if text.len() > crate::document::MAX_DOCUMENT_BYTES {
            return Err(tr("drawing.free_drawing_exceeds_supported_size").into());
        }
        Ok(text)
    }

    /// `None` identifies ordinary clipboard text, which the editor may paste as a text element.
    pub fn from_clipboard(text: &str) -> Result<Option<Self>, String> {
        if text.len() > crate::document::MAX_DOCUMENT_BYTES {
            return Err(tr("drawing.free_drawing_exceeds_supported_size").into());
        }
        let value: serde_json::Value = match serde_json::from_str(text) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        if value["format"].as_str() != Some(FORMAT) {
            return Ok(None);
        }
        let fragment: Self = serde_json::from_value(value).map_err(|_| tr("drawing.invalid_clipboard").to_string())?;
        fragment.validate()?;
        Ok(Some(fragment))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum StackOrder {
    ToFront,
    Forward,
    Backward,
    ToBack,
}

impl Board {
    /// Copy resolved connector geometry and retain only attachments within the selection.
    pub fn fragment(&self, ids: &BTreeSet<Id>) -> Fragment {
        let geometry = self.geometry();
        let elements = self
            .elements
            .iter()
            .filter(|e| ids.contains(&e.id))
            .map(|e| {
                let mut copy = e.clone();
                if e.kind.is_connector() {
                    let points = geometry.points(e);
                    copy.set_endpoints(points[0], points[1]);
                }
                copy.start_attachment = copy.start_attachment.filter(|a| ids.contains(&a.element));
                copy.end_attachment = copy.end_attachment.filter(|a| ids.contains(&a.element));
                copy
            })
            .collect();
        Fragment {
            format: FORMAT.into(),
            version: 1,
            elements,
        }
    }

    /// Validate the entire result before replacing the document, including clipboard data.
    pub fn paste(&mut self, fragment: &Fragment, offset: Pos) -> Result<BTreeSet<Id>, String> {
        fragment.validate()?;
        if !offset.x.is_finite() || !offset.y.is_finite() {
            return Err(tr("drawing.invalid_clipboard").into());
        }
        let mut next = self.clone();
        let mut used = next.elements.iter().map(|e| e.id).collect();
        let remap: std::collections::HashMap<_, _> = fragment.elements.iter().map(|e| (e.id, next.allocate_id(&mut used))).collect();
        for source in &fragment.elements {
            let mut element = source.clone();
            element.id = remap[&source.id];
            element.bounds.x += offset.x;
            element.bounds.y += offset.y;
            for attachment in [&mut element.start_attachment, &mut element.end_attachment] {
                *attachment = attachment.and_then(|a| {
                    remap.get(&a.element).map(|id| Attachment {
                        element: *id,
                        port: a.port,
                    })
                });
            }
            next.elements.push(element);
        }
        next.validate()?;
        let ids = remap.values().copied().collect();
        *self = next;
        Ok(ids)
    }

    pub fn reorder(&mut self, ids: &BTreeSet<Id>, order: StackOrder) {
        match order {
            StackOrder::ToFront | StackOrder::ToBack => {
                let (selected, others): (Vec<_>, Vec<_>) =
                    std::mem::take(&mut self.elements).into_iter().partition(|e| ids.contains(&e.id));
                self.elements = if matches!(order, StackOrder::ToFront) {
                    others.into_iter().chain(selected).collect()
                } else {
                    selected.into_iter().chain(others).collect()
                };
            }
            StackOrder::Forward => {
                for i in (0..self.elements.len().saturating_sub(1)).rev() {
                    if ids.contains(&self.elements[i].id) && !ids.contains(&self.elements[i + 1].id) {
                        self.elements.swap(i, i + 1);
                    }
                }
            }
            StackOrder::Backward => {
                for i in 1..self.elements.len() {
                    if ids.contains(&self.elements[i].id) && !ids.contains(&self.elements[i - 1].id) {
                        self.elements.swap(i, i - 1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Port;

    fn connected_board() -> Board {
        let mut board = Board::new("Clipboard");
        let a = board.add(Element::new(
            Kind::Rectangle,
            Pos::new(0.0, 0.0),
            Pos::new(100.0, 80.0),
            Paint::default(),
        ));
        let b = board.add(Element::new(
            Kind::Ellipse,
            Pos::new(300.0, 100.0),
            Pos::new(400.0, 180.0),
            Paint::default(),
        ));
        let mut arrow = Element::new(Kind::Arrow, Pos::new(100.0, 40.0), Pos::new(300.0, 140.0), Paint::default());
        arrow.start_attachment = Some(Attachment {
            element: a,
            port: Port::Right,
        });
        arrow.end_attachment = Some(Attachment {
            element: b,
            port: Port::Left,
        });
        board.add(arrow);
        board.elements[0].text = "Résumé\n日本語".into();
        board
    }

    #[test]
    fn portable_selection_preserves_styles_geometry_and_internal_connections_with_fresh_ids() {
        let source = connected_board();
        let original = source.clone();
        let selected = source.elements.iter().map(|e| e.id).collect();
        let fragment = source.fragment(&selected);
        let text = fragment.to_clipboard().unwrap();
        let imported = Fragment::from_clipboard(&text).unwrap().unwrap();
        assert_eq!(fragment, imported);
        assert_eq!(source, original, "copy is read-only");
        let mut target = source.clone();
        let offset = Pos::new(-50.0, 250.0);
        let ids = target.paste(&imported, offset).unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.is_disjoint(&selected));
        let copied: Vec<_> = target.elements.iter().filter(|e| ids.contains(&e.id)).collect();
        for (old, new) in source.elements.iter().zip(&copied) {
            assert_eq!(new.text, old.text);
            assert_eq!(new.paint, old.paint);
            assert_eq!(new.bounds.x, old.bounds.x + offset.x);
            assert_eq!(new.bounds.y, old.bounds.y + offset.y);
        }
        let arrow = copied.iter().find(|e| e.kind == Kind::Arrow).unwrap();
        assert!(ids.contains(&arrow.start_attachment.unwrap().element));
        assert!(ids.contains(&arrow.end_attachment.unwrap().element));
        let old_points = source.geometry().points(source.element(3).unwrap());
        let new_points = target.geometry().points(arrow);
        assert_eq!(new_points, old_points.iter().map(|p| *p + offset).collect::<Vec<_>>());
        target.validate().unwrap();
    }

    #[test]
    fn copying_a_connector_alone_detaches_external_targets_without_moving_its_endpoints() {
        let board = connected_board();
        let fragment = board.fragment(&BTreeSet::from([3]));
        let arrow = &fragment.elements[0];
        assert!(arrow.start_attachment.is_none() && arrow.end_attachment.is_none());
        assert_eq!(arrow.world_points(), board.geometry().points(board.element(3).unwrap()));
        let partial = board.fragment(&BTreeSet::from([1, 3]));
        assert!(partial.elements[1].start_attachment.is_some());
        assert!(partial.elements[1].end_attachment.is_none());
        partial.validate().unwrap();
    }

    #[test]
    fn invalid_or_over_budget_pastes_leave_the_destination_untouched() {
        let mut board = connected_board();
        let original = board.clone();
        let fragment = board.fragment(&BTreeSet::from([1]));
        let mut bad = fragment.clone();
        bad.elements[0].bounds.w = -1.0;
        assert!(board.paste(&bad, Pos::default()).is_err());
        assert!(board.paste(&fragment, Pos::new(f32::INFINITY, 0.0)).is_err());
        let mut future: serde_json::Value = serde_json::from_str(&fragment.to_clipboard().unwrap()).unwrap();
        future["version"] = 999.into();
        assert!(Fragment::from_clipboard(&future.to_string()).is_err());
        assert!(Fragment::from_clipboard("plain text").unwrap().is_none());
        assert!(Fragment::from_clipboard("{\"unrelated\":true}").unwrap().is_none());
        assert_eq!(board, original);
        let mut full = Board::new("Full");
        full.elements = (1..=20_000)
            .map(|id| {
                let mut e = fragment.elements[0].clone();
                e.id = id;
                e
            })
            .collect();
        full.next_id = 20_001;
        let original = full.clone();
        assert!(full.paste(&fragment, Pos::default()).is_err());
        assert_eq!(full, original);
    }

    #[test]
    fn stacking_preserves_selection_and_nonselection_relative_order() {
        let base = connected_board();
        for (order, expected) in [
            (StackOrder::Forward, vec![2, 1, 3]),
            (StackOrder::Backward, vec![1, 3, 2]),
            (StackOrder::ToFront, vec![2, 1, 3]),
            (StackOrder::ToBack, vec![1, 3, 2]),
        ] {
            let mut board = base.clone();
            board.reorder(&BTreeSet::from([1, 3]), order);
            assert_eq!(board.elements.iter().map(|e| e.id).collect::<Vec<_>>(), expected);
            for e in &base.elements {
                assert_eq!(board.element(e.id), Some(e));
            }
        }
    }

    #[test]
    fn constrained_drawing_and_movement_cover_all_quadrants() {
        let start = Pos::new(20.0, 30.0);
        for delta in [
            Pos::new(40.0, 10.0),
            Pos::new(-40.0, 10.0),
            Pos::new(0.0, -30.0),
            Pos::new(-10.0, -40.0),
        ] {
            for kind in [Kind::Rectangle, Kind::Ellipse, Kind::Diamond, Kind::Note] {
                let end = kind.constrained_endpoint(start, start + delta) - start;
                assert_eq!(end.x.abs(), end.y.abs());
            }
            let end = Kind::Arrow.constrained_endpoint(start, start + delta) - start;
            let angle = end.y.atan2(end.x) / std::f32::consts::FRAC_PI_4;
            assert!((angle - angle.round()).abs() < 0.001);
            let locked = delta.axis_locked();
            assert!(locked.x == 0.0 || locked.y == 0.0);
        }
    }
}
