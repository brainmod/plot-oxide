# PlotOxide

A high-performance data visualization and Statistical Process Control (SPC) application built with Rust and egui.

## Overview

PlotOxide is a desktop application for interactive data analysis and quality control charting. It provides real-time visualization of time-series and statistical data with advanced SPC capabilities including Western Electric rules, process capability analysis, and multiple chart types.

## Current Features

### Data Import & Handling
- CSV file loading with automatic type detection
- Timestamp parsing and display
- Multiple Y-series support
- Interactive data table with sorting and filtering
- Recent files list

### Visualization Modes
- **Scatter Plots** - Time-series and XY plotting
- **Histograms** - Distribution analysis with configurable bins
- **Box Plots** - Statistical distribution visualization
- **Pareto Charts** - Categorical data analysis
- **X-bar R Charts** - Subgroup-based process control
- **P-Charts** - Proportion/attribute data control charts

### Statistical Process Control (SPC)
- Configurable sigma limits (1σ to 6σ)
- Sigma zone visualization
- Western Electric (WE) rules detection
- Process capability analysis (Cp, Cpk)
- Specification limits (LSL/USL)
- Outlier detection and highlighting

### Analysis Tools
- Moving average (MA) overlay
- Exponentially Weighted Moving Average (EWMA)
- Polynomial regression (configurable order)
- Basic statistics (mean, std dev, min, max, median)
- Custom range filtering (X and Y axes)

### User Interface
- Dark/Light mode themes
- Interactive point selection and hover tooltips
- Grid and legend toggles
- Zoom and pan controls
- Keyboard shortcuts
- Line style options (line, points, or both)
- Downsampling for large datasets (LTTB algorithm)

## Installation

### Prerequisites
- Rust 2024 edition or later
- Cargo package manager

### Build from Source

```bash
git clone https://github.com/brainmod/plot-oxide.git
cd plot-oxide
cargo build --release
```

The compiled binary will be available at `target/release/plot-oxide`.

### Run

```bash
cargo run --release
```

## Usage

1. Launch the application
2. Click the file menu or use keyboard shortcuts to open a CSV file
3. Select X-axis column (or use row index)
4. Select one or more Y-axis columns from the series panel
5. Choose visualization mode and analysis options
6. Interact with the plot using mouse controls

### Keyboard Shortcuts
- `G` - Toggle grid
- `L` - Toggle legend
- Various shortcuts for plot modes and features (see Help menu)

## Current Architecture

PlotOxide currently uses:
- **Data Storage**: `Vec<Vec<f64>>` for numeric data, `Vec<Vec<String>>` for raw data
- **CSV Parsing**: `csv` crate with manual type conversion
- **UI Framework**: egui/eframe for cross-platform GUI
- **Plotting**: egui_plot for 2D visualization

## Refactoring Roadmap

PlotOxide is undergoing a major architectural refactoring to improve performance, maintainability, and capabilities. See [CLAUDE.md](CLAUDE.md) for detailed plans.

### Phase 1: Polars/Parquet Migration 🚀 In Progress

Transition from CSV-only support to a Polars-based backend enabling:
- **Lazy evaluation** for large datasets
- **Parquet file support** alongside CSV
- **Native datetime handling** without manual parsing
- **Efficient filtering and transformations** using Polars expressions
- **Reduced memory duplication** (single DataFrame instead of raw_data + data)

**Progress (Updated 2025-11-26, Session 3):**
- [x] Add polars dependency (v0.46 with lazy, parquet, csv, temporal features)
- [x] Create DataSource wrapper abstraction (src/data/source.rs)
- [x] Implement DataError type for robust error handling
- [x] Add parquet format support (via DataSource::load())
- [x] Migrate CSV loading to use DataSource
- [x] Add DataSource field to PlotOxide struct (backward compatible)
- [x] Add compatibility methods (column_as_f64, as_row_major_f64, etc.)
- [x] Optimize row-major conversion (O(n*m²) → O(n*m) complexity)
- [x] Create polars-based statistics module (src/data/stats.rs)
- [x] Add statistics methods to DataSource (get_column_series, column_stats)
- [ ] Test with CSV and Parquet files
- [ ] Gradually migrate to polars statistics
- [ ] Remove legacy Vec<Vec<f64>> fields
- [ ] Remove csv crate dependency

