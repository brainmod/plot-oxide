# PlotOxide Performance Optimization Roadmap

High-performance data visualization requires coordinated optimization across the entire pipeline: storage → loading → transformation → rendering. This document outlines a systematic approach to maximizing performance for massive datasets.

## Architecture Overview

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Parquet   │───▶│   Polars    │───▶│  LTTB/LOD   │───▶│  egui_plot  │
│   Storage   │    │   Loading   │    │    Cache    │    │  Rendering  │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
      │                  │                  │                  │
      ▼                  ▼                  ▼                  ▼
  Pre-computed       Column proj.       Zoom-level        Culling +
  LOD tiers          Predicates         quantization      batching
```

---

## Phase 0: Instrumentation

**Goal**: Identify actual bottlenecks before optimizing.

### Minimal Timing Macro

```rust
use std::time::Instant;

macro_rules! timed {
    ($name:expr, $block:expr) => {{
        let t = Instant::now();
        let r = $block;
        #[cfg(debug_assertions)]
        eprintln!("{}: {:?}", $name, t.elapsed());
        r
    }};
}
```

### Puffin Integration (Recommended)

Add to `Cargo.toml`:
```toml
[dependencies]
puffin = "0.19"
puffin_egui = "0.27"

[features]
profiling = ["puffin/puffin_http"]
```

Integration:
```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    puffin::profile_function!();
    puffin::GlobalProfiler::lock().new_frame();
    
    puffin_egui::profiler_window(ctx);
    
    // ... rest of update
}
```

### Key Measurement Points

| Location | What to Measure |
|----------|-----------------|
| File load | Parquet scan → collect duration |
| LTTB | Downsample computation time |
| Table render | Per-frame table body time |
| Plot render | Per-frame plot widget time |
| Total frame | Full `update()` duration |

### Success Criteria
- Frame time consistently < 16ms (60 FPS)
- Load time < 1s for typical files
- Pan/zoom feels instantaneous

---

## Phase 1: Data Pipeline Optimization

### 1.1 Parquet Loading

**Problem**: Loading entire files with all columns wastes IO and memory.

**Before**:
```rust
let df = LazyFrame::scan_parquet("data.parquet", Default::default())?
    .collect()?;
```

**After**:
```rust
let df = LazyFrame::scan_parquet("data.parquet", ScanArgsParquet {
    low_memory: true,
    parallel: polars::io::parquet::ParallelStrategy::Auto,
    ..Default::default()
})?
.select([col("timestamp"), col("value")])  // Column pruning
.filter(
    col("timestamp")
        .gt(lit(range_start))
        .and(col("timestamp").lt(lit(range_end)))
)  // Predicate pushdown
.collect()?;
```

**Expected Impact**: 2-10x improvement depending on column count and filter selectivity.

### 1.2 Pre-computed LOD Tiers

**Problem**: Running LTTB on millions of points at runtime is expensive.

**Solution**: Generate resolution tiers at import time.

#### File Structure
```
project_data/
├── raw.parquet           # Full resolution (millions of points)
├── lod_100k.parquet      # ~100k points
├── lod_10k.parquet       # ~10k points  
└── lod_1k.parquet        # ~1k points (overview)
```

#### Import-Time Generation
```rust
pub fn generate_lod_tiers(
    raw_path: &Path,
    output_dir: &Path,
    tiers: &[(&str, usize)],  // [("100k", 100_000), ("10k", 10_000), ...]
) -> Result<()> {
    let raw_df = LazyFrame::scan_parquet(raw_path, Default::default())?
        .collect()?;
    
    let raw_points = extract_points(&raw_df);
    
    for (name, target_count) in tiers {
        let downsampled = lttb_downsample(&raw_points, *target_count);
        let tier_df = points_to_dataframe(&downsampled);
        
        let tier_path = output_dir.join(format!("lod_{}.parquet", name));
        tier_df.write_parquet(&tier_path, Default::default())?;
    }
    
    Ok(())
}
```

#### Runtime Tier Selection
```rust
fn select_lod_tier(visible_range: f64, total_range: f64) -> &'static str {
    let zoom_ratio = visible_range / total_range;
    
    match zoom_ratio {
        r if r > 0.5 => "1k",      // Zoomed way out
        r if r > 0.1 => "10k",     // Moderate zoom
        r if r > 0.01 => "100k",   // Close zoom
        _ => "raw",                 // Tight zoom - need full detail
    }
}
```

**Expected Impact**: Eliminates LTTB entirely for zoomed-out views. 10-100x faster initial render.

---

## Phase 2: LTTB Caching Strategy

### 2.1 Zoom-Level Quantization

**Problem**: Every small zoom change triggers full LTTB recompute.

**Solution**: Quantize zoom levels into discrete buckets.

```rust
use std::collections::HashMap;

