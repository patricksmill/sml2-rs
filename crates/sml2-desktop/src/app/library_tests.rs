use super::*;

fn visible_text(output: egui::FullOutput) -> String {
    fn collect(shape: &egui::Shape, text: &mut String) {
        match shape {
            egui::Shape::Text(shape) => {
                text.push_str(shape.galley.text());
                text.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let mut text = String::new();
    for shape in output.shapes {
        collect(&shape.shape, &mut text);
    }
    text
}

#[test]
fn setup_reveals_optional_dx_only_after_original_verification() {
    let (mut app, ctx, path) = super::tests::app();
    app.game = None;
    app.screen = Screen::Home;
    app.options.launch_library = true;
    app.start_library_import(LibraryPick::Dx);
    assert!(
        app.dx_worker.is_none(),
        "DX cannot precede the original ROM"
    );
    super::tests::frame(&mut app, &ctx, vec![]);
    let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
    assert!(text.contains("1. ORIGINAL GAME"));
    assert!(text.contains("Import ROM"));
    assert!(!text.contains("2. DX MODE"));
    assert!(!text.contains("Play original"));

    let (sender, receiver) = std::sync::mpsc::channel();
    app.library.checking = true;
    app.vanilla_check = Some(receiver);
    sender.send(Ok("Super Mario Land 2".into())).unwrap();
    app.poll_imports();
    assert!(app.library.play_enabled(), "DX is optional");
    super::tests::frame(&mut app, &ctx, vec![]);
    let text = visible_text(super::tests::frame(&mut app, &ctx, vec![]));
    assert!(text.contains("2. DX MODE · OPTIONAL"));
    assert!(text.contains("Add DX ROM / IPS"));
    assert!(text.contains("Play original"));
    assert!(!text.contains("Level packs"));
    assert!(!text.contains("Level editor"));
    let _ = std::fs::remove_dir_all(path);
}
