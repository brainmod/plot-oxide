# PlotOxide Refactoring Plan

## Overview

Transition from csv/`Vec<Vec<f64>>` to polars/parquet, improve idiomatic Rust patterns, fix layout sizing, and modularize controls.

## Recent Progress

**Phase 1: Polars/Parquet Migration** - Started 2025-11-26

### Completed (Session 1)
- ✅ Added polars v0.46 to Cargo.toml with features: lazy, parquet, csv, temporal, dtype-datetime
- ✅ Created data module structure (`src/data/mod.rs`, `src/data/source.rs`)
- ✅ Implemented `DataSource` wrapper with:
  - `load()` method supporting both CSV and Parquet files
  - `column_values()` for accessing series data
  - `dataframe()` for direct DataFrame access
  - `apply_filters()` for lazy filtering
  - Schema introspection methods
- ✅ Implemented `DataError` type for proper error handling
- ✅ Parquet file format support added alongside CSV

### Completed (Session 2)
- ✅ Added compatibility methods to `DataSource`:
  - `column_as_f64()` - Extract column as Vec<f64>
  - `column_as_string()` - Extract column as Vec<String>
  - `as_row_major_f64()` - Get all data as Vec<Vec<f64>>
  - `as_row_major_string()` - Get all data as Vec<Vec<String>>
  - `get_f64()`, `get_string()` - Cell access methods
- ✅ Added `data_source: Option<DataSource>` field to PlotOxide struct
- ✅ Migrated `load_csv()` to use `DataSource::load()`
- ✅ Maintained backward compatibility by populating legacy fields
- ✅ Removed csv::ReaderBuilder import (no longer needed)
- ✅ Clean build with no warnings or errors

### Completed (Session 3)
- ✅ Optimized `as_row_major_f64()` and `as_row_major_string()` methods
  - Changed from O(n*m²) to O(n*m) complexity
  - Extract columns once, then transpose (much faster)
- ✅ Created polars-based statistics module (`src/data/stats.rs`):
  - `Stats` struct with mean, std_dev, median, min, max, count
  - `calculate_stats()` - Comprehensive statistics using polars
  - `calculate_statistics()` - Mean and std dev (compatible API)
  - `calculate_median()` - Median calculation
  - `detect_outliers()` - Z-score based outlier detection
  - Legacy Vec<f64> compatibility functions
  - Full test coverage
- ✅ Added `get_column_series()` and `column_stats()` to DataSource
- ✅ Analyzed LTTB downsampling - current implementation is optimal
- ✅ Build verified - all modules compile cleanly

### Completed (Session 4) - 🎉 CSV Crate Removed!
- ✅ Created comprehensive integration tests for DataSource:
  - `test_datasource_csv_loading` - Verifies CSV loading works correctly
  - `test_datasource_row_major_conversion` - Tests data structure conversion
  - `test_datasource_statistics` - Validates statistics calculations
  - All tests use temporary files with proper extensions
- ✅ Added tempfile dev dependency for testing
- ✅ Verified no remaining csv crate usage in codebase
- ✅ **Removed csv crate dependency completely** 🚀
- ✅ All 6 tests pass (3 stats + 3 integration tests)
- ✅ Clean build with zero warnings

### Migration Status: Phase 1 Nearly Complete! ✨
The CSV crate has been fully replaced by Polars. The application now uses:
- ✅ Polars for all data loading (CSV and Parquet)
- ✅ Polars for all statistics calculations
- ✅ Comprehensive test coverage
- ✅ Backward compatibility maintained

### Next Steps
- Test application with real-world CSV and Parquet files
- Gradually migrate existing statistics call sites to use stats module directly
- Monitor performance improvements
- Consider removing legacy Vec<Vec<f64>> fields in future cleanup

---

**Phase 2: Idiomatic Rust Improvements** - Started 2025-11-27

### Completed (Session 1) - 🎉 State Module Refactoring Complete!
- ✅ Created `src/constants.rs` module:
  - SPC defaults (sigma, outlier threshold, MA window, EWMA lambda, etc.)
  - Filter defaults (outlier sigma)
  - Performance constants (downsample threshold, max recent files)
  - Plot defaults (histogram bins, point select tolerance)
  - Layout constants (panel widths/heights, breakpoints)
  - DateTime and numeric precision constants

- ✅ Created `src/error.rs` with thiserror:
  - `PlotError` enum with structured error types
  - User-friendly error messages via `user_message()` and `title()` methods
  - Proper error conversion from io::Error, PolarsError, JSON errors
  - Infrastructure for UI error display (replacing eprintln!)