pub struct LttbCache {
    /// Key: (series_id, zoom_bucket)
    /// Value: downsampled points
    cache: HashMap<(usize, i32), Vec<[f64; 2]>>,
    target_points: usize,
}

impl LttbCache {
    pub fn new(target_points: usize) -> Self {
        Self {
            cache: HashMap::new(),
            target_points,
        }
    }
    
    /// Quantize zoom level to reduce cache invalidation
    fn zoom_bucket(visible_range: f64) -> i32 {
        // Each bucket represents ~40% zoom change (2^0.5)
        (visible_range.log2() * 2.0).floor() as i32
    }
    
    pub fn get_or_compute(
        &mut self,
        series_id: usize,
        visible_range: (f64, f64),
        raw_data: &[(f64, f64)],
    ) -> &[[f64; 2]] {
        let range_width = visible_range.1 - visible_range.0;
        let bucket = Self::zoom_bucket(range_width);
        
        self.cache.entry((series_id, bucket)).or_insert_with(|| {
            // Add margin for smooth panning
            let margin = range_width * 0.2;
            let start = visible_range.0 - margin;
            let end = visible_range.1 + margin;
            
            // Filter to extended range
            let filtered: Vec<_> = raw_data.iter()
                .filter(|(x, _)| *x >= start && *x <= end)
                .copied()
                .collect();
            
            // Downsample
            lttb_downsample(&filtered, self.target_points)
        })
    }
    
    /// Clear cache when data changes
    pub fn invalidate(&mut self) {
        self.cache.clear();
    }
    
    /// Clear specific series
    pub fn invalidate_series(&mut self, series_id: usize) {
        self.cache.retain(|(id, _), _| *id != series_id);
    }
}
```

### 2.2 Cache Sizing

```rust
impl LttbCache {
    /// Limit memory usage by evicting old entries
    pub fn enforce_limit(&mut self, max_entries: usize) {
        if self.cache.len() > max_entries {
            // Simple strategy: clear half the cache
            // More sophisticated: LRU eviction
            let to_remove: Vec<_> = self.cache.keys()
                .take(self.cache.len() / 2)
                .cloned()
                .collect();
            
            for key in to_remove {
                self.cache.remove(&key);
            }
        }
    }
}
```

**Expected Impact**: 10-50x fewer LTTB computations during interactive use.

---

## Phase 3: Table View Optimization

### 3.1 Virtual Scrolling

**Problem**: Rendering all rows kills performance.

**Solution**: Only render visible rows using `egui_extras::TableBuilder`.

```rust
use egui_extras::{TableBuilder, Column};

fn render_data_table(ui: &mut egui::Ui, df: &DataFrame) {
    let row_height = 18.0;
    let total_rows = df.height();
    
    TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .column(Column::auto().at_least(80.0))  // Index
        .column(Column::auto().at_least(150.0)) // Timestamp
        .column(Column::remainder())             // Value
        .header(20.0, |mut header| {
            header.col(|ui| { ui.strong("Index"); });
            header.col(|ui| { ui.strong("Timestamp"); });
            header.col(|ui| { ui.strong("Value"); });
        })
        .body(|body| {
            // Key: rows() only calls closure for visible rows
            body.rows(row_height, total_rows, |mut row| {
                let idx = row.index();
                
                // Direct DataFrame indexing - no allocation
                row.col(|ui| {
                    ui.label(format!("{}", idx));
                });
                row.col(|ui| {
                    if let Ok(val) = df["timestamp"].get(idx) {
                        ui.label(val.to_string());
                    }
                });
                row.col(|ui| {
                    if let Ok(val) = df["value"].get(idx) {
                        ui.label(format!("{:.6}", val));
                    }
                });
            });
        });
}
```

### 3.2 Format Caching

**Problem**: `format!()` on every frame for visible rows adds up.

**Solution**: Cache formatted strings for visible window.

```rust
pub struct TableFormatCache {
    formatted: Vec<[String; 3]>,  // [index, timestamp, value]
    start_idx: usize,
    len: usize,
}

impl TableFormatCache {
    pub fn update_if_needed(
        &mut self,
        df: &DataFrame,
        visible_start: usize,
        visible_count: usize,
    ) {
        // Only reformat if window moved significantly
        if self.start_idx == visible_start && self.len >= visible_count {
            return;
        }
        
        self.start_idx = visible_start;
        self.len = visible_count + 20;  // Buffer for smooth scroll
        self.formatted.clear();
        
        let end = (visible_start + self.len).min(df.height());
        
        for i in visible_start..end {
            self.formatted.push([
                format!("{}", i),
                df["timestamp"].get(i).map(|v| v.to_string()).unwrap_or_default(),
                df["value"].get(i).map(|v| format!("{:.6}", v)).unwrap_or_default(),
            ]);
        }
    }
    
