pub mod commands;
pub mod feed;
pub mod media_controls;
pub mod player;
pub mod settings;
pub mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    #[test]
    fn harness_works() {
        assert_eq!(1 + 1, 2);
    }
}
