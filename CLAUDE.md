# PlotOxide Development Guide

## Refactoring Status

| Phase | Status | Description |
|-------|--------|-------------|
| 1 | ✅ Complete | Polars/Parquet migration + polish |
| 2 | ✅ Complete | Idiomatic Rust improvements |
| 3 | ✅ Complete | StripBuilder layout |
| 4 | ✅ Complete | Modular widget system |
| 5 | ✅ Complete | UI module extraction |

**All refactoring phases complete as of 2025-11-27.**

---

## Architecture

```
src/
├── main.rs              # Entry point (~167 lines)
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
│   └── ui.rs            # UiState
├── ui/
│   ├── mod.rs
│   ├── toolbar.rs
│   ├── series_panel.rs
│   ├── plot.rs
│   ├── stats_panel.rs
│   ├── data_table.rs
│   └── help_dialog.rs
└── widgets/
    ├── mod.rs
    ├── spc_controls.rs
    ├── filter_controls.rs
    └── range_input.rs
```

---

## Future Roadmap

### High Priority
| Feature | Rationale | Effort |
|---------|-----------|--------|
| Timezone support | Manufacturing data needs local time display; polars temporal features available | Medium |
| X-axis range in stats panel | Show filtered time/value range context alongside Y stats | Low |

### Deferred
| Feature | Rationale |
|---------|-----------|
| Custom date format | Polars auto-detection handles most cases; add if users request |

### Not Planned
| Feature | Rationale |
|---------|-----------|
| Date picker UI | egui date pickers are clunky; current range inputs sufficient |
| Relative time display | Niche; clutters UI |
| Axis label customization | egui_plot handles adequately |
| Epoch conversion utilities | Polars handles natively |

---

## Technical Debt

| Item | Location | Priority |
|------|----------|----------|
| Unused `LayoutMode` | state/view.rs | Low |
| Some dead code warnings | Various unused helpers | Low |

---

## Test Coverage

- 9 tests passing (3 stats + 3 integration + 2 error + 1 performance)
- All tests use `tempfile` for CSV creation
- Performance test validates 100k row handling (<125ms)

---

## Performance

Validated with 100k row dataset:
| Operation | Time |
|-----------|------|
| Load CSV | 32ms |
| Row-major conversion | 90ms |
| Stats calculation | 2ms |
| **Total** | **124ms** |

LTTB downsampling at 5000 points. Outlier stats cached per-column.