    pub fn get(&self, idx: usize) -> Option<&[String; 3]> {
        if idx >= self.start_idx && idx < self.start_idx + self.formatted.len() {
            Some(&self.formatted[idx - self.start_idx])
        } else {
            None
        }
    }
}
```

**Expected Impact**: Table goes from O(total_rows) to O(visible_rows) per frame.

---

## Phase 4: Rendering Optimization

### 4.1 Point Culling

**Problem**: Sending off-screen points to egui_plot wastes GPU bandwidth.

```rust
/// Binary search for visible range (assumes x-sorted data)
fn cull_to_visible(data: &[[f64; 2]], bounds: &PlotBounds) -> &[[f64; 2]] {
    let x_min = bounds.min()[0];
    let x_max = bounds.max()[0];
    
    // Add small margin for line continuity at edges
    let margin = (x_max - x_min) * 0.01;
    
    let start = data.partition_point(|p| p[0] < x_min - margin);
    let end = data.partition_point(|p| p[0] <= x_max + margin);
    
    &data[start..end]
}
```

### 4.2 Pre-allocated Point Buffers

**Problem**: Allocating `Vec` every frame causes GC pressure.

```rust
pub struct PlotBuffer {
    points: Vec<[f64; 2]>,
}

impl PlotBuffer {
    pub fn new() -> Self {
        Self {
            points: Vec::with_capacity(10_000),
        }
    }
    
    pub fn fill_from(&mut self, source: &[[f64; 2]]) -> PlotPoints {
        self.points.clear();
        self.points.extend_from_slice(source);
        PlotPoints::Owned(std::mem::take(&mut self.points))
    }
    
    /// Call after plot render to reclaim buffer
    pub fn reclaim(&mut self, points: PlotPoints) {
        if let PlotPoints::Owned(mut v) = points {
            v.clear();
            self.points = v;
        }
    }
}
```

### 4.3 Simplified Pan Rendering

**Problem**: Full LTTB during fast pan is overkill.

**Solution**: Use faster nth-point sampling during motion, LTTB when settled.

```rust
pub struct AdaptiveDownsampler {
    is_interacting: bool,
    settle_frames: u8,
}

impl AdaptiveDownsampler {
    pub fn downsample(
        &mut self,
        data: &[(f64, f64)],
        target: usize,
        currently_dragging: bool,
    ) -> Vec<[f64; 2]> {
        if currently_dragging {
            self.is_interacting = true;
            self.settle_frames = 10;
            // Fast nth-point sampling during interaction
            return self.nth_point_sample(data, target);
        }
        
        if self.settle_frames > 0 {
            self.settle_frames -= 1;
            return self.nth_point_sample(data, target);
        }
        
        // Full LTTB when settled
        self.is_interacting = false;
        lttb_downsample(data, target)
    }
    
    fn nth_point_sample(&self, data: &[(f64, f64)], target: usize) -> Vec<[f64; 2]> {
        let step = (data.len() / target).max(1);
        data.iter()
            .step_by(step)
            .map(|&(x, y)| [x, y])
            .collect()
    }
}
```

**Expected Impact**: Buttery smooth pan/zoom, LTTB quality when stationary.

---

## Phase 5: Background Threading

### 5.1 Channel-Based Architecture

**Problem**: Heavy computation blocks UI thread.

```rust
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub enum WorkerRequest {
    LoadFile { path: PathBuf },
    ComputeLttb { series_id: usize, range: (f64, f64), data: Arc<[(f64, f64)]> },
}

pub enum WorkerResult {
    FileLoaded { df: DataFrame },
    LttbReady { series_id: usize, points: Vec<[f64; 2]> },
    Error { msg: String },
}

pub struct BackgroundWorker {
    tx: Sender<WorkerRequest>,
    rx: Receiver<WorkerResult>,
}

impl BackgroundWorker {
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = channel::<WorkerRequest>();
        let (res_tx, res_rx) = channel::<WorkerResult>();
        
        thread::spawn(move || {
            while let Ok(request) = req_rx.recv() {
                let result = match request {
                    WorkerRequest::LoadFile { path } => {
                        match load_parquet(&path) {
                            Ok(df) => WorkerResult::FileLoaded { df },
                            Err(e) => WorkerResult::Error { msg: e.to_string() },
                        }
                    }
                    WorkerRequest::ComputeLttb { series_id, range, data } => {
                        let points = compute_lttb_for_range(&data, range);
                        WorkerResult::LttbReady { series_id, points }
                    }
                };
                
                if res_tx.send(result).is_err() {
                    break;
                }
            }
        });
        
