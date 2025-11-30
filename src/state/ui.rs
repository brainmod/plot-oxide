//! UI interaction state

#![allow(dead_code)]

/// Active panel in the Focus Mode layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePanel {
    None,
    Controls, // Toolbar (Files, SPC, View settings)
    Series,   // Y-Axis selection
    Table,    // Data table
    Stats,    // Statistics
}

impl Default for ActivePanel {
    fn default() -> Self {
        ActivePanel::Series // Open series selection by default
    }
}

/// UI state manages table interaction, sorting, and layout
#[derive(Debug, Clone, Default)]
pub struct UiState {
    /// Currently active side panel
    pub active_panel: ActivePanel,

    /// Search/filter string for data table rows
    pub row_filter: String,

    /// Scroll to specific row in data table
    pub scroll_to_row: Option<usize>,

    /// Column to sort by in data table
    pub sort_column: Option<usize>,

    /// Sort direction (true = ascending, false = descending)
    pub sort_ascending: bool,

    /// Error message to display in UI (toast/status bar)
    pub error_message: Option<String>,
}

impl UiState {
    /// Create a new UiState with default values
    pub fn new() -> Self {
        Self {
            active_panel: ActivePanel::default(),
            row_filter: String::new(),
            scroll_to_row: None,
            sort_column: None,
            sort_ascending: true,
            error_message: None,
        }
    }

    /// Toggle a specific panel
    pub fn toggle_panel(&mut self, panel: ActivePanel) {
        if self.active_panel == panel {
            self.active_panel = ActivePanel::None;
        } else {
            self.active_panel = panel;
        }
    }

    /// Clear the row filter
    pub fn clear_filter(&mut self) {
        self.row_filter.clear();
    }

    /// Set the row filter string
    pub fn set_filter(&mut self, filter: impl Into<String>) {
        self.row_filter = filter.into();
    }

    /// Check if a filter is active
    pub fn has_filter(&self) -> bool {
        !self.row_filter.is_empty()
    }

    /// Clear the sort configuration
    pub fn clear_sort(&mut self) {
        self.sort_column = None;
        self.sort_ascending = true;
    }

    /// Set the sort column and direction
    pub fn set_sort(&mut self, column: usize, ascending: bool) {
        self.sort_column = Some(column);
        self.sort_ascending = ascending;
    }

    /// Toggle sort direction for a column
    pub fn toggle_sort(&mut self, column: usize) {
        if self.sort_column == Some(column) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(column);
            self.sort_ascending = true;
        }
    }

    /// Check if currently sorting
    pub fn is_sorting(&self) -> bool {
        self.sort_column.is_some()
    }

    /// Scroll to a specific row
    pub fn scroll_to(&mut self, row: usize) {
        self.scroll_to_row = Some(row);
    }

    /// Clear scroll target
    pub fn clear_scroll_target(&mut self) {
        self.scroll_to_row = None;
    }

    /// Set an error message
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.error_message = Some(message.into());
    }

    /// Clear the current error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Check if there's an error to display
    pub fn has_error(&self) -> bool {
        self.error_message.is_some()
    }
}