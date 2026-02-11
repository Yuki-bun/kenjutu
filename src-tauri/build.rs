fn main() {
    let _ = dotenvy::dotenv();

    // tell rust to recompile when .env changes
    println!("cargo:rerun-if-env-changed=GH_APP_CLIENT_SECRET");

    if let Ok(val) = std::env::var("GH_APP_CLIENT_SECRET") {
        println!("cargo:rustc-env=GH_APP_CLIENT_SECRET={}", val);
    }

    tauri_build::build()
}
