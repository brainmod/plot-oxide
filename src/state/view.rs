//! View and visualization state

use crate::constants::performance::DOWNSAMPLE_THRESHOLD;
use crate::constants::plot::DEFAULT_HISTOGRAM_BINS;
use serde::{Deserialize, Serialize};

/// Plot mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlotMode {
    Scatter,
    Histogram,
    BoxPlot,
    Pareto,
    XbarR,
    PChart,
}

impl Default for PlotMode {
    fn default() -> Self {
        PlotMode::Scatter
    }
}

/// Line style enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    Line,
    Points,
    LineAndPoints,
}

impl Default for LineStyle {
    fn default() -> Self {
        LineStyle::Line
    }
}

/// Layout mode for responsive design
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Compact layout for small screens (<800px) - stack panels vertically
    Compact,
    /// Normal layout for medium screens (800-1200px) - hide data table by default
    Normal,
    /// Wide layout for large screens (>1200px) - show all panels
    Wide,
}

/// View state manages all visualization and display options
#[derive(Debug, Clone)]
pub struct ViewState {
    // Column selection & indexing
    /// Current X axis column index
    pub x_index: usize,

    /// Use row number as X axis instead of column data
    pub use_row_index: bool,

    /// Multiple Y series column indices
    pub y_indices: Vec<usize>,

    // Display options
    /// Dark mode theme toggle
    pub dark_mode: bool,

    /// Show help panel
    pub show_help: bool,

    /// Grid visibility
    pub show_grid: bool,

    /// Legend visibility
    pub show_legend: bool,

    /// Data table panel visibility
    pub show_data_table: bool,

    /// Statistics panel visibility
    pub show_stats_panel: bool,

    /// Series panel visibility (left panel)
    pub show_series_panel: bool,

    // Plot interaction
    /// Enable zoom functionality
    pub allow_zoom: bool,

    /// Enable pan/drag
    pub allow_drag: bool,

    /// Reset zoom bounds flag
    pub reset_bounds: bool,

    // Plot mode & styling
    /// Current plot mode (Scatter, Histogram, BoxPlot)
    pub plot_mode: PlotMode,

    /// Line rendering style
    pub line_style: LineStyle,

    /// X axis is timestamp data
    pub x_is_timestamp: bool,

    /// Show histogram overlay
    pub show_histogram: bool,

    /// Number of histogram bins
    pub histogram_bins: usize,

    /// Show box plot overlay
    pub show_boxplot: bool,

    /// Downsampling threshold for large datasets (using LTTB algorithm)
    pub downsample_threshold: usize,

    // Interactivity state
    /// Currently hovered point (series_idx, point_idx)
    pub hovered_point: Option<(usize, usize)>,

    /// Currently selected point (series_idx, point_idx)
    pub selected_point: Option<(usize, usize)>,

    /// Row index hovered in data table
    pub table_hovered_row: Option<usize>,

    /// Last clicked series index
    pub last_selected_series: Option<usize>,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            // Column selection
            x_index: 0,
            use_row_index: false,
            y_indices: Vec::new(),

            // Display options
            dark_mode: true,
            show_help: false,
            show_grid: true,
            show_legend: true,
            show_data_table: false,
            show_stats_panel: false,
            show_series_panel: true,

            // Plot interaction
            allow_zoom: true,
            allow_drag: true,
            reset_bounds: false,

            // Plot mode & styling
            plot_mode: PlotMode::default(),
            line_style: LineStyle::default(),
            x_is_timestamp: false,
            show_histogram: false,
            histogram_bins: DEFAULT_HISTOGRAM_BINS,
            show_boxplot: false,
            downsample_threshold: DOWNSAMPLE_THRESHOLD,

            // Interactivity
            hovered_point: None,
            selected_point: None,
            table_hovered_row: None,
            last_selected_series: None,
        }
    }
}

impl ViewState {
    /// Create a new ViewState with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all selection state
    pub fn clear_selection(&mut self) {
        self.hovered_point = None;
        self.selected_point = None;
        self.table_hovered_row = None;
        self.last_selected_series = None;
    }

    /// Reset plot bounds on next frame
    pub fn reset_plot_bounds(&mut self) {
        self.reset_bounds = true;
    }

    /// Toggle dark mode
    pub fn toggle_dark_mode(&mut self) {
        self.dark_mode = !self.dark_mode;
    }

    /// Check if any Y series are selected
    pub fn has_y_series(&self) -> bool {
        !self.y_indices.is_empty()
    }

    /// Get the number of selected Y series
    pub fn y_series_count(&self) -> usize {
        self.y_indices.len()
    }

    /// Determine layout mode based on screen width
    pub fn get_layout_mode(screen_width: f32) -> LayoutMode {
        use crate::constants::layout::{COMPACT_BREAKPOINT, NORMAL_BREAKPOINT};

        match screen_width {
            w if w < COMPACT_BREAKPOINT => LayoutMode::Compact,
            w if w < NORMAL_BREAKPOINT => LayoutMode::Normal,
            _ => LayoutMode::Wide,
        }
    }
}
