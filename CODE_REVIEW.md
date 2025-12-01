# PlotOxide Code Review
**Date:** 2025-11-30  
**Reviewer:** Claude  
**Scope:** Full codebase review

---

## Executive Summary

PlotOxide is a well-architected Rust/egui data visualization application with strong performance optimizations. The codebase demonstrates good separation of concerns and thoughtful module organization. The recent performance work (35x improvement in data table rendering) shows effective profiling-driven optimization.

**Overall Grade: B+**

### Strengths
- Clean modular architecture (`state/`, `ui/`, `data/`, `perf/`, `widgets/`)
- Comprehensive profiling infrastructure (puffin/tracy)
- Excellent performance optimizations (LTTB caching, virtual scrolling, column prefetch)
- Good use of Polars for data handling
- Solid SPC feature set

### Areas for Improvement
- Code duplication (LTTB algorithm appears 3x)
- Dead code accumulation (~35 warnings)
- Missing features in alternate plot modes
- Some UI/UX gaps

---

## Critical Issues

### 1. **Invalid Rust Edition** (P0 - Build Risk)
**File:** `Cargo.toml:4`
```toml
edition = "2024"  # ❌ Should be "2021"
```
Rust 2024 edition doesn't exist yet. This likely works because Cargo falls back, but it's incorrect and could cause issues with stricter toolchains.

**Fix:** Change to `edition = "2021"`

---

## Code Quality Issues

### 2. **LTTB Algorithm Duplication** (P1 - Maintainability)
The LTTB downsampling algorithm is implemented in three places:
- `src/app.rs:downsample_lttb()` (lines ~420-460)
- `src/perf/downsample.rs:lttb_downsample()` (lines ~45-95)
- `src/perf/worker.rs:compute_lttb()` (lines ~85-140)

**Impact:** Bug fixes need to be applied 3x. Inconsistent behavior risk.

**Recommendation:** Consolidate into `perf/downsample.rs` and re-export. Update `app.rs` to call the shared implementation.

### 3. **Series Panel Performance** (P1 - Performance)
**File:** `src/ui/series_panel.rs:18-35`

Statistics are calculated for ALL columns on every frame, not just selected ones:
```rust
let violation_color = if !data.is_empty() {
    let y_values: Vec<f64> = data.iter().map(|row| row[i]).collect();
    let (mean, std_dev) = PlotOxide::calculate_statistics(&y_values);
    // ... calculates for every column
```

**Impact:** O(columns × rows) work per frame, even for unselected columns.

**Recommendation:** Cache violation status per column with version tracking (similar to `CachedStats`).

### 4. **Dead Code Accumulation** (P2 - Code Health)
~35 `#[allow(dead_code)]` annotations scattered throughout. Some are legitimate (public API), but many indicate unused code that should be removed or actually used.

Notable examples:
- `ViewState::show_data_table`, `show_stats_panel`, `show_series_panel` - seem to be legacy from before Focus Panel refactor
- `PlotBuffer` in `AppState` - allocated but never used in rendering
- `RangeInput` widget - fully implemented but commented out from exports

**Recommendation:** Audit each `allow(dead_code)` and either use the code, remove it, or document why it's kept.

### 5. **Clipboard Inconsistency** (P2 - API Usage)
**File:** `src/ui/data_table.rs` uses `arboard::Clipboard` directly, while `stats_panel.rs` uses `ui.ctx().copy_text()`.

**Recommendation:** Standardize on `ctx.copy_text()` for consistency with egui patterns.

---

## Feature Gaps

### 6. **No Downsample Rate Indicator** (P1 - UX)
Users have no visibility into whether data is being downsampled or at what rate. This is important for understanding data fidelity.

**Location to add:** `src/ui/plot.rs` - add status text or badge near minimap

### 7. **Alternate Plot Mode X-Axis Labels** (P1 - UX)
**File:** `src/ui/plot.rs`

Histogram, Pareto, BoxPlot, XbarR, and PChart modes don't properly label the X-axis:
- **Histogram:** X-axis shows bin values but lacks "Value" label
- **Pareto:** X-axis shows indices (0, 1, 2...) instead of category labels
- **BoxPlot:** X-axis shows series index (0, 1...) instead of series names
- **XbarR/PChart:** X-axis shows subgroup numbers but no label

**Recommendation:** Add `plot.x_axis_label()` calls per mode, consider custom formatters.

### 8. **Minimap Data Representation** (P2 - UX)
**File:** `src/ui/plot.rs:780-800`

