use super::*;
use op_core::{
    geom::{Port, Rect as Bounds},
    style::EdgeStyle,
};

#[test]
fn reciprocal_arcs_keep_transition_ports_and_follow_moved_nodes() {
    let mut net = PetriNet::new("Attached arcs");
    let place = NodeRef::Place(net.add_place(Pos::new(0.0, 0.0), 0));
    let id = net.add_transition(Pos::new(160.0, 80.0));
    let transition = NodeRef::Transition(id);
    net.add_arc(place, transition).unwrap();
    net.add_arc(transition, place).unwrap();
    let mut editor = PetriEditor::new(net);
    for style in [EdgeStyle::Straight, EdgeStyle::Orthogonal, EdgeStyle::Curved] {
        editor.net.style.edges = style;
        for position in [
            Pos::new(160.0, 80.0),
            Pos::new(0.0, 180.0),
            Pos::new(180.0, 0.0),
            Pos::new(0.0, -180.0),
        ] {
            editor.net.transition_mut(id).unwrap().pos = position;
            let forward = editor.arc_path(place, transition).unwrap();
            let reverse = editor.arc_path(transition, place).unwrap();
            let (w, h) = editor.trans_size_of(id);
            let bounds = Bounds::from_center(position, w, h);
            assert!(
                Port::ALL
                    .iter()
                    .any(|port| port.point(bounds).dist(*forward.last().unwrap()) < 0.001)
            );
            assert!(Port::ALL.iter().any(|port| port.point(bounds).dist(reverse[0]) < 0.001));
            assert!((forward[0].dist(Pos::default()) - PLACE_R).abs() < 0.001);
            assert_ne!(forward, reverse.into_iter().rev().collect::<Vec<_>>());
        }
    }
}
