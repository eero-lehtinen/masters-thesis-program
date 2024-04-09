use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        navigation2: { feature = "navigation2" },
        navigation1: { not(navigation2) },
        spatial_hash: { feature = "spatial_hash" },
        spatial_array: { not(spatial_hash) },
    }
}
