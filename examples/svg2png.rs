use pathfinder_svg::SVGScene;
use usvg::{Tree, Options};
use pathfinder_rasterize::rasterize_scene;

fn main() {
    let mut args = std::env::args();
    args.next().unwrap();

    let input = args.next().expect("input");
    let output = args.next().expect("output");

    let input_data = std::fs::read(&input).expect("read input");
    let tree = Tree::from_data(&input_data, &Options::default()).unwrap();
    let scene = SVGScene::from_tree(&tree).scene;

    let image = rasterize_scene(scene);
    image.save(&output).unwrap();
}