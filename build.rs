use std::fs;

fn render_png(svg: &str, width: u32, height: u32, out: &str) {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .expect("failed to parse SVG");

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .expect("failed to allocate pixmap");

    let scale_x = width as f32 / tree.size().width();
    let scale_y = height as f32 / tree.size().height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.save_png(&out).expect("failed to write PNG");

    // Tell cargo to re-run this build script if the SVG changes
    println!("cargo:rerun-if-changed=assets/pipeweaver-midi.svg");
}

fn main() {
    let base_path = "res/icons";
    let svg = fs::read_to_string(format!("{}/pipeweaver-midi.svg", base_path))
        .expect("failed to read SVG");

    render_png(&svg, 128, 128, format!("{}/pipeweaver-midi-large.png", base_path).as_str());
    render_png(&svg, 48, 48, format!("{}/pipeweaver-midi.png", base_path).as_str());
}