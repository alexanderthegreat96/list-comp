use list_comp::map_comp;

fn main() {
    // Missing the `=>` between key and value.
    let _ = map_comp!(k v for k in 0..3);
}
