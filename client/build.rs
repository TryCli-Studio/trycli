fn main() {
    // Load the .env file from the root
    let dotenv_path = std::env::current_dir().unwrap().join("../.env");
    dotenv::from_path(&dotenv_path).ok();

    // Tell Cargo to re-run this script if .env changes
    println!("cargo:rerun-if-changed=../.env");

    // Pass the CONTAINER_ID to the actual code
    if let Ok(id) = std::env::var("CONTAINER_ID") {
        println!("cargo:rustc-env=CONTAINER_ID={}", id);
    } else {
        println!("cargo:warning=CONTAINER_ID not found in .env");
    }
}