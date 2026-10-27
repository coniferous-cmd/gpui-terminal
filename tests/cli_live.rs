#[cfg(unix)]
#[path = "cli_live/interface.rs"]
mod interface;

#[cfg(unix)]
fn main() {
    if std::env::var_os("LUNA_CLI_LIVE").is_none() {
        println!("cli_live: skipped; set LUNA_CLI_LIVE=1 with a desktop to run");
        return;
    }
    interface::run();
}

#[cfg(not(unix))]
fn main() {
    println!("cli_live: Unix desktop harness; Windows requires native release verification");
}
