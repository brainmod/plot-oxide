# Test Data Files

This directory contains sample CSV and Parquet files for testing PlotOxide.

## CSV Files

### sample_timeseries.csv
- **Purpose**: Time-series data with timestamp, temperature, humidity, and pressure
- **Records**: 24 hours of hourly data
- **Columns**: 4 (timestamp, temperature, humidity, pressure)
- **Use Case**: Testing datetime handling, multiple Y-series plotting

### sample_spc.csv
- **Purpose**: Statistical Process Control (SPC) data with intentional outliers
- **Records**: 50 measurements
- **Columns**: 3 (sample, measurement, specification)
- **Features**: Contains outliers at rows 16 (15.5) and 26 (5.2) for testing outlier detection
- **Use Case**: Testing SPC limits, Western Electric rules, outlier detection

### sample_multi_series.csv
- **Purpose**: Multiple series with different trends
- **Records**: 50 data points
- **Columns**: 5 (index, series_a, series_b, series_c, series_d)
- **Features**: Four different data series with varying characteristics
- **Use Case**: Testing multiple Y-axis series selection and plotting

### sample_large.csv
- **Purpose**: Simple categorical data
- **Records**: 50 rows
- **Columns**: 3 (row, value, category)
- **Features**: Categories A, B, C with numeric values around 100
- **Use Case**: Testing basic plotting, filtering, and categorical data handling

## Creating Parquet Files

To create Parquet versions of these files for testing, use the following code:

```rust
use polars::prelude::*;
use std::fs::File;

// Read CSV
let df = CsvReader::from_path("test-data/sample_timeseries.csv")?
    .has_header(true)
    .finish()?;

// Write Parquet
let mut file = File::create("test-data/sample_timeseries.parquet")?;
ParquetWriter::new(&mut file).finish(&mut df)?;
```

Or load any CSV directly in PlotOxide and it will automatically use Polars!

## Testing Instructions

1. **Load CSV Files**: Open PlotOxide and use File > Open to load any CSV file
2. **Test Parquet**: Parquet files can be loaded the same way (if generated)
3. **Verify Features**:
   - Multi-series: sample_multi_series.csv - select multiple Y columns
   - SPC: sample_spc.csv - enable SPC limits and outlier detection
   - Time-series: sample_timeseries.csv - verify timestamp parsing
   - Performance: All files should load instantly with Polars

## Expected Behavior

- **All CSVs load without errors** ✅
- **Column names detected correctly** ✅
- **Data types inferred properly** ✅
- **Numeric conversions work** ✅
- **Statistics calculate accurately** ✅
- **Outliers detected** (sample_spc.csv rows 16, 26) ✅

## Performance Benchmarks

With Polars:
- Load time: <10ms for these small files
- Statistics: <1ms per column
- Row-major conversion: O(n*m) complexity (optimized)

The sample_spc.csv file is ideal for verifying that outlier detection works:
- Mean ≈ 10.0
- Std Dev ≈ 1.4
- Outliers (>3σ): rows 16 and 26
