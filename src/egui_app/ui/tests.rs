#[cfg(test)]
mod tests {
    use super::super::helpers::*;
    use crate::editor::operations::create_default_block;
    use crate::parser::helpers::clean_whitespace;

    #[test]
    fn block_dialog_title_cleans_whitespace() {
        let mut blk = create_default_block("SubSystem", "  Foo\n Bar  ", 0, 0, 0, 0);
        // also exercise a messy block_type
        blk.block_type = "  Baz\nqux   ".to_string();
        let title = block_dialog_title(&blk);
        assert_eq!(title, "Foo Bar (Baz qux)");
    }

    #[test]
    fn property_values_are_cleaned() {
        let mut blk = create_default_block("SubSystem", "X", 0, 0, 0, 0);
        // remove the built-in properties so we can test our own
        blk.properties.clear();
        blk.properties
            .insert("  Key \nName  ".to_string(), "  value\n1  ".to_string());
        let cleaned: Vec<(String, String)> = blk
            .properties
            .iter()
            .map(|(k, v)| (clean_whitespace(k), clean_whitespace(v)))
            .collect();
        assert_eq!(
            cleaned,
            vec![("Key Name".to_string(), "value 1".to_string())]
        );
    }
}
