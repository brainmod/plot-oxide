# PlotOxide

A high-performance data visualization and Statistical Process Control (SPC) application built with Rust and egui.

## Features

### Data Import
- **CSV and Parquet** file support via Polars
- Automatic type detection and timestamp parsing
- Multiple Y-series support with interactive selection
- Drag-and-drop file loading

### Visualization Modes
- **Scatter/Line** plots with configurable styles
- **Histograms** with adjustable bin count
- **Box Plots** for distribution analysis
- **Pareto Charts** with cumulative percentage line
- **X-bar R Charts** for subgroup-based SPC
- **P-Charts** for proportion/attribute data

### Statistical Process Control
- Configurable σ limits (1-6σ)
- Sigma zone visualization (±1σ, ±2σ, ±3σ)
- Western Electric rules detection
- Process capability analysis (Cp, Cpk)
- Specification limits (LSL/USL)
- Outlier detection and highlighting

### Analysis Tools
- Moving Average (MA) overlay
- Exponentially Weighted Moving Average (EWMA)
- Polynomial regression (linear through 4th order)
- Real-time statistics (mean, median, std dev, min, max)
- Data filtering (X/Y range, outliers, empty values)

### User Interface
- Dark/Light themes
- Interactive tooltips and point selection
- Collapsible control panels
- Keyboard shortcuts
- LTTB downsampling for large datasets

## Installation

```bash
git clone https://github.com/brainmod/plot-oxide.git
cd plot-oxide
cargo build --release
```

## Usage

```bash
cargo run --release
```

1. Open a CSV or Parquet file (📂 button or drag-and-drop)
2. Select X-axis column or use row index
3. Select Y-series from the left panel (Ctrl+click for multi-select)
4. Choose visualization mode and enable SPC features as needed

### Keyboard Shortcuts
| Key | Action |
|-----|--------|
| G | Toggle grid |
| L | Toggle legend |
| T | Toggle theme |
| R | Reset view |
| H/F1 | Help |

## Architecture

PlotOxide uses a modular architecture:

```
src/
├── main.rs          # Entry point + UI rendering
├── constants.rs     # Configuration constants
├── error.rs         # PlotError type
├── data/
│   ├── source.rs    # DataSource (Polars wrapper)
│   └── stats.rs     # Statistics calculations
├── state/
│   ├── view.rs      # ViewState, PlotMode, LineStyle
│   ├── spc.rs       # SpcConfig, WEViolation
│   ├── filters.rs   # FilterConfig
│   └── ui.rs        # UiState
└── widgets/
    ├── spc_controls.rs
    └── filter_controls.rs
```

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust 2024 |
| GUI | egui/eframe |
| Plotting | egui_plot |
| Data | Polars v0.46 |
| Serialization | serde |
| Errors | thiserror |

## Refactoring Status

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ | Polars/Parquet migration (csv crate removed) |
| 2 | ✅ | Idiomatic Rust (state modules, error handling) |
| 3 | ✅ | StripBuilder layout |
| 4 | ✅ | Modular widgets |
| 5 | ⏳ | UI module extraction |

See [CLAUDE.md](CLAUDE.md) for detailed refactoring notes.

## Performance

- Target: <100ms load for 100k row CSV
- LTTB downsampling at 5000 points
- Lazy DataFrame evaluation via Polars
- Cached outlier statistics

## License

MIT License - Copyright (c) 2025 Corey Swinth