The minimap uses aggressive stepping (`step_by(series.len() / 50)`) which can completely miss data features. For 100k points, it samples every 2000th point.

**Current:**
```rust
let step = (series.len() / 50).max(1);
let mini_points: Vec<eframe::egui::Pos2> = series.iter()
    .step_by(step)
```

**Recommendation:** Use LTTB-downsampled data (already computed) for minimap, or compute min/max envelope per horizontal pixel bucket.

### 9. **No Date Range Filters** (P2 - Feature)
X-axis filters use numeric `DragValue` which is poor UX for timestamp data. Table filtering has no date-aware options.

**File:** `src/widgets/filter_controls.rs`

**Recommendation:** Detect when X-axis is timestamp and show date picker or date input format.

### 10. **No Advanced Table Filters** (P2 - Feature)
**File:** `src/ui/data_table.rs`

Current filtering is text-based substring matching only. No support for:
- Numeric comparisons (>, <, >=, <=, between)
- Outlier filtering (show only rows > N sigma)
- Empty/non-empty filtering
- Regex matching

**Recommendation:** Add filter mode selector (Text, Numeric, Outlier) with appropriate UI per mode.

---

## Minor Issues

### 11. **Missing Windows Icon Configuration**
**Files:** `Cargo.toml`, `assets/`

`icon.png` and `icon_mac.png` exist but there's no Windows `.ico` file or `windows_subsystem` configuration beyond the console hide. Modern Windows apps need proper icon embedding via `winres` or `embed-resource`.

### 12. **Potential Panic in Plot Transform**
**File:** `src/ui/plot.rs:680`
```rust
let y_idx = app.state.view.y_indices[series_idx];
```
If `y_indices` is modified during iteration (unlikely but possible with async), this could panic. Consider bounds checking.

### 13. **Magic Numbers**
Despite having `constants.rs`, there are still magic numbers scattered:
- `80.0` minimap size (plot.rs:757)
- `0.0004` point selection tolerance (plot.rs:721, 743)
- `50` minimap point step (plot.rs:787)

**Recommendation:** Move to `constants.rs` under `plot` or `minimap` module.

### 14. **Unused Imports Warning Suppression**
**File:** `src/data/mod.rs`
```rust
#[allow(unused_imports)]
pub use source::{DataSource, DataError};
```
These are actually used - the warning suppression is unnecessary.

---

## Architecture Observations

### Positive Patterns

1. **State Management:** Clean separation of view, SPC, filter, and UI state into distinct structs.

2. **Performance Caching:** Good use of `RefCell<HashMap>` for numeric column cache, `CachedStats` with version tracking.

3. **Profiling Integration:** The `profiling` crate usage is exemplary - zero-cost when disabled, comprehensive when enabled.

4. **Error Handling:** `PlotError` enum with `user_message()` method is a nice pattern for user-facing errors.

### Suggestions for Future

1. **Consider `im` or `rpds` for immutable data structures** - would simplify undo/redo implementation.

2. **Event system for state changes** - currently UI directly mutates state; an event system would enable features like history, state persistence.

3. **Plugin architecture for plot types** - the `PlotMode` match arms are getting long; consider trait-based plot renderers.

---

## Test Coverage

Current: 9 tests (3 stats, 3 integration, 2 error, 1 performance)

**Missing test coverage:**
- UI rendering (would need egui test harness)
- LTTB accuracy verification
- Filter edge cases (NaN handling, empty data)
- Concurrent file loading
- Clipboard operations

---

## Security Considerations

1. **File Loading:** Uses Polars which handles malformed files gracefully. No path traversal risks identified.

2. **Clipboard:** Direct text operations only, no binary/HTML clipboard usage.

3. **No Network:** Application is fully offline - no security surface there.

---

## Summary of Recommendations

| Priority | Issue | Effort |
|----------|-------|--------|
| P0 | Fix Cargo.toml edition | 1 min |
| P1 | Consolidate LTTB implementations | 30 min |
| P1 | Cache series panel violations | 1 hr |
| P1 | Add downsample indicator | 30 min |
| P1 | Fix alternate plot x-axis labels | 1-2 hr |
| P2 | Improve minimap accuracy | 1 hr |
| P2 | Add date-aware filters | 2-3 hr |
| P2 | Add advanced table filters | 3-4 hr |
| P2 | Clean up dead code | 1-2 hr |
| P2 | Add Windows icon | 30 min |
| P3 | Move magic numbers to constants | 30 min |

---

*End of Review*
