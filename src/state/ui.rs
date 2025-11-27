//! UI interaction state

/// UI state manages table interaction and sorting
#[derive(Debug, Clone, Default)]
pub struct UiState {
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
            row_filter: String::new(),
            scroll_to_row: None,
            sort_column: None,
            sort_ascending: true,
            error_message: None,
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
    ///
    /// If the column is already sorted, toggle the direction.
    /// Otherwise, sort by this column in ascending order.
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

    /// Clear scroll target (should be called after scrolling is complete)
    pub fn clear_scroll_target(&mut self) {
        self.scroll_to_row = None;
    }

    /// Set an error message to display in the UI
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
