//! Connection identity and resolved geometry are shared by canvas, hit testing and SVG.
use super::*;
use crate::{
    document::{Document, DocumentCodec, ReadError},
    geom::Port,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub element: Id,
    pub port: Port,
}

pub struct Geometry<'a> {
    board: &'a Board,
    by_id: HashMap<Id, &'a Element>,
}

impl Board {
    pub fn geometry(&self) -> Geometry<'_> {
        Geometry {
            board: self,
            by_id: self.elements.iter().map(|e| (e.id, e)).collect(),
        }
    }

    /// Grid snapping is applied by the caller only to endpoints that remain free.
    pub fn attach_connector(&self, element: &mut Element, from: Pos, to: Pos, tolerance: f32) {
        if !element.kind.is_connector() {
            return;
        }
        let geometry = self.geometry();
        element.start_attachment = geometry.attachment_at(from, to, tolerance);
        element.end_attachment = geometry.attachment_at(to, from, tolerance);
        let points = geometry.points(element);
        element.set_endpoints(points[0], points[1]);
    }

    pub fn set_endpoint(&mut self, id: Id, start: bool, point: Pos, attachment: Option<Attachment>) {
        let geometry = self.geometry();
        let Some(element) = self.element(id).filter(|e| e.kind.is_connector()) else {
            return;
        };
        let mut points = geometry.points(element);
        let attachment = attachment.filter(|a| geometry.valid_attachment(id, *a));
        let resolved = attachment.and_then(|a| geometry.attachment_point(a)).unwrap_or(point);
        points[usize::from(!start)] = resolved;
        let element = self.elements.iter_mut().find(|e| e.id == id).unwrap();
        element.set_endpoints(points[0], points[1]);
        if start {
            element.start_attachment = attachment;
        } else {
            element.end_attachment = attachment;
        }
    }

    /// Moving shapes keeps attached ends; dragging a connector alone releases its ends.
    pub fn prepare_move(&mut self, selected: &BTreeSet<Id>) {
        let geometry = self.geometry();
        let resolved: HashMap<_, _> = self
            .elements
            .iter()
            .filter(|e| selected.contains(&e.id) && e.kind.is_connector())
            .map(|e| (e.id, geometry.points(e)))
            .collect();
        for e in &mut self.elements {
            let Some(points) = resolved.get(&e.id) else { continue };
            e.set_endpoints(points[0], points[1]);
            let with_shape = [e.start_attachment, e.end_attachment]
                .into_iter()
                .flatten()
                .any(|a| selected.contains(&a.element));
            if !with_shape {
                e.start_attachment = None;
                e.end_attachment = None;
            }
        }
    }
}

