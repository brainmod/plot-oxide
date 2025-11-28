/// Level of Detail (LOD) tier management for massive datasets
use crate::app::PlotOxide;
use crate::data::DataError;
use polars::prelude::*;
use std::path::{Path, PathBuf};

/// LOD tier configuration
#[derive(Debug, Clone, Copy)]
pub enum LodTier {
    /// Full resolution (raw data)
    Raw,
    /// ~100k points (close zoom)
    High,
    /// ~10k points (moderate zoom)
    Medium,
    /// ~1k points (overview)
    Low,
}

impl LodTier {
    /// Get the target point count for this tier
    pub fn target_points(&self) -> Option<usize> {
        match self {
            LodTier::Raw => None, // No downsampling
            LodTier::High => Some(100_000),
            LodTier::Medium => Some(10_000),
            LodTier::Low => Some(1_000),
        }
    }

    /// Get the file suffix for this tier
    pub fn suffix(&self) -> &'static str {
        match self {
            LodTier::Raw => "raw",
            LodTier::High => "lod_100k",
            LodTier::Medium => "lod_10k",
            LodTier::Low => "lod_1k",
        }
    }

    /// Select appropriate LOD tier based on zoom ratio
    /// zoom_ratio = visible_range / total_range
    pub fn for_zoom(zoom_ratio: f64) -> Self {
        match zoom_ratio {
            r if r > 0.5 => LodTier::Low,    // Zoomed way out (viewing > 50%)
            r if r > 0.1 => LodTier::Medium, // Moderate zoom (viewing 10-50%)
            r if r > 0.01 => LodTier::High,  // Close zoom (viewing 1-10%)
            _ => LodTier::Raw,               // Tight zoom (<1%) - need full detail
        }
    }

    /// Get all tiers except Raw
    pub fn all_computed() -> Vec<LodTier> {
        vec![LodTier::Low, LodTier::Medium, LodTier::High]
    }
}

/// Generate LOD tiers for a dataset and save them to disk
pub fn generate_lod_tiers(
    raw_path: &Path,
    output_dir: &Path,
    x_column: &str,
    y_column: &str,
) -> Result<Vec<PathBuf>, DataError> {
    #[cfg(feature = "profiling")]
        puffin::profile_function!();

    // Load raw data
    let raw_df = {
        puffin::profile_scope!("load_raw");
        LazyFrame::scan_parquet(raw_path, Default::default())?
            .select([col(x_column), col(y_column)])
            .collect()?
    };

    let raw_points = extract_points(&raw_df, x_column, y_column)?;

    let mut generated_paths = Vec::new();

    // Generate each tier
    for tier in LodTier::all_computed() {
        puffin::profile_scope!("generate_tier");

        if let Some(target_count) = tier.target_points() {
            // Only generate if dataset is larger than target
            if raw_points.len() > target_count {
                let downsampled = PlotOxide::downsample_lttb(&raw_points, target_count);
                let tier_df = points_to_dataframe(&downsampled, x_column, y_column);

                let tier_path = output_dir.join(format!("{}.parquet", tier.suffix()));
                write_parquet(&tier_df, &tier_path)?;
                generated_paths.push(tier_path);
            }
        }
    }

    Ok(generated_paths)
}

/// Extract points from DataFrame
fn extract_points(
    df: &DataFrame,
    x_col: &str,
    y_col: &str,
) -> Result<Vec<[f64; 2]>, DataError> {
    let x_series = df
        .column(x_col)
        .map_err(|_| DataError::ColumnNotFound(x_col.to_string()))?;
    let y_series = df
        .column(y_col)
        .map_err(|_| DataError::ColumnNotFound(y_col.to_string()))?;

    // Convert to f64 vectors
    let x_vals: Vec<f64> = x_series
        .cast(&DataType::Float64)
        .map_err(|e| DataError::PolarsError(e))?
        .f64()
        .map_err(|e| DataError::PolarsError(e))?
        .into_iter()
        .map(|v| v.unwrap_or(f64::NAN))
        .collect();

    let y_vals: Vec<f64> = y_series
        .cast(&DataType::Float64)
        .map_err(|e| DataError::PolarsError(e))?
        .f64()
        .map_err(|e| DataError::PolarsError(e))?
        .into_iter()
        .map(|v| v.unwrap_or(f64::NAN))
        .collect();

    Ok(x_vals
        .into_iter()
        .zip(y_vals)
        .map(|(x, y)| [x, y])
        .collect())
}

