pub struct PackagesView {
    selected_package: u16,
    package_entry_search_cache: Vec<(usize, String, TagType, UEntryHeader)>,
    package_filter: String,
    package_entry_filter: String,
    texture_cache: TextureCache,
    sorted_package_paths: Vec<(u16, PackagePath)>,
    show_only_hash64: bool,
    sort_by_size: bool,
}