- ✅ Created modular state structure (`src/state/`):
  - `mod.rs`: AppState wrapper with helper methods
  - `view.rs`: ViewState (18 fields) - display, plot mode, interactivity
  - `spc.rs`: SpcConfig (17 fields) - SPC features & configuration
  - `filters.rs`: FilterConfig (7 fields) - data filtering options
  - `ui.rs`: UiState (4 fields) - table interaction & sorting

- ✅ Refactored PlotOxide struct:
  - Replaced 50+ individual fields with single `state: AppState` field
  - Maintained legacy fields (headers, raw_data, data) for compatibility
  - Simplified Default implementation from 65 lines to 8 lines

- ✅ Updated all field accesses (309 instances!):
  - Automated sed script for bulk replacement
  - `self.field` → `self.state.view.field` (and spc, filters, ui)
  - Consolidated type definitions (LineStyle, PlotMode, WEViolation)

- ✅ Build & test verification:
  - Clean compilation (only unused code warnings)
  - All 8 tests passing
  - Zero regression - full backward compatibility

### Migration Status: Phase 2 COMPLETE! ✨
The codebase has been transformed from a monolithic mega-struct to a modular, maintainable architecture:
- ✅ 50+ fields organized into logical state modules
- ✅ Magic numbers extracted to constants
- ✅ Proper error handling infrastructure
- ✅ Type safety improved
- ✅ Code organization follows Rust idioms

### Metrics
- **Field reduction**: 50+ individual fields → 1 AppState field (98% reduction)
- **Lines refactored**: 900+ across 10 files
- **Compilation errors fixed**: 309 → 0
- **Test success rate**: 100% (8/8 tests passing)
- **Default impl**: 65 lines → 8 lines (88% reduction)

### Completed (Session 2) - Polish Work
- ✅ Replaced `Result<(), String>` with `Result<(), PlotError>`
  - Added `From<DataError>` conversion to PlotError
  - Updated load_csv() signature
- ✅ Replaced all `eprintln!` calls (9 instances) with UI error handling
  - Added error_message field to UiState
  - Errors captured for UI display (toast/status bar)
- ✅ Option combinator improvements
  - Simplified nested if-let chains
  - Applied `.map()`, `.and_then()` patterns
  - More functional, idiomatic Rust

### Future Improvements (Optional Polish)
- [ ] Add builder patterns for complex config objects
- [ ] Additional iterator refactoring opportunities
- [ ] Further Option combinator simplifications

---

## Phase 1: Polars/Parquet Migration

### Current Pain Points
- `Vec<Vec<f64>>` + `Vec<Vec<String>>` duplication
- Manual parsing of timestamps/values
- No lazy evaluation for large datasets
- CSV-only support

### Target Architecture

```rust
// Cargo.toml additions
polars = { version = "0.46", features = ["lazy", "parquet", "csv", "temporal", "dtype-datetime"] }
```

```rust
struct DataSource {
    df: LazyFrame,           // Lazy for filtering/transforms
    materialized: DataFrame, // Cached for display
    schema: Arc<Schema>,
    file_path: Option<PathBuf>,
}

impl DataSource {
    fn load(path: &Path) -> Result<Self, DataError> {
        let df = match path.extension().and_then(|s| s.to_str()) {
            Some("parquet") => LazyFrame::scan_parquet(path, Default::default())?,
            Some("csv") => LazyCsvReader::new(path).finish()?,
            _ => return Err(DataError::UnsupportedFormat),
        };
        // ...
    }

    fn column_values(&self, col: &str) -> PolarsResult<Series> {
        self.materialized.column(col).cloned()
    }

    fn apply_filters(&mut self, filters: &FilterConfig) -> PolarsResult<()> {
        let mut expr = lit(true);
        if let Some(min) = filters.y_min {
            expr = expr.and(col(&filters.y_col).gt_eq(lit(min)));
        }
        // ...
        self.materialized = self.df.clone().filter(expr).collect()?;
        Ok(())
    }
}
```

### Migration Steps

1. **Add polars dependency**, keep csv crate temporarily
2. **Create `DataSource` wrapper** that abstracts storage
3. **Replace stat calculations** with polars expressions:
   ```rust
   // Before
   fn calculate_statistics(values: &[f64]) -> (f64, f64) { ... }
   
   // After
   fn statistics(series: &Series) -> PolarsResult<Stats> {
       Ok(Stats {
           mean: series.mean().unwrap_or(0.0),
           std: series.std(1).unwrap_or(0.0),
           min: series.min::<f64>()?.unwrap_or(0.0),
           max: series.max::<f64>()?.unwrap_or(0.0),
           median: series.median().unwrap_or(0.0),
       })
   }
   ```
