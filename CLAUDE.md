# PlotOxide Refactoring Plan

## Summary

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ Complete | Polars/Parquet migration |
| 2 | ✅ Complete | Idiomatic Rust improvements |
| 3 | ✅ Complete | StripBuilder layout |
| 4 | ✅ Complete | Modular widget system |
| 5 | ⏳ Pending | UI module extraction |

---

## Phase 1: Polars/Parquet Migration ✅

**Completed 2025-11-26**

- Polars v0.46 with lazy, parquet, csv, temporal features
- `DataSource` wrapper (`src/data/source.rs`) supporting CSV + Parquet
- `DataError` type for error handling
- `Stats` struct with polars-based calculations (`src/data/stats.rs`)
- Compatibility methods: `column_as_f64()`, `as_row_major_f64()`, etc.
- Integration tests (6 passing)
- **csv crate removed**

### Remaining Polish
- [ ] Remove legacy `Vec<Vec<f64>>` fields after validation
- [ ] Profile with large files (target: <100ms for 100k rows)
- [ ] Remove `#[allow(dead_code)]` after full migration

---

## Phase 2: Idiomatic Rust ✅

**Completed 2025-11-27**

- `src/constants.rs` - all magic numbers extracted
- `src/error.rs` - `PlotError` enum with `thiserror`
- `src/state/` module structure:
  - `mod.rs` - `AppState` container
  - `view.rs` - `ViewState` (18 fields), `PlotMode`, `LineStyle`, `LayoutMode`
  - `spc.rs` - `SpcConfig` (17 fields), `WEViolation`
  - `filters.rs` - `FilterConfig` (7 fields)
  - `ui.rs` - `UiState` (5 fields)
- `PlotOxide` reduced from 50+ fields to `state: AppState` + 3 legacy fields
- All `eprintln!` replaced with `UiState::set_error()`
- Option combinators applied throughout

---

## Phase 3: StripBuilder Layout ✅

**Completed 2025-11-27**

- `egui_extras::StripBuilder` for horizontal/vertical panel layout
- 6 extracted render methods:
  - `render_series_panel()` - Y series selection
  - `render_stats_panel()` - statistics summary
  - `render_data_table_panel()` - data table with filter/sort
  - `render_help_dialog()` - keyboard shortcuts
  - `render_toolbar_and_controls()` - toolbar + collapsible sections
  - `render_plot()` - main plot (~760 lines)
- `update()` reduced from ~1400 lines to ~135 lines
- `LayoutMode` enum for responsive breakpoints (unused pending Phase 5)

---

## Phase 4: Modular Controls ✅

**Completed 2025-11-27**

- `src/widgets/` module:
  - `spc_controls.rs` - `SpcControls` widget
  - `filter_controls.rs` - `FilterControls` widget
  - `range_input.rs` - `RangeInput` widget (reusable)
- Compact icon toolbar (14 buttons)
- Collapsible sections via `egui::CollapsingHeader`:
  - "📈 Plot Mode & Style"
  - "📊 SPC Controls"
  - "🔍 Filters"

---

## Phase 5: UI Module Extraction ⏳

**Not started**

Move render methods from `main.rs` to dedicated modules:

```
src/
├── main.rs              # Entry point only (~50 lines)
├── app.rs               # PlotOxide struct + App impl
├── ui/
│   ├── mod.rs
│   ├── toolbar.rs       # render_toolbar_and_controls()
│   ├── series_panel.rs  # render_series_panel()
│   ├── plot.rs          # render_plot()
│   ├── stats_panel.rs   # render_stats_panel()
│   └── data_table.rs    # render_data_table_panel()
```

### Steps
1. [ ] Create `src/app.rs` with `PlotOxide` struct
2. [ ] Create `src/ui/mod.rs` with trait or free functions
3. [ ] Move each `render_*` method to corresponding file
4. [ ] Update imports in `main.rs`
5. [ ] Remove legacy `headers`, `raw_data`, `data` fields

---

## Current Architecture

```
src/
├── main.rs              # Entry + App impl + render methods (~2100 lines)
├── constants.rs         # Magic numbers
├── error.rs             # PlotError enum
├── data/
│   ├── mod.rs
│   ├── source.rs        # DataSource (polars wrapper)
│   └── stats.rs         # Statistics calculations
├── state/
│   ├── mod.rs           # AppState
│   ├── view.rs          # ViewState, PlotMode, LineStyle
│   ├── spc.rs           # SpcConfig, WEViolation
│   ├── filters.rs       # FilterConfig
│   └── ui.rs            # UiState
└── widgets/
    ├── mod.rs
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
```

---

## Technical Debt

| Item | Location | Priority |
|------|----------|----------|
| Legacy Vec fields | `PlotOxide` struct | Medium |
| `#[allow(dead_code)]` | data/source.rs, stats.rs | Low |
| 2100-line main.rs | src/main.rs | Medium |
| Unused `LayoutMode` | state/view.rs | Low |
| Manual stat calculations | main.rs (duplicates stats.rs) | Low |

---

## Test Coverage

- 8 tests passing (3 stats + 3 integration + 2 error)
- All tests use `tempfile` for CSV creation
- No UI tests (would require `egui` test harness)

---

## Performance Notes

- LTTB downsampling at 5000 points (optimal, no changes needed)
- Row-major conversion optimized: O(n*m) instead of O(n*m²)
- Outlier stats cached per-column
- Consider `cargo flamegraph` profiling for large files
