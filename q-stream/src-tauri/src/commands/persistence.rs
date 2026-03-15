use crate::models::PersistentAppData;
use crate::persistence;

/// Load all persistent user data (recently played, dismissed albums, search history).
/// Called once on app startup.
#[tauri::command]
pub async fn load_app_data() -> Result<PersistentAppData, String> {
    Ok(persistence::load())
}

/// Persist all user data to disk. Called automatically by the frontend after
/// any state change, debounced to avoid excessive writes.
#[tauri::command]
pub async fn save_app_data(data: PersistentAppData) -> Result<(), String> {
    persistence::save(&data);
    Ok(())
}
