use arena_container::default_images::ALL;

#[test]
fn default_images_all_entries_have_distinct_ids() {
    let mut ids = ALL.iter().map(|entry| entry.id).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), ALL.len());
}

#[test]
fn default_images_all_entries_have_image_and_tag() {
    for entry in ALL {
        assert!(!entry.id.is_empty());
        assert!(!entry.image.is_empty());
        assert!(!entry.tag.is_empty());
    }
}
