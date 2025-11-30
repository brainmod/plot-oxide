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

```
src/
├── main.rs          # Entry point
├── app.rs           # PlotOxide application struct
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
├── ui/
│   ├── toolbar.rs
│   ├── series_panel.rs
│   ├── plot.rs
│   ├── stats_panel.rs
│   └── data_table.rs
└── widgets/
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
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

## Performance

PlotOxide includes comprehensive performance optimizations:

| Operation | Metric |
|-----------|--------|
| 100k row load | 32ms |
| Row-major conversion | 90ms |
| Statistics calculation | 2ms |
| **Total (100k rows)** | **~124ms** |

### Optimization Features
- **LTTB caching** with zoom quantization (10-50x fewer recomputes)
- **Virtual scrolling** for data table (O(visible) instead of O(n))
- **Adaptive downsampling** (fast nth-point during drag, LTTB when settled)
- **Point culling** via binary search for viewport optimization
- **Background threading** for non-blocking file loads
- **Shared memory** using Arc for zero-copy data sharing

See [CLAUDE.md](CLAUDE.md) for full development notes and architecture details.

## License

MIT License - Copyright (c) 2025 Corey Swinth