4. **Replace timestamp parsing** with polars datetime:
   ```rust
   df.with_column(
       col("timestamp").str().to_datetime(None, None, StrptimeOptions::default(), lit("raise"))
   )
   ```
5. **Remove raw_data/data duplication** - single DataFrame source
6. **Remove csv crate** after validation

### Downsampling with Polars

```rust
fn downsample_lttb_polars(df: &DataFrame, x_col: &str, y_col: &str, threshold: usize) -> DataFrame {
    // Use polars sample() for initial reduction, then LTTB on smaller set
    if df.height() <= threshold {
        return df.clone();
    }
    // Implementation using polars operations
}
```

---

## Phase 2: Idiomatic Rust Improvements

### 2.1 Break Up Mega-Struct

Current `PlotOxide` has 50+ fields. Split into:

```rust
// src/state/mod.rs
mod view;
mod spc;
mod filters;

pub struct AppState {
    data: Option<DataSource>,
    view: ViewState,
    spc: SpcConfig,
    filters: FilterConfig,
    ui: UiState,
}

// src/state/view.rs
#[derive(Default)]
pub struct ViewState {
    pub x_column: Option<String>,
    pub y_columns: Vec<String>,
    pub use_row_index: bool,
    pub plot_mode: PlotMode,
    pub line_style: LineStyle,
    pub show_grid: bool,
    pub show_legend: bool,
    pub dark_mode: bool,
}

// src/state/spc.rs
#[derive(Default)]
pub struct SpcConfig {
    pub show_limits: bool,
    pub sigma: f64,
    pub show_zones: bool,
    pub show_we_rules: bool,
    pub capability: Option<CapabilitySpec>,
}

pub struct CapabilitySpec {
    pub lsl: f64,
    pub usl: f64,
}

// src/state/filters.rs
#[derive(Default)]
pub struct FilterConfig {
    pub y_col: String,
    pub y_range: Option<(f64, f64)>,
    pub x_range: Option<(f64, f64)>,
    pub exclude_empty: bool,
    pub outlier_sigma: Option<f64>,
}
```

### 2.2 Error Handling

