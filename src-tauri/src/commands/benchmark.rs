use crate::benchmark::engine::{BenchmarkComparison, BenchmarkRequest};
use crate::commands::network::AppState;
use crate::storage::db::DbBenchmarkRecord;
use tauri::State;

#[tauri::command]
pub async fn run_benchmark(
    state: State<'_, AppState>,
    scenario: String,
    target_url: Option<String>,
) -> Result<DbBenchmarkRecord, String> {
    let runner = state.benchmark_runner.clone();
    let req = BenchmarkRequest {
        scenario,
        target_url,
    };
    runner.run_benchmark(req).await
}

#[tauri::command]
pub fn get_benchmark_history(state: State<'_, AppState>) -> Result<Vec<DbBenchmarkRecord>, String> {
    state
        .db
        .get_benchmark_history()
        .map_err(|e| format!("Failed to read benchmark history: {}", e))
}

#[tauri::command]
pub fn clear_benchmark_history(state: State<'_, AppState>) -> Result<(), String> {
    state
        .db
        .clear_benchmark_history()
        .map_err(|e| format!("Failed to clear benchmark history: {}", e))
}

#[tauri::command]
pub fn get_benchmark_comparison(
    state: State<'_, AppState>,
    current_id: String,
) -> Result<BenchmarkComparison, String> {
    state.benchmark_runner.get_comparison(&current_id)
}
