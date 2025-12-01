//! UI interaction state

#![allow(dead_code)]

use std::collections::HashSet;

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

/// Cached statistics for a column
#[derive(Debug, Clone, Default)]
pub struct CachedStats {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub p5: f64,
    pub p25: f64,
    pub p75: f64,
    pub p95: f64,
    pub histogram: Vec<u32>,  // Bin counts for sparkline
    pub histogram_max: u32,   // Max bin count for scaling
}

/// Table state with pre-computed filter/sort indices
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// Pre-computed row indices after filtering
    pub filtered_indices: Vec<usize>,
    /// Pre-computed row indices after filtering AND sorting
    pub display_indices: Vec<usize>,
    /// Currently selected rows
    pub selected_rows: HashSet<usize>,
    /// Last filter string (to detect changes)
    filter_cache_key: String,
    /// Last sort config (column, ascending) 
    sort_cache_key: (Option<usize>, bool),
    /// Data version counter (increments on data load)
    data_version: u64,
    /// Go-to-row input field
    pub goto_row_input: String,
}

impl TableState {
    /// Check if cache is valid for current filter/sort/data
    pub fn is_cache_valid(&self, filter: &str, sort_col: Option<usize>, sort_asc: bool, data_version: u64) -> bool {
        self.filter_cache_key == filter 
            && self.sort_cache_key == (sort_col, sort_asc)
            && self.data_version == data_version
    }
    
    /// Update cache keys after recomputation
    pub fn update_cache_keys(&mut self, filter: &str, sort_col: Option<usize>, sort_asc: bool, data_version: u64) {
        self.filter_cache_key = filter.to_string();
        self.sort_cache_key = (sort_col, sort_asc);
        self.data_version = data_version;
    }
    
    /// Invalidate cache (forces recomputation)
    pub fn invalidate(&mut self) {
        self.data_version = 0;
        self.filtered_indices.clear();
        self.display_indices.clear();
    }
    
    /// Toggle row selection
    pub fn toggle_selection(&mut self, row: usize) {
        if self.selected_rows.contains(&row) {
            self.selected_rows.remove(&row);
        } else {
            self.selected_rows.insert(row);
        }
    }
    
    /// Select range of rows (for shift-click)
    pub fn select_range(&mut self, start: usize, end: usize) {
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        for i in lo..=hi {
            self.selected_rows.insert(i);
        }
    }
    
    /// Clear all selections
    pub fn clear_selection(&mut self) {
        self.selected_rows.clear();
    }
    
    /// Check if row is selected
    pub fn is_selected(&self, row: usize) -> bool {
        self.selected_rows.contains(&row)
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
    
    /// Table state with pre-computed indices
    pub table: TableState,
    
    /// Cached statistics per column (column_idx -> stats)
    pub stats_cache: std::collections::HashMap<usize, CachedStats>,
    
    /// Stats cache version (invalidate when data changes)
    pub stats_cache_version: u64,
    
    /// Data version counter (increments on load)
    pub data_version: u64,
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
            table: TableState::default(),
            stats_cache: std::collections::HashMap::new(),
            stats_cache_version: 0,
            data_version: 0,
        }
    }
    
    /// Increment data version (call after loading new data)
    pub fn on_data_loaded(&mut self) {
        self.data_version += 1;
        self.stats_cache.clear();
        self.stats_cache_version = 0;
        self.table.invalidate();
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
    
    /// Get cached stats for a column, or None if not cached
    pub fn get_cached_stats(&self, col_idx: usize) -> Option<&CachedStats> {
        if self.stats_cache_version == self.data_version {
            self.stats_cache.get(&col_idx)
        } else {
            None
        }
    }
    
    /// Cache stats for a column
    pub fn cache_stats(&mut self, col_idx: usize, stats: CachedStats) {
        if self.stats_cache_version != self.data_version {
            self.stats_cache.clear();
            self.stats_cache_version = self.data_version;
        }
        self.stats_cache.insert(col_idx, stats);
    }
}
