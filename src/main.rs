use chess::{cli, ui::App};

fn main() {
    match cli::parse(std::env::args().skip(1)) {
        Ok(cli::Action::Run(options)) => {
            if let Err(error) = App::new(options).run() {
                eprintln!("chess: {error}");
                std::process::exit(1);
            }
        }
        Ok(cli::Action::Print(text)) => println!("{text}"),
        Err(error) => {
            eprintln!("chess: {error}\n\n{}", cli::help());
            std::process::exit(2);
        }
    }
}
