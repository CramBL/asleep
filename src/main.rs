fn main() {
    asleep::signal::install_handler();
    let config = asleep::args::parse();
    let ec = asleep::sleep::run_sleep(config);
    std::process::exit(ec);
}
