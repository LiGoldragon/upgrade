fn main() {
    match upgrade::daemon_placeholder_response(std::env::args()) {
        Ok(reply) => println!("{reply}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
