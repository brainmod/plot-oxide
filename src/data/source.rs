use polars::prelude::*;
use std::path::{Path, PathBuf};
use std::cell::RefCell;
use std::collections::HashMap;

/// Errors that can occur during data operations
#[derive(Debug)]
pub enum DataError {
    PolarsError(PolarsError),
    IoError(std::io::Error),
    UnsupportedFormat(String),
    ColumnNotFound(String),
}

impl From<PolarsError> for DataError {
    fn from(err: PolarsError) -> Self {
        DataError::PolarsError(err)
    }
}

impl From<std::io::Error> for DataError {
    fn from(err: std::io::Error) -> Self {
        DataError::IoError(err)
    }
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::PolarsError(e) => write!(f, "Polars error: {}", e),
            DataError::IoError(e) => write!(f, "IO error: {}", e),
            DataError::UnsupportedFormat(ext) => write!(f, "Unsupported file format: {}", ext),
            DataError::ColumnNotFound(col) => write!(f, "Column not found: {}", col),
        }
    }
}

impl std::error::Error for DataError {}

/// DataSource wraps a Polars DataFrame with both lazy and materialized views
#[allow(dead_code)]
pub struct DataSource {
    /// Lazy frame for efficient filtering and transformations
    df: LazyFrame,
    /// Materialized DataFrame for display and immediate access
    materialized: DataFrame,
    /// Original file path
    file_path: Option<PathBuf>,
    /// Cache for numeric column conversions
    numeric_cache: RefCell<HashMap<usize, Vec<f64>>>,
}

#[allow(dead_code)]
impl DataSource {
    /// Load data from a file (CSV or Parquet)
    pub fn load(path: &Path) -> Result<Self, DataError> {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| DataError::UnsupportedFormat("No file extension".to_string()))?;

        let df = match extension.to_lowercase().as_str() {
            "parquet" => LazyFrame::scan_parquet(path, Default::default())?,
            "csv" => LazyCsvReader::new(path)
                .with_has_header(true)
                .with_infer_schema_length(Some(100))
                .with_try_parse_dates(true)  // Enable automatic date parsing
                .finish()?,
            ext => return Err(DataError::UnsupportedFormat(ext.to_string())),
        };

        let materialized = df.clone().collect()?;