Replace `eprintln!` with proper error types:

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlotError {
    #[error("Failed to load file: {0}")]
    FileLoad(#[from] std::io::Error),
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
}

// Show errors in UI toast/status bar instead of eprintln
```

### 2.3 Builder Pattern for Complex Objects

```rust
impl SpcConfig {
    pub fn builder() -> SpcConfigBuilder {
        SpcConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct SpcConfigBuilder {
    show_limits: bool,
    sigma: f64,
    // ...
}

impl SpcConfigBuilder {
    pub fn with_sigma(mut self, sigma: f64) -> Self {
        self.sigma = sigma;
        self
    }
    pub fn build(self) -> SpcConfig { /* ... */ }
}
```

### 2.4 Replace Manual Loops with Iterators

```rust
// Before
let mut result = Vec::new();
for i in 0..values.len() {
    if i + 1 >= window {
        let sum: f64 = values[i + 1 - window..=i].iter().sum();
        result.push([i as f64, sum / window as f64]);
    }
}

// After
let result: Vec<_> = values
    .windows(window)
    .enumerate()
    .map(|(i, w)| [(i + window - 1) as f64, w.iter().sum::<f64>() / window as f64])
    .collect();
```

### 2.5 Use `Option` Combinators

```rust
// Before
if let Some(path) = self.current_file {
    if let Some(name) = path.file_name() {
        ui.label(format!("File: {}", name.to_string_lossy()));
    }
}

// After
self.current_file
    .as_ref()
    .and_then(|p| p.file_name())
    .map(|n| ui.label(format!("File: {}", n.to_string_lossy())));
```

### 2.6 Const for Magic Numbers

```rust
// Before: scattered literals
if points.len() > 5000 { ... }
if min_dist < 0.0004 { ... }

// After
mod constants {
    pub const DOWNSAMPLE_THRESHOLD: usize = 5000;
    pub const POINT_SELECT_TOLERANCE: f64 = 0.0004;
    pub const DEFAULT_SIGMA: f64 = 3.0;
    pub const MAX_RECENT_FILES: usize = 5;
}
```

---

## Phase 3: Layout Improvements (Strip Layout)

### Current Issues
- Side panels fight for space
- Fixed widths don't adapt
- Stats panel height fixed

### Strip Layout Solution

```rust
use egui_extras::{StripBuilder, Size};

fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    CentralPanel::default().show(ctx, |ui| {
        StripBuilder::new(ui)
            .size(Size::exact(200.0))      // Left panel (series)
            .size(Size::remainder())        // Center (plot)
            .size(Size::initial(300.0).at_least(200.0)) // Right panel (data table)
            .horizontal(|mut strip| {
                strip.cell(|ui| self.render_series_panel(ui));
                strip.strip(|builder| {
                    builder
                        .size(Size::exact(32.0))    // Toolbar
                        .size(Size::remainder())     // Plot
                        .size(Size::initial(100.0)) // Stats (collapsible)
                        .vertical(|mut strip| {
                            strip.cell(|ui| self.render_toolbar(ui));
                            strip.cell(|ui| self.render_plot(ui));
                            if self.ui.show_stats {
                                strip.cell(|ui| self.render_stats(ui));
                            }
                        });
                });
                if self.ui.show_data_table {
                    strip.cell(|ui| self.render_data_table(ui));
                }
            });
    });
}
```

### Responsive Breakpoints

```rust
fn layout_mode(ctx: &egui::Context) -> LayoutMode {
    let width = ctx.screen_rect().width();
    match width {
        w if w < 800.0 => LayoutMode::Compact,   // Stack panels
        w if w < 1200.0 => LayoutMode::Normal,   // Hide data table
        _ => LayoutMode::Wide,                    // Full layout
    }
}
```

---

## Phase 4: Modular Controls

### 4.1 Control Groups as Reusable Widgets

```rust
// src/widgets/mod.rs
mod spc_controls;
mod filter_controls;
mod plot_mode_selector;

// src/widgets/spc_controls.rs
pub struct SpcControls<'a> {
    config: &'a mut SpcConfig,
}

impl<'a> SpcControls<'a> {
    pub fn new(config: &'a mut SpcConfig) -> Self {
        Self { config }
    }

    pub fn show(&mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.toggle_value(&mut self.config.show_limits, "σ Limits");
            if self.config.show_limits {
                ui.add(Slider::new(&mut self.config.sigma, 1.0..=6.0).step_by(0.5));
            }
            ui.toggle_value(&mut self.config.show_zones, "Zones");
            ui.toggle_value(&mut self.config.show_we_rules, "WE");
        })
        .response
    }
}

// Usage in main update:
SpcControls::new(&mut self.state.spc).show(ui);
```

### 4.2 Collapsible Sections

```rust
pub fn collapsible_section(ui: &mut Ui, title: &str, id: impl Hash, content: impl FnOnce(&mut Ui)) {
    CollapsingHeader::new(title)
        .id_salt(id)
        .default_open(false)
        .show(ui, content);
}

// Usage
collapsible_section(ui, "📊 SPC Controls", "spc", |ui| {
    SpcControls::new(&mut self.state.spc).show(ui);
});
collapsible_section(ui, "🔍 Filters", "filters", |ui| {
    FilterControls::new(&mut self.state.filters).show(ui);
});
```

### 4.3 Toolbar with Icon Buttons

```rust
fn render_toolbar(&mut self, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui.button("📂").on_hover_text("Open file").clicked() {
            self.open_file_dialog();
        }
        ui.separator();
        
        // Plot mode as segmented button
        ui.selectable_value(&mut self.state.view.plot_mode, PlotMode::Scatter, "📈");
        ui.selectable_value(&mut self.state.view.plot_mode, PlotMode::Histogram, "📊");
        ui.selectable_value(&mut self.state.view.plot_mode, PlotMode::BoxPlot, "📦");
        
        ui.separator();
        ui.toggle_value(&mut self.state.view.show_grid, "⊞").on_hover_text("Grid (G)");
        ui.toggle_value(&mut self.state.ui.show_stats, "∑").on_hover_text("Statistics");
        
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(if self.state.view.dark_mode { "🌙" } else { "☀" }).clicked() {
                self.state.view.dark_mode = !self.state.view.dark_mode;
            }
        });
    });
}
```

### 4.4 Compact Control Rows

```rust
// Instead of sprawling horizontal layouts, use grid
fn render_filter_controls(&mut self, ui: &mut Ui) {
    Grid::new("filter_grid")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Y:");
            range_input(ui, &mut self.state.filters.y_range);
            ui.end_row();
            
            ui.label("X:");
            range_input(ui, &mut self.state.filters.x_range);
            ui.end_row();
            
            ui.checkbox(&mut self.state.filters.exclude_empty, "Empty");
            ui.checkbox(&mut self.state.filters.outlier_sigma.is_some(), "Outliers");
            ui.end_row();
        });
}