**Latest Commits:**
- Session 1: DataSource wrapper with lazy and materialized DataFrame support
- Session 2: Migrated load_csv() to use DataSource, maintained backward compatibility
- Session 3: Added statistics module, optimized data access methods

### Phase 2: Idiomatic Rust Improvements 📋 Planned

Refactor codebase to follow Rust best practices:
- **Break up mega-struct**: Split 50+ field `PlotOxide` struct into logical modules
  - `AppState`, `ViewState`, `SpcConfig`, `FilterConfig`, `UiState`
- **Error handling**: Replace `eprintln!` with `thiserror`-based error types
- **Builder patterns** for complex configuration objects
- **Iterator usage** instead of manual loops
- **Option combinators** for cleaner conditional logic
- **Constants** instead of magic numbers

### Phase 3: Layout Improvements 📋 Planned

Modernize UI layout system:
- **Strip-based layout** using `egui_extras::StripBuilder`
- **Responsive breakpoints** for different screen sizes
- **Collapsible panels** for better space management
- **Adaptive sizing** instead of fixed widths

### Phase 4: Modular Controls 📋 Planned

Extract UI controls into reusable widgets:
- `SpcControls` widget for SPC configuration
- `FilterControls` widget for data filtering
- Compact toolbar with icon buttons
- Collapsible control sections
- Grid-based control layouts

### Phase 5: Project Structure 📋 Planned

Reorganize codebase into logical modules:

```
src/
├── main.rs              # Entry point
├── app.rs               # Application logic
├── error.rs             # Error types
├── constants.rs         # Configuration constants
├── data/                # Data handling
│   ├── source.rs        # DataSource abstraction
│   ├── stats.rs         # Statistics calculations
│   └── spc.rs           # SPC algorithms
├── state/               # Application state
│   ├── view.rs          # View configuration
│   ├── spc.rs           # SPC configuration
│   └── filters.rs       # Filter state
├── ui/                  # UI components
│   ├── toolbar.rs
│   ├── series_panel.rs
│   ├── plot.rs
│   ├── stats_panel.rs
│   └── data_table.rs
└── widgets/             # Reusable widgets
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
```

## Performance Goals

- **Load time**: <100ms for 100k row CSV files
- **Downsampling**: Automatic LTTB for datasets >5000 points
- **Memory**: Reduced duplication via single DataFrame storage
- **Lazy evaluation**: Defer computation until needed

## Contributing

This project is in active refactoring. Contributions are welcome, especially for:
- Testing the Polars migration
- Performance profiling and optimization
- Additional SPC chart types
- Documentation improvements

## License

MIT License - Copyright (c) 2025 Corey Swinth

See [LICENSE](LICENSE) for full details.

## Technology Stack

- **Language**: Rust (2024 edition)
- **GUI**: egui/eframe
- **Plotting**: egui_plot
- **Data Processing**:
  - Polars v0.46 (lazy, parquet, csv, temporal) - ✅ Added
  - csv crate (legacy, will be removed)
- **Serialization**: serde, serde_json
- **Date/Time**: chrono
- **File Dialogs**: rfd

## Development Status

**Current Version**: 0.1.0 (Pre-Polars Migration)

PlotOxide is functional and feature-complete in its current CSV-based form. The planned refactoring will enhance performance, add parquet support, and improve code maintainability without removing existing functionality.

Latest commit: `8a05a87 - add polars migration plan`

---

For detailed refactoring plans and technical architecture decisions, see [CLAUDE.md](CLAUDE.md).
