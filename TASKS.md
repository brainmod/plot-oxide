# PlotOxide Task List
**Last Updated:** 2025-11-30  
**Status:** Post-Phase 6 (Performance Complete)

---

## Legend
- 🔴 **P0** - Critical/Blocking
- 🟠 **P1** - High Priority
- 🟡 **P2** - Medium Priority  
- 🟢 **P3** - Nice to Have
- ✅ Complete | 🔄 In Progress | ⏳ Planned | ❌ Won't Do

---

## Critical Fixes (P0)

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | Fix Cargo.toml `edition = "2024"` → `"2021"` | Invalid edition, could break on stricter toolchains |

---

## High Priority (P1)

### Code Health

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | Consolidate LTTB implementations | Currently in `app.rs`, `downsample.rs`, `worker.rs` |
| ⏳ | Cache series panel violation indicators | Currently recalculates for ALL columns every frame |

### User Experience

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | **Downsample rate indicator** | Show "Showing N of M points" or downsample ratio |
| ⏳ | **Fix alternate plot X-axis labels** | Histogram, Pareto, BoxPlot, XbarR, PChart need proper labels |

---

## Medium Priority (P2)

### User Experience

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | **Improve minimap accuracy** | Currently uses naive stepping; should use LTTB or envelope |
| ⏳ | **Date range filters for X-axis** | Detect timestamp columns, show date-aware input |
| ⏳ | **Advanced table filters** | Numeric (>, <, between), outliers, empty/non-empty |
| ⏳ | Timezone support for timestamps | Display in local time, configurable timezone |
| ⏳ | X-axis range display in stats panel | Show filtered time/value range context |
| ⏳ | Panel collapse/auto-hide | Maximize plot area on demand |

### Build & Distribution

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | **Add Windows icon (.ico)** | `icon.png` exists, need `.ico` + `winres`/`embed-resource` |
| ⏳ | macOS app bundle configuration | `.app` structure, `Info.plist` |
| ⏳ | Linux desktop file | `.desktop` entry for app launchers |

### Code Quality

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | Dead code audit | ~35 `#[allow(dead_code)]` warnings |
| ⏳ | Standardize clipboard API | Use `ctx.copy_text()` consistently |
| ⏳ | Move magic numbers to constants | Minimap size, point tolerance, etc. |
| ⏳ | Add bounds checking in plot.rs | `y_indices[series_idx]` potential panic |

---

## Nice to Have (P3)

### Features

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | Export filtered/downsampled data | "What you see is what you export" |
| ⏳ | Column statistics comparison | Side-by-side stats for multiple series |
| ⏳ | Custom date format input | Currently relies on Polars auto-detect |
| ⏳ | Annotation/marker system | Let users mark specific points |
| ⏳ | Session state persistence | Remember last file, view settings |
| ⏳ | Print/PDF export | Plot image export |

### Performance

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | String column caching | `column_as_string()` could cache like numeric |
| ⏳ | Parallel statistics computation | Use Rayon for multi-series stats |
| ⏳ | Lazy filter evaluation | Only compute visible rows for large datasets |

### Testing

| Status | Task | Notes |
|--------|------|-------|
| ⏳ | LTTB accuracy tests | Verify downsampled output maintains shape |
| ⏳ | Filter edge case tests | NaN, empty, single-value datasets |
| ⏳ | Integration test coverage | End-to-end scenarios |

---

## Completed ✅

### Phase 6 (Nov 2025) - Performance

| Status | Task | Notes |
|--------|------|-------|
| ✅ | Column string prefetch in data table | 35x improvement (1500ms → 42ms) |
| ✅ | Stats panel column-major access | Fixed `app.data()` materialization |
| ✅ | Pre-computed filter/sort indices | No per-frame recomputation |
| ✅ | CachedStats with version tracking | Stats calculated once per data load |
| ✅ | Profiling integration (puffin/tracy) | `--features profile-with-puffin` |
| ✅ | Edge indicators (gradient + arrows) | Show when data extends beyond view |
| ✅ | Minimap overview | Auto-appearing when zoomed >1.5x |
| ✅ | Enhanced stats (percentiles, sparkline) | P5/P25/P75/P95, histogram bars |
| ✅ | Table sorting | Click column headers |
| ✅ | Table search highlighting | Yellow highlight on matches |
| ✅ | Table row selection + copy | Checkboxes, Ctrl+A, Ctrl+C |
| ✅ | Go-to-row input | Jump to specific row |

### Phase 5 - Background Threading

| Status | Task | Notes |
|--------|------|-------|
| ✅ | BackgroundWorker with channels | Non-blocking file loading |
| ✅ | DataSource::from_dataframe() | Worker integration |

### Phase 4 - Rendering Optimizations

| Status | Task | Notes |
|--------|------|-------|
| ✅ | Point culling (binary search) | O(log n) viewport filtering |
| ✅ | PlotBuffer pre-allocation | Avoid per-frame allocations |
| ✅ | Adaptive downsampling | Fast nth-point during drag, LTTB when settled |

### Phase 3 - Virtual Scrolling

| Status | Task | Notes |
|--------|------|-------|
| ✅ | TableBuilder virtual rows | O(visible) rendering |

### Phase 2 - LTTB Caching

| Status | Task | Notes |
|--------|------|-------|
| ✅ | LttbCache with zoom quantization | 10-50x fewer recomputes |

### Phase 1 - Data Pipeline

| Status | Task | Notes |
|--------|------|-------|
| ✅ | Polars/Parquet migration | Lazy + materialized DataFrames |

---

## Won't Do ❌

| Task | Reason |
|------|--------|
| puffin_egui in-app viewer | Version incompatibility; use external viewer |
| Date picker UI | egui date pickers clunky; range inputs sufficient |
| Relative time display | Niche use case, clutters UI |

---

## Notes

### Build Commands
```bash
# Standard build
cargo build --release

# With profiling
cargo build --release --features profile-with-puffin
puffin_viewer  # In another terminal

# Run tests
cargo test
```

### Performance Targets
| Metric | Target | Current |
|--------|--------|---------|
| File load (100k CSV) | <100ms | 32ms ✅ |
| Data table render | <50ms | 42ms ✅ |
| Plot render (panning) | <15ms | 9.8ms ✅ |
| Stats calculation | <5ms | 2ms ✅ |

### Technical Debt Tracking
- ~35 dead code warnings (low priority)
- LTTB duplication (high priority)
- Magic numbers scattered (low priority)
