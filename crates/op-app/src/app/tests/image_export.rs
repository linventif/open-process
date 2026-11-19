use super::*;

fn bpmn<'a>(h: &'a mut Harness<'static, OpenProcessApp>, i: usize) -> &'a mut BpmnEditor {
    h.state_mut().docs[i].editor.as_any_mut().downcast_mut::<BpmnEditor>().unwrap()
}

/// A zoomed-in view shows a fraction of the diagram; the PNG must still hold all of it.
#[test]
fn png_export_covers_the_whole_diagram_and_restores_the_view() {
    let mut h = harness_sized(false, [900.0, 600.0]);
    h.run_steps(4);
    let i = h.state().active;
    bpmn(&mut h, i).view = crate::canvas::View {
        offset: egui::vec2(-300.0, -250.0),
        zoom: 3.0,
    };
    h.run_steps(2);
    let view = bpmn(&mut h, i).view;
    h.state_mut().docs[i].editor.execute(PNG, 0.0);
    let mut finished = None;
    for _ in 0..200 {
        h.step();
        if let Some(result) = crate::io::poll_finished_png() {
            finished = Some(result);
            break;
        }
    }
    let (png, name) = finished.expect("the export finishes").unwrap();
    assert_eq!(name, "Order_handling.png");
    let image = image::load_from_memory(&png).unwrap().to_rgba8();
    let d = &bpmn(&mut h, i).diagram;
    let rects: Vec<_> = d.nodes.iter().map(|n| n.rect()).chain(d.pools.iter().map(|p| p.rect())).collect();
    let width = rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max) - rects.iter().map(|r| r.x).fold(f32::MAX, f32::min);
    assert!(image.width() as f32 >= 2.0 * width, "{} px for {width} units", image.width());
    assert!(image.width() > 900, "larger than the whole window");
    // Drawn content reaches both ends of the image.
    let dark = |x: u32| (0..image.height()).any(|y| image.get_pixel(x, y).0[..3].iter().all(|&c| c < 110));
    let quarter = image.width() / 4;
    assert!((0..quarter).any(dark) && (3 * quarter..image.width()).any(dark));
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
    std::fs::create_dir_all(&dir).unwrap();
    image.save(dir.join("bpmn_full_export.png")).unwrap();
    let after = bpmn(&mut h, i).view;
    assert_eq!((after.offset, after.zoom), (view.offset, view.zoom), "the view is restored");
    assert!(!crate::canvas::export::active());
}
