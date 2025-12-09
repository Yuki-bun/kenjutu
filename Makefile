db_url = sqlite:src-tauri/db.pr

migrate:
	sqlx database setup --source src-tauri/migrations

sqlx-prepare:
	cargo  sqlx prepare --workspace
