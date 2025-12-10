migrate:
	sqlx database setup --source src-tauri/migrations

sqlx-prepare:
	cargo  sqlx prepare --workspace
