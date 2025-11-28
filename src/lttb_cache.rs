/// LTTB downsampling cache with zoom-level quantization
use std::collections::HashMap;

/// Cache for LTTB downsampled data
/// Uses zoom-level quantization to reduce cache invalidation
pub struct LttbCache {
    /// Key: (series_id, zoom_bucket)
    /// Value: downsampled points
    cache: HashMap<(usize, i32), Vec<[f64; 2]>>,
    /// Target number of points for downsampling
    target_points: usize,
}

impl LttbCache {
    /// Create a new LTTB cache with specified target point count
    pub fn new(target_points: usize) -> Self {
        Self {
            cache: HashMap::new(),
            target_points,
        }
    }

    /// Quantize zoom level to reduce cache invalidation
    /// Each bucket represents ~40% zoom change (2^0.5)
    fn zoom_bucket(visible_range: f64) -> i32 {
        (visible_range.log2() * 2.0).floor() as i32
    }

    /// Get cached downsampled data or compute it
    /// Returns a reference to the cached data
    pub fn get_or_compute<F>(
        &mut self,
        series_id: usize,
        visible_range: (f64, f64),
        raw_data: &[(f64, f64)],
        downsample_fn: F,
    ) -> &[[f64; 2]]
    where
        F: FnOnce(&[(f64, f64)], usize) -> Vec<[f64; 2]>,
    {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();

        let range_width = visible_range.1 - visible_range.0;
        let bucket = Self::zoom_bucket(range_width);

        self.cache
            .entry((series_id, bucket))
            .or_insert_with(|| {
                puffin::profile_scope!("compute_lttb");

                // Add margin for smooth panning (20% on each side)
                let margin = range_width * 0.2;
                let start = visible_range.0 - margin;
                let end = visible_range.1 + margin;

                // Filter to extended range
                let filtered: Vec<_> = raw_data
                    .iter()
                    .filter(|(x, _)| *x >= start && *x <= end)
                    .copied()
                    .collect();

                // Downsample using provided function
                downsample_fn(&filtered, self.target_points)
            })
    }

    /// Clear all cached data
    pub fn invalidate(&mut self) {
        self.cache.clear();
    }

    /// Clear cached data for a specific series
    pub fn invalidate_series(&mut self, series_id: usize) {
        self.cache.retain(|(id, _), _| *id != series_id);
    }

    /// Limit memory usage by evicting old entries
    /// Uses a simple strategy: clear half the cache when limit is exceeded
    pub fn enforce_limit(&mut self, max_entries: usize) {
        if self.cache.len() > max_entries {
            puffin::profile_scope!("enforce_cache_limit");

            // Simple strategy: clear half the cache
            // More sophisticated approach would use LRU eviction
            let to_remove: Vec<_> = self
                .cache
                .keys()
                .take(self.cache.len() / 2)
                .cloned()
                .collect();

            for key in to_remove {
                self.cache.remove(&key);
            }
        }
    }

    /// Get the current number of cached entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_points: usize = self.cache.values().map(|v| v.len()).sum();
        CacheStats {
            entries: self.cache.len(),
            total_points,
            estimated_bytes: total_points * std::mem::size_of::<[f64; 2]>(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Number of cache entries
    pub entries: usize,
    /// Total number of cached points across all entries
    pub total_points: usize,
    /// Estimated memory usage in bytes
    pub estimated_bytes: usize,
}

impl Default for LttbCache {
    fn default() -> Self {
        Self::new(5000) // Default to 5000 points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_downsample(data: &[(f64, f64)], target: usize) -> Vec<[f64; 2]> {
        // Simple nth-point sampling for testing
        let step = (data.len() / target).max(1);
        data.iter()
            .step_by(step)
            .take(target)
            .map(|&(x, y)| [x, y])
            .collect()
    }

    #[test]
    fn test_cache_basic() {
        let mut cache = LttbCache::new(100);

        let raw_data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, i as f64)).collect();

        // First call should compute
        let result1 = cache.get_or_compute(0, (0.0, 100.0), &raw_data, dummy_downsample);
        let len1 = result1.len();
        assert!(!result1.is_empty());

        // Second call with same range should use cache (verify cache hit by checking length)
        let result2 = cache.get_or_compute(0, (0.0, 100.0), &raw_data, dummy_downsample);
        assert_eq!(len1, result2.len());
    }

    #[test]
    fn test_zoom_bucket_quantization() {
        // Similar zoom levels should map to same bucket
        let bucket1 = LttbCache::zoom_bucket(100.0);
        let bucket2 = LttbCache::zoom_bucket(110.0);
        assert_eq!(bucket1, bucket2);

        // Very different zoom levels should map to different buckets
        let bucket3 = LttbCache::zoom_bucket(1000.0);
        assert_ne!(bucket1, bucket3);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = LttbCache::new(100);
        let raw_data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, i as f64)).collect();

        cache.get_or_compute(0, (0.0, 100.0), &raw_data, dummy_downsample);
        assert!(!cache.is_empty());

        cache.invalidate();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_series_invalidation() {
        let mut cache = LttbCache::new(100);
        let raw_data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, i as f64)).collect();

        cache.get_or_compute(0, (0.0, 100.0), &raw_data, dummy_downsample);
        cache.get_or_compute(1, (0.0, 100.0), &raw_data, dummy_downsample);

        assert_eq!(cache.len(), 2);

        cache.invalidate_series(0);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_limit() {
        let mut cache = LttbCache::new(100);
        let raw_data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, i as f64)).collect();

        // Fill cache with many entries
        for i in 0..100 {
            cache.get_or_compute(i, (0.0, 100.0), &raw_data, dummy_downsample);
        }

        assert_eq!(cache.len(), 100);

        // Enforce limit
        cache.enforce_limit(50);
        assert_eq!(cache.len(), 50);
    }
}