impl Geometry<'_> {
    pub fn valid_attachment(&self, connector: Id, attachment: Attachment) -> bool {
        attachment.element != connector && self.by_id.get(&attachment.element).is_some_and(|e| !e.kind.is_path())
    }
    pub fn attachment_point(&self, attachment: Attachment) -> Option<Pos> {
        self.by_id
            .get(&attachment.element)
            .filter(|e| !e.kind.is_path())
            .map(|e| attachment.port.point(e.bounds))
    }
    pub fn attachment_at(&self, point: Pos, toward: Pos, tolerance: f32) -> Option<Attachment> {
        for e in self.board.elements.iter().rev().filter(|e| !e.kind.is_path()) {
            let (port, distance) = Port::ALL
                .into_iter()
                .map(|port| (port, port.point(e.bounds).dist(point)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            let port = if distance <= tolerance {
                port
            } else if e.hit(point, tolerance) {
                Port::toward(e.bounds, toward)
            } else {
                continue;
            };
            return Some(Attachment { element: e.id, port });
        }
        None
    }
    pub fn points(&self, element: &Element) -> Vec<Pos> {
        let mut points = element.world_points();
        if element.kind.is_connector() && points.len() == 2 {
            if let Some(point) = element.start_attachment.and_then(|a| self.attachment_point(a)) {
                points[0] = point;
            }
            if let Some(point) = element.end_attachment.and_then(|a| self.attachment_point(a)) {
                points[1] = point;
            }
        }
        points
    }
    pub fn element_bounds(&self, element: &Element) -> Rect {
        if !element.kind.is_connector() {
            return element.bounds;
        }
        let points = self.points(element);
        if points.len() != 2 {
            return element.bounds;
        }
        let (a, b) = (points[0], points[1]);
        Rect::new(a.x.min(b.x), a.y.min(b.y), (b.x - a.x).abs().max(1.0), (b.y - a.y).abs().max(1.0))
    }
    pub fn bounds(&self) -> Option<Rect> {
        self.board.elements.iter().map(|e| self.element_bounds(e)).reduce(|a, b| {
            let x = a.x.min(b.x);
            let y = a.y.min(b.y);
            Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
        })
    }
    pub fn hit(&self, point: Pos, tolerance: f32) -> Option<Id> {
        self.board
            .elements
            .iter()
            .rev()
            .find(|e| {
                if e.kind.is_connector() {
                    dist_to_polyline(point, &self.points(e)) <= tolerance + e.paint.width / 2.0
                } else {
                    e.hit(point, tolerance)
                }
            })
            .map(|e| e.id)
    }
}

/// Version 2 adds typed endpoint attachments; v1 paths retain their original geometry.
pub struct WhiteboardCodec;
impl DocumentCodec<Board> for WhiteboardCodec {
    fn encode(&self, model: &Board) -> Document {
        Document {
            kind: "whiteboard".into(),
            model_version: 2,
            model: serde_json::to_value(model).expect("board is serializable"),
        }
    }
    fn decode(&self, document: &Document) -> Result<Board, ReadError> {
        if document.kind != "whiteboard" {
            return Err(ReadError::Invalid(format!("Expected whiteboard, got {}", document.kind)));
        }
        if !matches!(document.model_version, 1 | 2) {
            return Err(ReadError::Unsupported(format!(
                "Unsupported whiteboard model version: {}",
                document.model_version
            )));
        }
        let board: Board = serde_json::from_value(document.model.clone()).map_err(|e| ReadError::Invalid(e.to_string()))?;
        board.validate().map_err(ReadError::Invalid)?;
        Ok(board)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attachments_follow_shapes_for_hits_bounds_and_vector_export() {
        let mut board = Board::new("Connected drawing");
        let source = board.add(Element::new(
            Kind::Rectangle,
            Pos::new(0.0, 0.0),
            Pos::new(100.0, 80.0),
            Paint::default(),
        ));
        let target = board.add(Element::new(
            Kind::Diamond,
            Pos::new(240.0, 0.0),
            Pos::new(320.0, 80.0),
            Paint::default(),
        ));
        let mut arrow = Element::new(Kind::Arrow, Pos::new(100.0, 40.0), Pos::new(240.0, 40.0), Paint::default());
        board.attach_connector(&mut arrow, Pos::new(101.0, 41.0), Pos::new(239.0, 40.0), 12.0);
        let arrow = board.add(arrow);
        assert_eq!(
            board.element(arrow).unwrap().start_attachment,
            Some(Attachment {
                element: source,
                port: Port::Right
            })
        );
        assert_eq!(
            board.element(arrow).unwrap().end_attachment,
            Some(Attachment {
                element: target,
                port: Port::Left
            })
        );
        board.elements[0].bounds = Rect::new(70.0, 100.0, 200.0, 80.0);
        board.elements[1].bounds.x = 500.0;
        board.elements[1].bounds.y = 300.0;
        let points = board.geometry().points(board.element(arrow).unwrap());
        assert_eq!(points, [Pos::new(270.0, 140.0), Pos::new(500.0, 340.0)]);
        assert_eq!(board.hit(points[0].lerp(points[1], 0.5), 2.0), Some(arrow));
        assert!(svg::export(&board).unwrap().contains("270,140 500,340"));
        board.set_endpoint(arrow, false, Pos::new(900.0, 700.0), None);
        assert!(board.bounds().unwrap().right() >= 900.0);
        board.validate().unwrap();
    }

    #[test]
    fn deleting_duplicating_and_moving_keep_consistent_attachment_identities() {
        let mut board = example();
        board.prepare_move(&BTreeSet::from([1, 2]));
        assert_eq!(board.element(2).unwrap().end_attachment.unwrap().element, 3);
        board.next_id = Id::MAX - 1;
        let selected = BTreeSet::from([1, 2, 3]);
        let copied = board.duplicate(&selected);
        let arrow = board
            .elements
            .iter()
            .find(|e| copied.contains(&e.id) && e.kind == Kind::Arrow)
            .unwrap();
        assert!(copied.contains(&arrow.start_attachment.unwrap().element));
        assert!(copied.contains(&arrow.end_attachment.unwrap().element));
        let original_points = board.geometry().points(board.element(2).unwrap());
        board.delete(&BTreeSet::from([1]));
        assert_eq!(board.geometry().points(board.element(2).unwrap()), original_points);
        assert!(board.element(2).unwrap().start_attachment.is_none());
        assert_eq!(board.element(2).unwrap().end_attachment.unwrap().element, 3);
        board.prepare_move(&BTreeSet::from([2]));
        assert!(board.element(2).unwrap().end_attachment.is_none());
        assert_eq!(board.element(2).unwrap().world_points(), original_points);
        board.validate().unwrap();
    }

    #[test]
    fn legacy_drawing_migrates_and_invalid_or_future_connections_are_rejected() {
        let old = Document::read(include_bytes!("../../tests/fixtures/whiteboard-v1.opx")).unwrap();
        let mut board = Board::CODEC.decode(&old).unwrap();
        assert_eq!(board.elements[1].world_points(), [Pos::new(120.0, 40.0), Pos::new(220.0, 40.0)]);
        assert!(board.elements[1].start_attachment.is_none());
        let new = Board::CODEC.encode(&board);
        assert_eq!(new.model_version, 2);
        assert_eq!(Board::CODEC.decode(&new).unwrap(), board);
        let mut future = new.clone();
        future.model_version = 3;
        assert!(matches!(Board::CODEC.decode(&future), Err(ReadError::Unsupported(_))));
        board.elements[1].start_attachment = Some(Attachment {
            element: 999,
            port: Port::Right,
        });
        assert!(Board::CODEC.decode(&Board::CODEC.encode(&board)).is_err());
        board.elements[1].start_attachment = Some(Attachment {
            element: 2,
            port: Port::Right,
        });
        assert!(board.validate().is_err());
        board.elements[1].start_attachment = None;
        board.elements[0].end_attachment = Some(Attachment {
            element: 1,
            port: Port::Top,
        });
        assert!(board.validate().is_err());
    }
}
