//! Adaptive downsampling for smooth interaction (Phase 4.3)
//!
//! Uses fast nth-point sampling during pan/zoom, LTTB when settled.

/// Adaptive downsampler that switches between fast and quality modes
pub struct AdaptiveDownsampler {
    is_interacting: bool,
    settle_frames: u8,
}

impl AdaptiveDownsampler {
    pub fn new() -> Self {
        Self {
            is_interacting: false,
            settle_frames: 0,
        }
    }
    
    /// Downsample data, using fast method during interaction
    pub fn downsample(
        &mut self,
        data: &[(f64, f64)],
        target: usize,
        currently_dragging: bool,
    ) -> Vec<[f64; 2]> {
        if data.len() <= target {
            return data.iter().map(|&(x, y)| [x, y]).collect();
        }
        
        if currently_dragging {
            self.is_interacting = true;
            self.settle_frames = 10;
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
    
    /// Fast nth-point sampling for interaction
    fn nth_point_sample(&self, data: &[(f64, f64)], target: usize) -> Vec<[f64; 2]> {
        let step = (data.len() / target).max(1);
        data.iter()
            .step_by(step)
            .map(|&(x, y)| [x, y])
            .collect()
    }
    
    /// Check if currently in fast mode
    pub fn is_fast_mode(&self) -> bool {
        self.is_interacting || self.settle_frames > 0
    }
    
    /// Force settle (useful after data change)
    pub fn force_settle(&mut self) {
        self.settle_frames = 0;
        self.is_interacting = false;
    }
}

impl Default for AdaptiveDownsampler {
    fn default() -> Self {
        Self::new()
    }
}

/// LTTB (Largest Triangle Three Buckets) downsampling algorithm
pub fn lttb_downsample(data: &[(f64, f64)], target: usize) -> Vec<[f64; 2]> {
    if data.len() <= target {
        return data.iter().map(|&(x, y)| [x, y]).collect();
    }
    
    let mut result = Vec::with_capacity(target);
    
    // Always include first point
    result.push([data[0].0, data[0].1]);
    
    let bucket_size = (data.len() - 2) as f64 / (target - 2) as f64;
    let mut a = 0usize;
    
    for i in 0..(target - 2) {
        let bucket_start = ((i as f64 + 1.0) * bucket_size).floor() as usize + 1;
        let bucket_end = (((i as f64 + 2.0) * bucket_size).floor() as usize + 1).min(data.len() - 1);
        
        // Average of next bucket
        let next_start = bucket_end;
        let next_end = (((i as f64 + 3.0) * bucket_size).floor() as usize + 1).min(data.len());
        
        let (avg_x, avg_y) = if next_start < next_end {
            let sum: (f64, f64) = data[next_start..next_end]
                .iter()
                .fold((0.0, 0.0), |acc, &(x, y)| (acc.0 + x, acc.1 + y));
            let count = (next_end - next_start) as f64;
            (sum.0 / count, sum.1 / count)
        } else {
            data[data.len() - 1]
        };
        
        // Find point with largest triangle area
        let mut max_area = -1.0f64;
        let mut max_idx = bucket_start;
        let (ax, ay) = data[a];
        
        for j in bucket_start..bucket_end {
            let (bx, by) = data[j];
            let area = ((ax - avg_x) * (by - ay) - (ax - bx) * (avg_y - ay)).abs();
            if area > max_area {
                max_area = area;
                max_idx = j;
            }
        }
        
        result.push([data[max_idx].0, data[max_idx].1]);
        a = max_idx;
    }
    
    // Always include last point
    result.push([data[data.len() - 1].0, data[data.len() - 1].1]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_fast_mode() {
        let mut ds = AdaptiveDownsampler::new();
        let data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, i as f64)).collect();
        
        // Fast mode during drag
        let _ = ds.downsample(&data, 100, true);
        assert!(ds.is_fast_mode());
        
        // Still fast while settling
        let _ = ds.downsample(&data, 100, false);
        assert!(ds.is_fast_mode());
        
        // After settle frames, should use full LTTB
        for _ in 0..15 {
            ds.downsample(&data, 100, false);
        }
        assert!(!ds.is_fast_mode());
    }
    
    #[test]
    fn test_lttb_endpoints() {
        let data: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, i as f64)).collect();
        let result = lttb_downsample(&data, 10);
        
        assert_eq!(result[0][0], 0.0);
        assert_eq!(result[9][0], 99.0);
    }
}
