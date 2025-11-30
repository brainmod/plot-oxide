//! Background worker for async file loading and LTTB computation (Phase 5)
//!
//! Keeps UI at 60fps regardless of computation load.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::sync::Arc;

use polars::prelude::*;

/// Requests that can be sent to the background worker
pub enum WorkerRequest {
    /// Load a file (CSV or Parquet)
    LoadFile { path: PathBuf },
    /// Compute LTTB downsampling
    ComputeLttb {
        series_id: usize,
        data: Arc<[(f64, f64)]>,
        target_points: usize,
    },
    /// Shutdown the worker
    Shutdown,
}

/// Results returned from the background worker
pub enum WorkerResult {
    /// File loaded successfully
    FileLoaded { path: PathBuf, df: DataFrame },
    /// LTTB computation complete
    LttbReady { series_id: usize, points: Vec<[f64; 2]> },
    /// An error occurred
    Error { msg: String },
}

/// Background worker that processes requests off the main thread
pub struct BackgroundWorker {
    tx: Sender<WorkerRequest>,
    rx: Receiver<WorkerResult>,
    handle: Option<JoinHandle<()>>,
}

impl BackgroundWorker {
    /// Spawn a new background worker thread
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = channel::<WorkerRequest>();
        let (res_tx, res_rx) = channel::<WorkerResult>();
        
        let handle = thread::spawn(move || {
            Self::worker_loop(req_rx, res_tx);
        });
        
        Self {
            tx: req_tx,
            rx: res_rx,
            handle: Some(handle),
        }
    }
    
    fn worker_loop(rx: Receiver<WorkerRequest>, tx: Sender<WorkerResult>) {
        while let Ok(request) = rx.recv() {
            let result = match request {
                WorkerRequest::LoadFile { path } => {
                    Self::load_file(&path)
                }
                WorkerRequest::ComputeLttb { series_id, data, target_points } => {
                    let points = Self::compute_lttb(&data, target_points);
                    WorkerResult::LttbReady { series_id, points }
                }
                WorkerRequest::Shutdown => break,
            };
            
            if tx.send(result).is_err() {
                break;
            }
        }
    }
    
    fn load_file(path: &PathBuf) -> WorkerResult {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        let df_result = match extension.to_lowercase().as_str() {
            "parquet" => {
                // Phase 1.1: Use optimized parquet loading
                LazyFrame::scan_parquet(path, ScanArgsParquet {
                    low_memory: true,
                    parallel: polars::io::parquet::read::ParallelStrategy::Auto,
                    ..Default::default()
                })
                .and_then(|lf| lf.collect())
            }
            "csv" => {
                LazyCsvReader::new(path)
                    .with_has_header(true)
                    .with_infer_schema_length(Some(100))
                    .with_try_parse_dates(true)
                    .finish()
                    .and_then(|lf| lf.collect())
            }
            _ => {
                return WorkerResult::Error {
                    msg: format!("Unsupported format: {}", extension),
                };
            }
        };
        
        match df_result {
            Ok(df) => WorkerResult::FileLoaded { path: path.clone(), df },
            Err(e) => WorkerResult::Error { msg: e.to_string() },
        }
    }
    
    fn compute_lttb(data: &[(f64, f64)], target: usize) -> Vec<[f64; 2]> {
        if data.len() <= target {
            return data.iter().map(|&(x, y)| [x, y]).collect();
        }
        
        let mut result = Vec::with_capacity(target);
        
        // Always include first point
        result.push([data[0].0, data[0].1]);
        
        let bucket_size = (data.len() - 2) as f64 / (target - 2) as f64;
        
        let mut a = 0usize;
        
        for i in 0..(target - 2) {
            // Calculate bucket range
            let bucket_start = ((i as f64 + 1.0) * bucket_size).floor() as usize + 1;
            let bucket_end = (((i as f64 + 2.0) * bucket_size).floor() as usize + 1).min(data.len() - 1);
            
            // Calculate average of next bucket for reference
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
                // Triangle area formula (simplified, sign doesn't matter)
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
    
    /// Send a request to the worker (non-blocking)
    pub fn request(&self, req: WorkerRequest) {
        let _ = self.tx.send(req);
    }
    
    /// Poll for completed work (non-blocking)
    pub fn poll(&self) -> Option<WorkerResult> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }
    
    /// Check if there's pending work
    pub fn is_busy(&self) -> bool {
        // Simple heuristic - could be improved with atomic counter
        false
    }
}

impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(WorkerRequest::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for BackgroundWorker {
    fn default() -> Self {
        Self::spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    #[test]
    fn test_lttb_basic() {
        let data: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, (i as f64).sin())).collect();
        let result = BackgroundWorker::compute_lttb(&data, 100);
        
        assert_eq!(result.len(), 100);
        assert_eq!(result[0][0], 0.0);
        assert_eq!(result[99][0], 999.0);
    }
    
    #[test]
    fn test_worker_file_load() {
        let mut file = Builder::new().suffix(".csv").tempfile().unwrap();
        writeln!(file, "x,y").unwrap();
        writeln!(file, "1,2").unwrap();
        writeln!(file, "3,4").unwrap();
        file.flush().unwrap();
        
        let worker = BackgroundWorker::spawn();
        worker.request(WorkerRequest::LoadFile { path: file.path().to_path_buf() });
        
        // Wait for result
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        if let Some(WorkerResult::FileLoaded { df, .. }) = worker.poll() {
            assert_eq!(df.height(), 2);
        } else {
            panic!("Expected FileLoaded result");
        }
    }
}