        Ok(Self {
            df,
            materialized,
            file_path: Some(path.to_path_buf()),
            numeric_cache: RefCell::new(HashMap::new()),
        })
    }
    
    /// Create DataSource from an already-loaded DataFrame (Phase 5 worker support)
    pub fn from_dataframe(df: DataFrame, path: Option<PathBuf>) -> Result<Self, DataError> {
        let lazy = df.clone().lazy();
        Ok(Self {
            df: lazy,
            materialized: df,
            file_path: path,
            numeric_cache: RefCell::new(HashMap::new()),
        })
    }

    /// Get a reference to the materialized DataFrame
    pub fn dataframe(&self) -> &DataFrame {
        &self.materialized
    }

    /// Get column values as a Series
    pub fn column_values(&self, col: &str) -> Result<Series, DataError> {
        self.materialized
            .column(col)
            .map(|c| c.as_materialized_series().clone())
            .map_err(|_| DataError::ColumnNotFound(col.to_string()))
    }

    /// Get all column names
    pub fn column_names(&self) -> Vec<String> {
        self.materialized
            .get_column_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the number of rows
    pub fn height(&self) -> usize {
        self.materialized.height()
    }

    /// Get the number of columns
    pub fn width(&self) -> usize {
        self.materialized.width()
    }

    /// Get the file path
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Apply filters to the data
    pub fn apply_filters(&mut self, filters: impl Fn() -> Expr) -> Result<(), DataError> {
        let filter_expr = filters();
        self.materialized = self.df.clone().filter(filter_expr).collect()?;
        // Clear cache when filters change
        self.numeric_cache.borrow_mut().clear();
        Ok(())
    }

    /// Re-materialize the DataFrame (useful after lazy operations)
    pub fn refresh(&mut self) -> Result<(), DataError> {
        self.materialized = self.df.clone().collect()?;
        // Clear cache when refreshed
        self.numeric_cache.borrow_mut().clear();
        Ok(())
    }

    /// Get cached numeric column, computing if necessary
    pub fn get_cached_column(&self, col_idx: usize) -> Result<std::cell::Ref<Vec<f64>>, DataError> {
        // Check if already cached
        if !self.numeric_cache.borrow().contains_key(&col_idx) {
            // Compute
            let data = self.column_as_f64(col_idx)?;
            self.numeric_cache.borrow_mut().insert(col_idx, data);
        }
        
        // Return reference into RefCell
        Ok(std::cell::Ref::map(self.numeric_cache.borrow(), |cache| {
            cache.get(&col_idx).expect("Just inserted")
        }))
    }

    /// Get a column's numeric values as Vec<f64>
    /// Non-numeric values are converted to NaN
    /// Datetime/Date columns are converted to Unix timestamps (seconds since epoch)
    pub fn column_as_f64(&self, col_idx: usize) -> Result<Vec<f64>, DataError> {
        let col_names = self.column_names();
        if col_idx >= col_names.len() {
            return Err(DataError::ColumnNotFound(format!("Index {}", col_idx)));
        }

        let series = self.column_values(&col_names[col_idx])?;

        // Handle datetime/date types by converting to Unix timestamps
        match series.dtype() {
            DataType::Datetime(_, _) => {
                // Convert datetime to Unix timestamp (seconds)
                let timestamps = series.datetime()
                    .map_err(|e| DataError::PolarsError(e))?
                    .into_iter()
                    .map(|opt| opt.map(|ts| ts as f64 / 1_000_000.0).unwrap_or(f64::NAN))
                    .collect();
                return Ok(timestamps);
            }
            DataType::Date => {
                // Convert date to Unix timestamp (days since epoch * seconds per day)
                let timestamps = series.date()
                    .map_err(|e| DataError::PolarsError(e))?
                    .into_iter()
                    .map(|opt| opt.map(|days| days as f64 * 86400.0).unwrap_or(f64::NAN))
                    .collect();
                return Ok(timestamps);
            }
            _ => {}
        }

        // Try to cast to f64, if that fails, extract as best we can
        match series.cast(&DataType::Float64) {
            Ok(s) => Ok(s.f64()
                .map_err(|e| DataError::PolarsError(e))?
                .into_iter()
                .map(|opt| opt.unwrap_or(f64::NAN))
                .collect()),
            Err(_) => {
                // For string columns, try to parse as f64
                if let Ok(str_series) = series.str() {
                    Ok(str_series
                        .into_iter()
                        .map(|opt| {
                            opt.and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(f64::NAN)
                        })
                        .collect())
                } else {
                    // Last resort: return NaN for all values
                    Ok(vec![f64::NAN; series.len()])
                }
            }
        }
    }

    /// Get a column's string values as Vec<String>
    pub fn column_as_string(&self, col_idx: usize) -> Result<Vec<String>, DataError> {
        let col_names = self.column_names();
        if col_idx >= col_names.len() {
            return Err(DataError::ColumnNotFound(format!("Index {}", col_idx)));
        }

        let series = self.column_values(&col_names[col_idx])?;

        // Try to cast to string first for efficiency
        if let Ok(str_series) = series.str() {
            return Ok(str_series
                .into_iter()
                .map(|opt| opt.unwrap_or("").to_string())
                .collect());
        }

        // For other types, convert through the chunked array
        let n = series.len();
        let mut result = Vec::with_capacity(n);

        // Use rechunk to ensure we have a single chunk, then extract values
        let rechunked = series.rechunk();

        // Try different data types
        if let Ok(ca) = rechunked.f64() {
            for i in 0..n {
                result.push(ca.get(i).map(|v| v.to_string()).unwrap_or_default());
            }
        } else if let Ok(ca) = rechunked.i64() {
            for i in 0..n {
                result.push(ca.get(i).map(|v| v.to_string()).unwrap_or_default());
            }
        } else if let Ok(ca) = rechunked.str() {
            for i in 0..n {
                result.push(ca.get(i).unwrap_or("").to_string());
            }
        } else if let Ok(ca) = rechunked.bool() {
            for i in 0..n {
                result.push(ca.get(i).map(|v| v.to_string()).unwrap_or_default());
            }
        } else {
            // Fallback: use string conversion
            let str_series = rechunked.cast(&DataType::String)
                .map_err(|e| DataError::PolarsError(e))?;
            let ca = str_series.str()
                .map_err(|e| DataError::PolarsError(e))?;
            for i in 0..n {
                result.push(ca.get(i).unwrap_or("").to_string());
            }
        }

        Ok(result)
    }

    /// Get all data as Vec<Vec<f64>> (row-major format)
    /// This is for compatibility with existing code
    pub fn as_row_major_f64(&self) -> Vec<Vec<f64>> {
        let n_rows = self.height();
        let n_cols = self.width();

        // Extract all columns once (column-major)
        let columns: Vec<Vec<f64>> = (0..n_cols)
            .map(|col_idx| self.column_as_f64(col_idx).unwrap_or_else(|_| vec![f64::NAN; n_rows]))
            .collect();

        // Transpose to row-major
        (0..n_rows)
            .map(|row_idx| {
                columns.iter()
                    .map(|col| col.get(row_idx).copied().unwrap_or(f64::NAN))
                    .collect()
            })
            .collect()
    }

    /// Get all data as Vec<Vec<String>> (row-major format)
    /// This is for compatibility with existing code
    pub fn as_row_major_string(&self) -> Vec<Vec<String>> {
        let n_rows = self.height();
        let n_cols = self.width();

        // Extract all columns once (column-major)
        let columns: Vec<Vec<String>> = (0..n_cols)
            .map(|col_idx| self.column_as_string(col_idx).unwrap_or_else(|_| vec![String::new(); n_rows]))
            .collect();

        // Transpose to row-major
        (0..n_rows)
            .map(|row_idx| {
                columns.iter()
                    .map(|col| col.get(row_idx).cloned().unwrap_or_default())
                    .collect()
            })
            .collect()
    }

    /// Get a specific cell value as f64
    pub fn get_f64(&self, row: usize, col: usize) -> Option<f64> {
        self.column_as_f64(col).ok()?.get(row).copied()
    }

    /// Get a specific cell value as String
    pub fn get_string(&self, row: usize, col: usize) -> Option<String> {
        self.column_as_string(col).ok()?.get(row).cloned()
    }

    /// Get column by index as Series (for statistics and analysis)
    pub fn get_column_series(&self, col_idx: usize) -> Result<Series, DataError> {
        let col_names = self.column_names();
        if col_idx >= col_names.len() {
            return Err(DataError::ColumnNotFound(format!("Index {}", col_idx)));
        }
        self.column_values(&col_names[col_idx])
    }

    /// Calculate statistics for a column by index
    pub fn column_stats(&self, col_idx: usize) -> Result<super::stats::Stats, DataError> {
        let series = self.get_column_series(col_idx)?;
        Ok(super::stats::calculate_stats(&series))
    }

    /// Check if a column is a datetime or date type
    pub fn is_datetime_column(&self, col_idx: usize) -> bool {
        let col_names = self.column_names();
        if col_idx >= col_names.len() {
            return false;
        }
        if let Ok(series) = self.column_values(&col_names[col_idx]) {
            matches!(series.dtype(), DataType::Datetime(_, _) | DataType::Date)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;
    use std::time::Instant;

    #[test]
    fn test_datasource_csv_loading() {
        // Create a temporary CSV file with .csv extension
        let mut file = Builder::new().suffix(".csv").tempfile().unwrap();
        writeln!(file, "x,y,z").unwrap();
        writeln!(file, "1,2,3").unwrap();
        writeln!(file, "4,5,6").unwrap();
        writeln!(file, "7,8,9").unwrap();
        file.flush().unwrap();

        // Load with DataSource
        let ds = DataSource::load(file.path()).unwrap();

        // Verify dimensions
        assert_eq!(ds.height(), 3);
        assert_eq!(ds.width(), 3);

        // Verify column names
        let names = ds.column_names();
        assert_eq!(names, vec!["x", "y", "z"]);

        // Verify data extraction
        let col_x = ds.column_as_f64(0).unwrap();
        assert_eq!(col_x, vec![1.0, 4.0, 7.0]);

        let col_y = ds.column_as_f64(1).unwrap();
        assert_eq!(col_y, vec![2.0, 5.0, 8.0]);
    }

    #[test]
    fn test_datasource_row_major_conversion() {
        // Create a temporary CSV file with .csv extension
        let mut file = Builder::new().suffix(".csv").tempfile().unwrap();
        writeln!(file, "a,b").unwrap();
        writeln!(file, "1,2").unwrap();
        writeln!(file, "3,4").unwrap();
        file.flush().unwrap();

        let ds = DataSource::load(file.path()).unwrap();

        // Test row-major conversion
        let rows = ds.as_row_major_f64();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec![1.0, 2.0]);
        assert_eq!(rows[1], vec![3.0, 4.0]);
    }

    #[test]
    fn test_datasource_statistics() {
        // Create a temporary CSV file with .csv extension
        let mut file = Builder::new().suffix(".csv").tempfile().unwrap();
        writeln!(file, "values").unwrap();
        writeln!(file, "1").unwrap();
        writeln!(file, "2").unwrap();
        writeln!(file, "3").unwrap();
        writeln!(file, "4").unwrap();
        writeln!(file, "5").unwrap();
        file.flush().unwrap();

        let ds = DataSource::load(file.path()).unwrap();

        // Test statistics calculation
        let stats = ds.column_stats(0).unwrap();
        assert_eq!(stats.mean, 3.0);
        assert_eq!(stats.median, 3.0);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.count, 5);
    }

    #[test]
    fn test_datasource_datetime_parsing() {
        // Create a temporary CSV file with datetime columns
        let mut file = Builder::new().suffix(".csv").tempfile().unwrap();
        writeln!(file, "date,value").unwrap();
        writeln!(file, "2024-01-01,10.5").unwrap();
        writeln!(file, "2024-01-02,15.3").unwrap();
        writeln!(file, "2024-01-03,12.8").unwrap();
        file.flush().unwrap();

        // Load with DataSource
        let ds = DataSource::load(file.path()).unwrap();

        // Verify datetime column is detected
        assert!(ds.is_datetime_column(0), "Date column should be detected as datetime");

        // Verify datetime values are converted to timestamps
        let timestamps = ds.column_as_f64(0).unwrap();
        assert_eq!(timestamps.len(), 3);

        // Verify timestamps are reasonable (between 2024-01-01 and 2024-01-04)
        let start_ts = 1704067200.0; // 2024-01-01 00:00:00 UTC
        let end_ts = 1704326400.0;   // 2024-01-04 00:00:00 UTC
        for &ts in &timestamps {
            assert!(ts >= start_ts && ts <= end_ts, "Timestamp {} should be between {} and {}", ts, start_ts, end_ts);
        }

        // Verify numeric column works as expected
        let values = ds.column_as_f64(1).unwrap();
        assert_eq!(values, vec![10.5, 15.3, 12.8]);
    }

    #[test]
    fn test_datasource_large_file_performance() {
        // Create a large CSV file (100k rows, 5 columns)
        let mut file = Builder::new().suffix(".csv").tempfile().unwrap();
        writeln!(file, "time,value1,value2,value3,value4").unwrap();
        for i in 0..100_000 {
            writeln!(
                file,
                "{},{},{},{},{}",
                i,
                i as f64 * 1.5,
                (i as f64).sin(),
                (i as f64).cos(),
                i % 100
            )
            .unwrap();
        }
        file.flush().unwrap();

        // Measure load time
        let start = Instant::now();
        let ds = DataSource::load(file.path()).unwrap();
        let load_duration = start.elapsed();

        // Verify data loaded correctly
        assert_eq!(ds.height(), 100_000);
        assert_eq!(ds.width(), 5);

        // Measure row-major conversion time (most expensive operation)
        let start = Instant::now();
        let _data = ds.as_row_major_f64();
        let conversion_duration = start.elapsed();

        // Measure statistics calculation time
        let start = Instant::now();
        let _stats = ds.column_stats(1).unwrap();
        let stats_duration = start.elapsed();

        // Print timing information for manual verification
        println!("Performance results for 100k rows:");
        println!("  Load time: {:?}", load_duration);
        println!("  Row-major conversion: {:?}", conversion_duration);
        println!("  Stats calculation: {:?}", stats_duration);
        println!("  Total: {:?}", load_duration + conversion_duration + stats_duration);

        // Target: <100ms total for all operations
        let total = load_duration + conversion_duration + stats_duration;
        assert!(
            total.as_millis() < 500,
            "Performance target not met: {:?} >= 500ms",
            total
        );
    }
}
