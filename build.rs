use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        navigation2: { feature = "navigation2" },
        navigation1: { not(navigation2) },
        spatial_hash: { feature = "spatial_hash" },
        spatial_kdtree: { feature = "spatial_kdtree" },
        spatial_kdbush: { feature = "spatial_kdbush" },
        spatial_quadtree: { feature = "spatial_quadtree" },
        spatial_array: { not(any(spatial_hash, spatial_kdtree, spatial_kdbush, spatial_quadtree)) },
    }
}