fn range_input(ui: &mut Ui, range: &mut Option<(f64, f64)>) {
    let mut enabled = range.is_some();
    let (mut min, mut max) = range.unwrap_or((0.0, 100.0));
    
    ui.checkbox(&mut enabled, "");
    ui.add_enabled(enabled, DragValue::new(&mut min).speed(0.1));
    ui.label("–");
    ui.add_enabled(enabled, DragValue::new(&mut max).speed(0.1));
    
    *range = if enabled { Some((min, max)) } else { None };
}
```

---

## Phase 5: Project Structure

```
src/
├── main.rs              # Entry point, eframe setup
├── app.rs               # PlotOxide App impl
├── error.rs             # PlotError enum
├── constants.rs         # Magic numbers
├── data/
│   ├── mod.rs
│   ├── source.rs        # DataSource (polars wrapper)
│   ├── stats.rs         # Statistics calculations
│   └── spc.rs           # SPC calculations (WE rules, Cp/Cpk, etc.)
├── state/
│   ├── mod.rs           # AppState
│   ├── view.rs          # ViewState
│   ├── spc.rs           # SpcConfig
│   └── filters.rs       # FilterConfig
├── ui/
│   ├── mod.rs
│   ├── toolbar.rs       # Top toolbar
│   ├── series_panel.rs  # Left panel
│   ├── plot.rs          # Main plot area
│   ├── stats_panel.rs   # Bottom stats
│   └── data_table.rs    # Right panel
└── widgets/
    ├── mod.rs
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
```

---

## Migration Checklist

### Phase 1: Polars ✅ COMPLETE (Core Migration)
- [x] Add polars to Cargo.toml (v0.46 with lazy, parquet, csv, temporal features)
- [x] Create DataSource wrapper (src/data/source.rs)
- [x] Implement DataError type for proper error handling
- [x] Add parquet support (via DataSource::load())
- [x] Migrate load_csv to use DataSource
- [x] Add DataSource to PlotOxide struct (backward compatible)
- [x] Add compatibility methods for Vec<Vec<f64>> access
- [x] Optimize row-major conversion methods (O(n*m²) → O(n*m))
- [x] Create polars-based statistics module (src/data/stats.rs)
- [x] Add statistics methods to DataSource
- [x] Analyze downsampling (LTTB is optimal as-is)
- [x] Create integration tests for DataSource
- [x] **Remove csv crate dependency** 🎉

#### Phase 1 Polish (Optional Future Work)
- [ ] Test with large real-world CSV and Parquet files
- [ ] Gradually migrate call sites to use stats module directly
- [ ] Remove legacy Vec<Vec<f64>> fields (after extensive testing)
- [ ] Profile and document performance improvements

### Phase 2: Idioms ✅ COMPLETE (Core Refactoring)
- [x] Split PlotOxide into state modules
- [x] Add thiserror for error handling
- [x] Create error infrastructure (PlotError enum)
- [x] Extract constants (src/constants.rs)
- [x] Update all field accesses (309 instances)
- [x] Consolidate type definitions (LineStyle, PlotMode, WEViolation)

#### Phase 2 Polish ✅ COMPLETE
- [x] Replace Result<(), String> with Result<(), PlotError>
- [x] Replace eprintln! with UI error handling (9 instances)
- [x] Option combinator cleanup (nested if-let chains)

#### Phase 2 Polish (Optional Future Work)
- [ ] Add builder patterns for complex config objects
- [ ] Additional iterator refactoring opportunities
- [ ] Further Option combinator simplifications

### Phase 3: Layout
- [ ] Implement StripBuilder layout
- [ ] Add responsive breakpoints
- [ ] Make panels collapsible
- [ ] Test resize behavior

### Phase 4: Controls
- [ ] Extract SpcControls widget
- [ ] Extract FilterControls widget
- [ ] Create compact toolbar
- [ ] Add collapsible sections
- [ ] Test touch/small screen

### Phase 5: Structure
- [ ] Create module structure
- [ ] Move code to modules
- [ ] Update imports
- [ ] Final cleanup

---

## Notes

- Keep tests alongside each module
- Consider `egui_dock` for dockable panels (future)
- Profile with `cargo flamegraph` after polars migration
- Target: <100ms load for 100k row CSV
