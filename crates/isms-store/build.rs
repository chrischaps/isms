// `sqlx::migrate!()` embeds the migrations directory at macro-expansion time and
// cannot watch for a file that did not exist then; without this, a new migration
// leaves an incremental build with a stale migrator (the check worktree bit).
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