        Self { tx: req_tx, rx: res_rx }
    }
    
    pub fn request(&self, req: WorkerRequest) {
        let _ = self.tx.send(req);
    }
    
    /// Non-blocking poll - call each frame
    pub fn poll(&self) -> Option<WorkerResult> {
        self.rx.try_recv().ok()
    }
}
```

### 5.2 UI Integration

```rust
impl App {
    fn update(&mut self, ctx: &egui::Context) {
        // Poll for completed work
        while let Some(result) = self.worker.poll() {
            match result {
                WorkerResult::FileLoaded { df } => {
                    self.data = Some(df);
                    self.loading = false;
                }
                WorkerResult::LttbReady { series_id, points } => {
                    self.plot_cache.insert(series_id, points);
                }
                WorkerResult::Error { msg } => {
                    self.error = Some(msg);
                }
            }
        }
        
        // Render with current (possibly stale) data
        // Shows loading indicator if self.loading
    }
}
```

**Expected Impact**: UI stays at 60 FPS regardless of computation load.

---

## Phase 6: Memory Optimization

### 6.1 Shared Immutable Data

```rust
use std::sync::Arc;

pub struct SeriesData {
    // Arc allows cheap clones for passing to worker threads
    points: Arc<[(f64, f64)]>,
}

impl SeriesData {
    pub fn from_vec(v: Vec<(f64, f64)>) -> Self {
        Self {
            points: v.into(),
        }
    }
}
```

### 6.2 Memory-Conscious LOD

```rust
pub struct LodManager {
    /// Only keep one resolution in memory at a time (plus raw if zoomed in)
    current_tier: LodTier,
    raw_data: Option<Arc<[(f64, f64)]>>,
    lod_data: Arc<[(f64, f64)]>,
}

impl LodManager {
    pub fn set_zoom_level(&mut self, zoom: f64) {
        let needed_tier = LodTier::for_zoom(zoom);
        
        if needed_tier != self.current_tier {
            // Load new tier, drop old
            self.lod_data = self.load_tier(needed_tier);
            self.current_tier = needed_tier;
            
            // Only keep raw data if we might need it
            if needed_tier != LodTier::Raw {
                self.raw_data = None;
            }
        }
    }
}
```

### 6.3 Streaming Large Files

```rust
/// Process file in chunks for memory-constrained environments
pub fn stream_process_parquet(
    path: &Path,
    chunk_size: usize,
    mut processor: impl FnMut(&DataFrame),
) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = ParquetReader::new(file).with_batch_size(chunk_size);
    
    for batch in reader.iter_batches()? {
        processor(&batch?);
    }
    
    Ok(())
}
```

---

## Implementation Checklist

### Priority 1 (Highest Impact)
- [ ] Add instrumentation (Phase 0)
- [ ] Implement column projection + predicate pushdown (Phase 1.1)
- [ ] Implement LTTB caching with zoom quantization (Phase 2.1)
- [ ] Fix table virtual scrolling (Phase 3.1)

### Priority 2 (Significant Impact)
- [ ] Pre-compute LOD tiers at import (Phase 1.2)
- [ ] Add point culling (Phase 4.1)
- [ ] Background thread for file loading (Phase 5)

### Priority 3 (Polish)
- [ ] Table format caching (Phase 3.2)
- [ ] Adaptive downsampling during interaction (Phase 4.3)
- [ ] Memory optimization with Arc (Phase 6.1)
- [ ] Pre-allocated point buffers (Phase 4.2)

---

## Benchmarks to Track

| Metric | Baseline | Target | Notes |
|--------|----------|--------|-------|
| Cold load (1M points) | TBD ms | < 500ms | Measure with `timed!` |
| Frame time (idle) | TBD ms | < 8ms | `puffin` |
| Frame time (pan) | TBD ms | < 12ms | |
| Frame time (zoom) | TBD ms | < 16ms | |
| Memory (1M points) | TBD MB | < 100MB | |
| Table scroll (100k rows) | TBD ms/frame | < 4ms | |

---

## References

- [LTTB Algorithm Paper](https://skemman.is/bitstream/1946/15343/3/SS_MSthesis.pdf)
- [egui_extras TableBuilder](https://docs.rs/egui_extras/latest/egui_extras/struct.TableBuilder.html)
- [Polars Lazy API](https://pola-rs.github.io/polars/user-guide/lazy/intro/)
- [puffin profiler](https://github.com/EmbarkStudios/puffin)
