use rustylink::editor::block_catalog::{get_block_catalog, get_block_catalog_by_category};

#[test]
fn catalog_has_at_least_750_entries() {
    let catalog = get_block_catalog();
    assert!(
        catalog.len() >= 750,
        "Expected >= 750 blocks, got {}",
        catalog.len()
    );
}

#[test]
fn catalog_entries_have_non_empty_fields() {
    for entry in get_block_catalog() {
        assert!(!entry.block_type.is_empty(), "Empty block_type");
        assert!(
            !entry.display_name.is_empty(),
            "Empty display_name for {}",
            entry.block_type
        );
        assert!(
            !entry.category.is_empty(),
            "Empty category for {}",
            entry.block_type
        );
        assert!(
            !entry.description.is_empty(),
            "Empty description for {}",
            entry.block_type
        );
    }
}

#[test]
fn catalog_unique_block_types() {
    let catalog = get_block_catalog();
    let mut seen = std::collections::HashSet::new();
    for entry in catalog {
        assert!(
            seen.insert(&entry.block_type),
            "Duplicate block_type: {}",
            entry.block_type
        );
    }
}

#[test]
fn catalog_search_finds_gain() {
    let catalog = get_block_catalog();
    let matches: Vec<_> = catalog.iter().filter(|e| e.matches_query("gain")).collect();
    assert!(!matches.is_empty(), "Should find blocks matching 'gain'");
}

#[test]
fn catalog_search_empty_returns_all() {
    let catalog = get_block_catalog();
    let matches: Vec<_> = catalog.iter().filter(|e| e.matches_query("")).collect();
    assert_eq!(matches.len(), catalog.len());
}

#[test]
fn catalog_by_category_is_consistent() {
    let categories = get_block_catalog_by_category();
    let total: usize = categories.iter().map(|c| c.entries.len()).sum();
    assert_eq!(total, get_block_catalog().len());
}

#[test]
fn catalog_categories_are_non_empty() {
    for cat in get_block_catalog_by_category() {
        assert!(!cat.name.is_empty());
        assert!(!cat.entries.is_empty(), "Category '{}' is empty", cat.name);
    }
}
