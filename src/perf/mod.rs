//! Performance optimization module
//!
//! Implements all phases from PERFORMANCE_ROADMAP.md:
//! - Phase 0: Instrumentation (timing macro, puffin integration)
//! - Phase 2: LTTB caching with zoom quantization
//! - Phase 4: Rendering optimizations (culling, buffers, adaptive downsampling)
//! - Phase 5: Background threading
//! - Phase 6: Memory optimization (Arc-based shared data)

#![allow(dead_code)]

mod cache;
mod worker;
mod downsample;

pub use cache::LttbCache;
pub use worker::{BackgroundWorker, WorkerResult};
pub use downsample::AdaptiveDownsampler;

use std::sync::Arc;

/// Shared immutable point data (Phase 6.1)
pub type SharedPoints = Arc<[(f64, f64)]>;

/// Convert Vec to Arc slice for cheap cloning
pub fn to_shared(v: Vec<(f64, f64)>) -> SharedPoints {
    v.into()
}

/// Timing macro for instrumentation (Phase 0)
#[macro_export]
macro_rules! timed {
    ($name:expr, $block:expr) => {{
        let _t = std::time::Instant::now();
        let r = $block;
        #[cfg(debug_assertions)]
        eprintln!("{}: {:?}", $name, _t.elapsed());
        r
    }};
}

/// Point culling - only return points within visible range with margin (Phase 4.1)
#[inline]
pub fn cull_points(data: &[[f64; 2]], x_min: f64, x_max: f64) -> &[[f64; 2]] {
    if data.is_empty() {
        return data;
    }
    
    // Binary search for range bounds (data assumed sorted by x)
    let start = data.partition_point(|p| p[0] < x_min);
    let end = data.partition_point(|p| p[0] <= x_max);
    
    // Add small margin for smooth panning
    let margin = 10;
    let start = start.saturating_sub(margin);
    let end = (end + margin).min(data.len());
    
    &data[start..end]
}

/// Pre-allocated plot buffer to avoid per-frame allocations (Phase 4.2)
pub struct PlotBuffer {
    points: Vec<[f64; 2]>,
}

impl PlotBuffer {
    pub fn new() -> Self {
        Self {
            points: Vec::with_capacity(10_000),
        }
    }
    
    pub fn fill_from(&mut self, source: &[[f64; 2]]) -> &[[f64; 2]] {
        self.points.clear();
        self.points.extend_from_slice(source);
        &self.points
    }
    
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

impl Default for PlotBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cull_points() {
        let data: Vec<[f64; 2]> = (0..1000).map(|i| [i as f64, (i as f64).sin()]).collect();
        
        let culled = cull_points(&data, 100.0, 200.0);
        
        // Should have ~100 points plus margins
        assert!(culled.len() >= 100);
        assert!(culled.len() <= 130);
    }
    
    #[test]
    fn test_plot_buffer() {
        let mut buf = PlotBuffer::new();
        let source = vec![[1.0, 2.0], [3.0, 4.0]];
        
        let result = buf.fill_from(&source);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], [1.0, 2.0]);
    }
}
