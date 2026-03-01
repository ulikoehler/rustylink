use crate::model::{Block, Line};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClickAction {
    Primary,
    Secondary,
    DoublePrimary,
    DoubleSecondary,
}

#[derive(Clone, Debug)]
pub enum UpdateResponse {
    None,
    Block {
        action: ClickAction,
        block: Block,
        handled: bool,
    },
    Signal {
        action: ClickAction,
        line_idx: usize,
        line: Line,
        handled: bool,
    },
}
