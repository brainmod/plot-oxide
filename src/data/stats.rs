use polars::prelude::*;

/// Statistics results
#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub mean: f64,
    pub std_dev: f64,
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            mean: 0.0,
            std_dev: 0.0,
            median: 0.0,
            min: 0.0,
            max: 0.0,
            count: 0,
        }
    }
}

/// Calculate comprehensive statistics from a Series using polars
pub fn calculate_stats(series: &Series) -> Stats {
    let count = series.len();

    if count == 0 {
        return Stats::default();
    }

    // Cast to f64 for numeric operations
    let series_f64 = match series.cast(&DataType::Float64) {
        Ok(s) => s,
        Err(_) => return Stats::default(),
    };

    let chunked = match series_f64.f64() {
        Ok(c) => c,
        Err(_) => return Stats::default(),
    };

    Stats {
        mean: chunked.mean().unwrap_or(0.0),
        std_dev: chunked.std(1).unwrap_or(0.0), // ddof=1 for sample std dev
        median: chunked.median().unwrap_or(0.0),
        min: chunked.min().unwrap_or(0.0),
        max: chunked.max().unwrap_or(0.0),
        count,
    }
}

/// Calculate mean and std dev (compatible with existing code)
pub fn calculate_statistics(series: &Series) -> (f64, f64) {
    let stats = calculate_stats(series);
    (stats.mean, stats.std_dev)
}

/// Calculate mean and std dev from Vec<f64> (legacy compatibility)
pub fn calculate_statistics_vec(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }

    let series = Series::new("temp".into(), values);
    calculate_statistics(&series)
}

/// Calculate median (compatible with existing code)
pub fn calculate_median(series: &Series) -> f64 {
    calculate_stats(series).median
}

/// Calculate median from Vec<f64> (legacy compatibility)
pub fn calculate_median_vec(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let series = Series::new("temp".into(), values);
    calculate_median(&series)
}

/// Detect outliers using z-score method
pub fn detect_outliers(series: &Series, threshold: f64) -> Vec<usize> {
    let (mean, std_dev) = calculate_statistics(series);

    if std_dev == 0.0 {
        return vec![];
    }

    let series_f64 = match series.cast(&DataType::Float64) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let chunked = match series_f64.f64() {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    chunked
        .into_iter()
        .enumerate()
        .filter_map(|(idx, opt_val)| {
            opt_val.and_then(|v| {
                let z_score = ((v - mean) / std_dev).abs();
                if z_score > threshold {
                    Some(idx)
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Detect outliers from Vec<f64> (legacy compatibility)
pub fn detect_outliers_vec(values: &[f64], threshold: f64) -> Vec<usize> {
    if values.is_empty() {
        return vec![];
    }

    let series = Series::new("temp".into(), values);
    detect_outliers(&series, threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_stats() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let series = Series::new("test".into(), &data);
        let stats = calculate_stats(&series);

        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.median, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.count, 5);
    }

    #[test]
    fn test_detect_outliers() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 100.0]; // 100 is an outlier
        let series = Series::new("test".into(), &data);
        let outliers = detect_outliers(&series, 2.0);

        assert_eq!(outliers, vec![5]);
    }

    #[test]
    fn test_empty_series() {
        let data: Vec<f64> = vec![];
        let series = Series::new("test".into(), &data);
        let stats = calculate_stats(&series);

        assert_eq!(stats.mean, 0.0);
        assert_eq!(stats.count, 0);
    }
}
