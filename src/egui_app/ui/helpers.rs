use super::types::{ClickAction, UpdateResponse};

/// Normalize a user-facing string by collapsing all whitespace to a single
/// space and trimming leading/trailing whitespace.  This is used by various
/// UI components to avoid rendering stray newlines/tabs that may come from the
/// parsed Simulink model.
#[allow(dead_code)]
pub fn clean_display_string(s: &str) -> String {
    crate::parser::helpers::clean_whitespace(s)
}

/// Helper used when opening a block-info dialog to produce the window title.
///
/// Applies [`clean_display_string`] to both the block name and type and then
/// formats them as "name (type)".  Having a dedicated function makes it easy to
/// test.
#[allow(dead_code)]
pub fn block_dialog_title(block: &crate::model::Block) -> String {
    format!(
        "{} ({})",
        clean_display_string(&block.name),
        clean_display_string(&block.block_type)
    )
}

pub(crate) fn is_block_subsystem(b: &crate::model::Block) -> bool {
    (b.block_type == "SubSystem" || b.block_type == "Reference")
        && b.subsystem
            .as_ref()
            .map_or(false, |sub| sub.chart.is_none())
}

pub(crate) fn record_interaction(current: &mut UpdateResponse, new: UpdateResponse) {
    if matches!(new, UpdateResponse::None) {
        return;
    }
    fn is_double(resp: &UpdateResponse) -> bool {
        match resp {
            UpdateResponse::Block { action, .. } | UpdateResponse::Signal { action, .. } => {
                matches!(
                    action,
                    ClickAction::DoublePrimary | ClickAction::DoubleSecondary
                )
            }
            UpdateResponse::None => false,
        }
    }

    let current_is_double = is_double(current);
    let new_is_double = is_double(&new);

    if matches!(current, UpdateResponse::None) {
        *current = new;
    } else if current_is_double && !new_is_double {
        // Preserve the earlier double-click interaction.
    } else if new_is_double && !current_is_double {
        *current = new;
    } else {
        // Default: prefer the most recent interaction.
        *current = new;
    }
}
