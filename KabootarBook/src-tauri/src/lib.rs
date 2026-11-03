use tauri_plugin_sql::{Migration, MigrationKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![Migration {
        version: 1,
        description: "create_initial_schema",
        sql: include_str!("../migrations/001_init.sql"),
        kind: MigrationKind::Up,
    }];
    let builder = tauri::Builder::default();
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_safe_area_insets::init());
    builder
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:kabootarbook.db", migrations)
                .build(),
        )
        .run(tauri::generate_context!())
        .expect("error while running KabootarBook");
}
