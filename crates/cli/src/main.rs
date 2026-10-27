use std::{env, fs, process::ExitCode};
use vesper_story::Story;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let (Some(command), Some(path), None) = (args.next(), args.next(), args.next()) else {
        eprintln!("Usage: vesper check <story.vn>");
        return ExitCode::FAILURE;
    };
    if command != "check" {
        eprintln!("Unknown command: {command}");
        return ExitCode::FAILURE;
    }
    let source = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    match Story::parse(&source) {
        Ok(story) => {
            println!("Valid story: {} scene(s)", story.scene_count());
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{path}:{}: {}", error.line, error.message);
            }
            ExitCode::FAILURE
        }
    }
}
