fn main() {
    let _ = dotenvy::dotenv();

    // tell rust to recompile when .env changes
    println!("cargo:rerun-if-changed=.env");

    if let Ok(val) = std::env::var("GITHUB_APP_CLIENT_SECRET") {
        println!("cargo:rustc-env=GITHUB_APP_CLIENT_SECRET={}", val);
    }

    tauri_build::build()
}
