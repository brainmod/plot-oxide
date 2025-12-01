# PlotOxide Development Guide

## Refactoring Status

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ Complete | Polars/Parquet migration + polish |
| 2 | ✅ Complete | Idiomatic Rust improvements |
| 3 | ✅ Complete | StripBuilder layout |
| 4 | ✅ Complete | Modular widget system |
| 5 | ✅ Complete | UI module extraction |
| 6 | ✅ Complete | Performance optimizations (Nov 2025) |

**All refactoring phases complete as of 2025-11-29.**

---

## Recent Changes (Nov 30, 2025)

### Critical Performance Fix
- **Data Table Column Prefetch**: Fixed catastrophic bug where `get_string()` converted entire column for EACH cell. Pre-fetching column strings before row loop reduced render_data_table from **1500ms → 42ms** (35x improvement).

### Performance Metrics (100k rows, 1 series, all overlays enabled)
| Scope | Time | Notes |
|-------|------|-------|
| render_data_table | 42ms | Was 1500ms before prefetch fix |
| render_plot | 9.8ms | During rapid panning with LTTB |
| swap_buffers | 10ms | GPU present |
| prefetch_column_strings | ~30ms | One-time per frame, amortized across all rows |

### Nov 29, 2025 Changes

### Performance Fixes (P0)
- **Stats Panel**: Fixed catastrophic `app.data()` call that materialized entire dataset every frame. Now uses column-major access via `get_cached_column()`.
- **Data Table**: Pre-computed filtered/sorted row indices instead of per-row filtering.
- **Stats Caching**: Added `CachedStats` struct with version tracking to avoid recalculation.

### New Features
- **Profiling**: Integrated `profiling` crate for instrumentation. Enable with `--features profile-with-puffin` or `--features profile-with-tracy`.
- **Edge Indicators**: Gradient overlays with arrows when data extends beyond visible plot area.
- **Minimap**: Auto-appearing overview in top-right when zoomed >1.5x, shows data outline and viewport position.
- **Enhanced Stats**: Percentiles (P5, P25, P75, P95), sparkline histograms, copy-to-clipboard button.
- **Table Improvements**: 
  - Actual sorting implementation (click column headers)
  - Search highlighting in filtered cells
  - Go-to-row input field
  - Row selection with checkboxes
  - Ctrl+C to copy selected rows as TSV
  - Ctrl+A to select all visible rows

---

## Architecture

```
src/
├── main.rs              # Entry point (~250 lines)
├── app.rs               # PlotOxide struct (1 field: state)
├── constants.rs         # Magic numbers
├── error.rs             # PlotError enum
├── data/
│   ├── mod.rs
│   ├── source.rs        # DataSource (polars wrapper)
│   └── stats.rs         # Statistics calculations
├── state/
│   ├── mod.rs           # AppState container
│   ├── view.rs          # ViewState, PlotMode, LineStyle
│   ├── spc.rs           # SpcConfig, WEViolation
│   ├── filters.rs       # FilterConfig
│   └── ui.rs            # UiState, TableState, CachedStats
├── perf/
│   ├── mod.rs           # Profiling re-exports, culling, PlotBuffer
│   ├── cache.rs         # LttbCache with zoom quantization
│   ├── downsample.rs    # AdaptiveDownsampler, LTTB algorithm
│   └── worker.rs        # BackgroundWorker for async operations
├── ui/
│   ├── mod.rs
│   ├── toolbar.rs
│   ├── series_panel.rs
│   ├── plot.rs          # + edge indicators + minimap
│   ├── stats_panel.rs   # + percentiles + sparklines + caching
│   ├── data_table.rs    # + filtering + sorting + selection
│   └── help_dialog.rs
└── widgets/
    ├── mod.rs
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
```

---

## Profiling

### Using Puffin (recommended for quick inspection)
```bash
# Build with puffin backend
cargo build --release --features profile-with-puffin

# In another terminal, run puffin_viewer
cargo install puffin_viewer
puffin_viewer
```

### Using Tracy (recommended for deep analysis)
```bash
# Build with tracy backend
cargo build --release --features profile-with-tracy

# Run Tracy profiler (download from https://github.com/wolfpld/tracy)
```

### Adding Instrumentation
```rust
profiling::scope!("my_function");
// or for functions:
#[profiling::function]
fn my_function() { ... }
```

