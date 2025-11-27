# PlotOxide Refactoring Plan

## Summary

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ Complete | Polars/Parquet migration + polish |
| 2 | ✅ Complete | Idiomatic Rust improvements |
| 3 | ✅ Complete | StripBuilder layout |
| 4 | ✅ Complete | Modular widget system |
| 5 | ✅ Complete | UI module extraction |

---

## Phase 1: Polars/Parquet Migration ✅

**Completed 2025-11-26**
**Polish completed 2025-11-27**

- Polars v0.46 with lazy, parquet, csv, temporal features
- `DataSource` wrapper (`src/data/source.rs`) supporting CSV + Parquet
- `DataError` type for error handling
- `Stats` struct with polars-based calculations (`src/data/stats.rs`)
- Compatibility methods: `column_as_f64()`, `as_row_major_f64()`, etc.
- Integration tests (9 passing, including performance test)
- **csv crate removed**

### Phase 1 Polish (Completed 2025-11-27)
- ✅ Removed legacy `Vec<Vec<f64>>` fields from `PlotOxide` struct
- ✅ Added accessor methods: `headers()`, `raw_data()`, `data()`
- ✅ Replaced all direct field access with DataSource delegation
- ✅ Removed `#[allow(dead_code)]` from data/source.rs and stats.rs
- ✅ Added performance test for 100k rows (124ms total, well within target)
  - Load: 32ms, Row-major conversion: 90ms, Stats: 2ms

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
- `PlotOxide` reduced from 50+ fields to single `state: AppState` field (Phase 1 polish removed legacy fields)
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

## Phase 5: UI Module Extraction ✅

**Completed (already done in prior PR)**

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
1. [x] Create `src/app.rs` with `PlotOxide` struct (completed in Phase 1 polish)
2. [x] Create `src/ui/mod.rs` with free functions (completed in Phase 5 start)
3. [x] Move each `render_*` method to corresponding file (completed in Phase 5 start)
4. [x] Update imports in `main.rs` (completed in Phase 5 start)
5. [x] Remove legacy `headers`, `raw_data`, `data` fields (completed in Phase 1 polish)

---

## Current Architecture

```
src/
├── main.rs              # Entry point only (~167 lines)
├── app.rs               # PlotOxide struct (reduced to 1 field)
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
├── ui/
│   ├── mod.rs
│   ├── toolbar.rs       # render_toolbar_and_controls()
│   ├── series_panel.rs  # render_series_panel()
│   ├── plot.rs          # render_plot()
│   ├── stats_panel.rs   # render_stats_panel()
│   ├── data_table.rs    # render_data_table_panel()
│   └── help_dialog.rs   # render_help_dialog()
└── widgets/
    ├── mod.rs
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
```

---

## Technical Debt

| Item | Location | Priority | Status |
|------|----------|----------|--------|
| ~~Legacy Vec fields~~ | ~~PlotOxide struct~~ | ~~Medium~~ | ✅ Resolved |
| ~~#[allow(dead_code)]~~ | ~~data/source.rs, stats.rs~~ | ~~Low~~ | ✅ Resolved |
| ~~2100-line main.rs~~ | ~~src/main.rs~~ | ~~Medium~~ | ✅ Resolved |
| Unused `LayoutMode` | state/view.rs | Low | Pending |
| Unused helper methods | DataSource, state modules | Low | Pending |
| Some dead code warnings | Various | Low | Pending |

---

## Test Coverage

- 9 tests passing (3 stats + 3 integration + 2 error + 1 performance)
- All tests use `tempfile` for CSV creation
- Performance test validates 100k row handling (<125ms)
- No UI tests (would require `egui` test harness)

---

## Performance Notes

- LTTB downsampling at 5000 points (optimal, no changes needed)
- Row-major conversion optimized: O(n*m) instead of O(n*m²)
- Outlier stats cached per-column
- **Validated performance (100k rows):**
  - Load: 32ms
  - Row-major conversion: 90ms
  - Stats calculation: 2ms
  - **Total: 124ms** (well within acceptable range)

---

## Refactoring Complete! 🎉

All 5 phases completed. The codebase is now:
- **Modular**: Clean separation of concerns across modules
- **Type-safe**: Strong types with proper error handling
- **Performant**: Validated with real performance tests
- **Maintainable**: No legacy fields, clear architecture
- **Well-tested**: 9 passing tests including performance validation
