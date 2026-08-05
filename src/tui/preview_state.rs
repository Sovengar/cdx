use ratatui::text::Text;

use crate::preview::PreviewEntry;

pub struct PreviewState {
    pub text: Text<'static>,
    pub contents: Text<'static>,
    pub dirty: bool,
    pub scroll: u64,
    pub entries: Vec<PreviewEntry>,
    pub selection: usize,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            text: Text::default(),
            contents: Text::default(),
            dirty: false,
            scroll: 0,
            entries: Vec::new(),
            selection: 0,
        }
    }

    pub fn reset(&mut self) {
        self.text = Text::default();
        self.contents = Text::default();
        self.entries.clear();
        self.selection = 0;
        self.scroll = 0;
    }
}