/// Convert points back to DataFrame
fn points_to_dataframe(points: &[[f64; 2]], x_col: &str, y_col: &str) -> DataFrame {
    let x_vals: Vec<f64> = points.iter().map(|p| p[0]).collect();
    let y_vals: Vec<f64> = points.iter().map(|p| p[1]).collect();

    DataFrame::new(vec![
        Series::new(x_col.into(), x_vals).into(),
        Series::new(y_col.into(), y_vals).into(),
    ])
    .expect("Failed to create DataFrame from points")
}

/// Write DataFrame to Parquet file
fn write_parquet(df: &DataFrame, path: &Path) -> Result<(), DataError> {
    use polars::prelude::ParquetWriter;
    use std::fs::File;

    let file = File::create(path)?;
    let mut df_mut = df.clone();
    ParquetWriter::new(file)
        .finish(&mut df_mut)
        .map_err(|e| DataError::PolarsError(e))?;

    Ok(())
}

/// LOD tier manager for runtime tier selection
pub struct LodManager {
    base_path: PathBuf,
    current_tier: LodTier,
    available_tiers: Vec<LodTier>,
}

impl LodManager {
    /// Create a new LOD manager for a given base path
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            current_tier: LodTier::Raw,
            available_tiers: vec![LodTier::Raw],
        }
    }

    /// Scan directory for available LOD tiers
    pub fn scan_available_tiers(&mut self) -> Result<(), DataError> {
        self.available_tiers.clear();

        let dir = self
            .base_path
            .parent()
            .ok_or_else(|| DataError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No parent directory",
            )))?;

        // Always include raw
        self.available_tiers.push(LodTier::Raw);

        // Check for LOD tier files
        for tier in LodTier::all_computed() {
            let tier_path = dir.join(format!("{}.parquet", tier.suffix()));
            if tier_path.exists() {
                self.available_tiers.push(tier);
            }
        }

        Ok(())
    }

    /// Get path for a specific tier
    pub fn tier_path(&self, tier: LodTier) -> PathBuf {
        match tier {
            LodTier::Raw => self.base_path.clone(),
            _ => {
                let dir = self.base_path.parent().unwrap_or(Path::new("."));
                dir.join(format!("{}.parquet", tier.suffix()))
            }
        }
    }

    /// Select best available tier for zoom level
    pub fn select_tier(&mut self, zoom_ratio: f64) -> LodTier {
        let ideal = LodTier::for_zoom(zoom_ratio);

        // Find best available tier (prefer higher detail if ideal not available)
        let mut best = LodTier::Raw;
        for &tier in &self.available_tiers {
            if matches!(tier, ideal) {
                self.current_tier = ideal;
                return ideal;
            }
            // Pick the tier with more detail if ideal not found
            if tier_detail_level(tier) <= tier_detail_level(ideal)
                && tier_detail_level(tier) > tier_detail_level(best)
            {
                best = tier;
            }
        }

        self.current_tier = best;
        best
    }
}

/// Helper to get detail level for comparison
fn tier_detail_level(tier: LodTier) -> u8 {
    match tier {
        LodTier::Low => 0,
        LodTier::Medium => 1,
        LodTier::High => 2,
        LodTier::Raw => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_selection() {
        assert!(matches!(LodTier::for_zoom(0.8), LodTier::Low));
        assert!(matches!(LodTier::for_zoom(0.3), LodTier::Medium));
        assert!(matches!(LodTier::for_zoom(0.05), LodTier::High));
        assert!(matches!(LodTier::for_zoom(0.005), LodTier::Raw));
    }

    #[test]
    fn test_tier_target_points() {
        assert_eq!(LodTier::Raw.target_points(), None);
        assert_eq!(LodTier::High.target_points(), Some(100_000));
        assert_eq!(LodTier::Medium.target_points(), Some(10_000));
        assert_eq!(LodTier::Low.target_points(), Some(1_000));
    }

    #[test]
    fn test_tier_detail_level() {
        assert!(tier_detail_level(LodTier::Raw) > tier_detail_level(LodTier::High));
        assert!(tier_detail_level(LodTier::High) > tier_detail_level(LodTier::Medium));
        assert!(tier_detail_level(LodTier::Medium) > tier_detail_level(LodTier::Low));
    }
}
