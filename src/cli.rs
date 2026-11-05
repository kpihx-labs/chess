#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opponent {
    Computer,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    White,
    Black,
    Random,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub opponent: Opponent,
    pub elo: u16,
    pub side: Side,
    pub minutes: u64,
    pub increment: u64,
}

pub enum Action {
    Run(Options),
    Print(String),
}

pub fn parse(args: impl Iterator<Item = String>) -> Result<Action, String> {
    let mut options = Options {
        opponent: Opponent::Computer,
        elo: 1600,
        side: Side::White,
        minutes: 10,
        increment: 0,
    };
    let mut args = args.peekable();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(Action::Print(help())),
            "-V" | "--version" => {
                return Ok(Action::Print(format!(
                    "chess {}",
                    env!("CARGO_PKG_VERSION")
                )));
            }
            "--vs" => {
                options.opponent = match value(&mut args, "--vs")?.as_str() {
                    "computer" => Opponent::Computer,
                    "human" => Opponent::Human,
                    _ => return Err("--vs must be computer or human".into()),
                }
            }
            "--elo" => {
                options.elo = value(&mut args, "--elo")?
                    .parse()
                    .map_err(|_| "invalid Elo")?;
                if !(600..=2600).contains(&options.elo) {
                    return Err("--elo must be between 600 and 2600".into());
                }
            }
            "--side" => {
                options.side = match value(&mut args, "--side")?.as_str() {
                    "white" => Side::White,
                    "black" => Side::Black,
                    "random" => Side::Random,
                    _ => return Err("--side must be white, black, or random".into()),
                }
            }
            "--time" => {
                let time = value(&mut args, "--time")?;
                let (minutes, increment) = time.split_once('+').unwrap_or((&time, "0"));
                options.minutes = minutes.parse().map_err(|_| "invalid minutes")?;
                options.increment = increment.parse().map_err(|_| "invalid increment")?;
                if options.minutes == 0 || options.minutes > 180 || options.increment > 60 {
                    return Err("time must be 1..180 minutes with 0..60 seconds increment".into());
                }
            }
            unknown => return Err(format!("unknown option: {unknown}")),
        }
    }
    Ok(Action::Run(options))
}

fn value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

pub fn help() -> String {
    format!(
        "chess {}\nHigh-quality terminal chess powered by a native Rust engine.\n\nUSAGE:\n  chess [OPTIONS]\n\nOPTIONS:\n      --vs <OPPONENT>  computer or human [default: computer]\n      --elo <RATING>    Target engine strength 600..2600 [default: 1600]\n      --side <COLOR>    white, black, or random [default: white]\n      --time <M+I>      Minutes plus increment seconds [default: 10+0]\n  -h, --help            Print help\n  -V, --version         Print version\n\nEXAMPLES:\n  chess\n  chess --elo 2200 --side black --time 15+10\n  chess --vs human --time 5+3",
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_engine_game() {
        let args = ["--elo", "2200", "--side", "black", "--time", "15+10"];
        let Action::Run(options) = parse(args.into_iter().map(str::to_string)).expect("options")
        else {
            panic!("run")
        };
        assert_eq!(options.elo, 2200);
        assert_eq!((options.minutes, options.increment), (15, 10));
        assert_eq!(options.side, Side::Black);
    }
    #[test]
    fn rejects_fake_elo() {
        let args = ["--elo", "3000"];
        assert!(parse(args.into_iter().map(str::to_string)).is_err());
    }
}