---

## Future Roadmap

### High Priority
| Feature | Rationale | Effort |
|---------|-----------|--------|
| Timezone support | Manufacturing data needs local time display; polars temporal features available | Medium |
| X-axis range in stats panel | Show filtered time/value range context alongside Y stats | Low |
| Panel collapse/auto-hide | Maximize plot area on demand | Low |

### Deferred
| Feature | Rationale |
|---------|-----------|
| Custom date format | Polars auto-detection handles most cases; add if users request |

### Not Planned
| Feature | Rationale |
|---------|-----------|
| Date picker UI | egui date pickers are clunky; current range inputs sufficient |
| Relative time display | Niche; clutters UI |
| puffin_egui in-app viewer | Version incompatibility; use external viewer instead |

---

## Technical Debt

| Item | Location | Priority | Notes |
|------|----------|----------|-------|
| Dead code warnings | Various modules | Low | ~35 warnings for unused constants, structs, and methods |
| Unused `show_profiler` | state/mod.rs | Low | Kept for potential future status indicator |
| Table clipboard | data_table.rs | Medium | `copy_selected_rows` needs egui context for actual clipboard |
| String column cache | source.rs | Low | Could cache `column_as_string()` like numeric columns for further speedup |

### Build Status (Nov 30, 2025)
- **Build**: ✅ Passing (0 errors, ~35 warnings)
- **Tests**: ✅ 9 passing
- **Warnings**: Dead code only (unused public API methods and constants)
- **Profiling**: ✅ puffin/tracy integration working

---

## Test Coverage

- 9 tests passing (3 stats + 3 integration + 2 error + 1 performance)
- All tests use `tempfile` for CSV creation
- Performance test validates 100k row handling (<125ms)

---

## Performance

### Current Benchmarks (Nov 30, 2025)

Tested with 100k row CSV, 1 series selected, all filters/overlays enabled:
| Scope | Time | Status |
|-------|------|--------|
| render_data_table | 42ms | ✅ Fixed (was 1500ms) |
| render_plot | 9.8ms | ✅ Good |
| swap_buffers | 10ms | ✅ GPU-bound |
| prefetch_column_strings | ~30ms | ✅ Expected |
| File load (CSV) | 32ms | ✅ Good |
| Stats calculation | 2ms | ✅ Cached |

**Total frame time during interaction: ~52ms (~19 FPS)**

### Optimizations Implemented (All 6 Phases Complete)

#### Phase 0: Instrumentation ✅
- `profiling` crate integration (puffin/tracy backends)
- Zero-cost when no feature enabled

#### Phase 1: Data Pipeline ✅
- `DataSource::from_dataframe()` for worker integration
- Parquet loading with `low_memory: true` and `ParallelStrategy::Auto`

#### Phase 2: LTTB Caching ✅
- Zoom-level quantization (~40% per bucket)
- 10-50x fewer LTTB recomputes during interactive use

#### Phase 3: Virtual Scrolling ✅
- Table renders O(visible_rows) instead of O(total_rows)
- **Column string prefetch** before row loop (critical: 35x speedup)
- Pre-computed filter/sort indices (no per-frame recomputation)

#### Phase 4: Rendering Optimizations ✅
- Point culling with binary search
- Pre-allocated buffers via `PlotBuffer`
- Adaptive downsampling (nth-point during drag, LTTB when settled)

#### Phase 5: Background Threading ✅
- `BackgroundWorker` with channel-based architecture
- Non-blocking file loading

#### Phase 6: Memory Optimization ✅
- `SharedPoints = Arc<[(f64, f64)]>` for cheap cloning
- Zero-copy data sharing
- Stats caching with version invalidation

LTTB downsampling at 5000 points. Outlier stats cached per-column.

---

## Key Files Changed (Nov 30, 2025)

| File | Changes |
|------|--------|
| `src/ui/data_table.rs` | Column string prefetch, filtering, sorting, selection |
| `src/ui/stats_panel.rs` | CachedStats, percentiles, sparklines |
| `src/ui/plot.rs` | Edge indicators, minimap |
| `src/state/ui.rs` | TableState, CachedStats structs |
| `src/main.rs` | Puffin server initialization |
| `Cargo.toml` | profiling, puffin, puffin_http deps |
