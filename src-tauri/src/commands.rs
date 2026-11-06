//! Shared, explicit command contract for the native launcher. No shell evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    Tools,
    Copy,
    Pets,
    Find(String),
    Workspaces(String),
    Timer { minutes: u32, title: String },
    Reminder { minutes: u32, title: String },
    Todo(String),
}

pub fn suggestions(text: &str) -> Vec<(&'static str, &'static str)> {
    let prefix = text.trim_start().to_ascii_lowercase();
    if !prefix.is_empty() && (!prefix.starts_with('/') || prefix.chars().any(char::is_whitespace)) {
        return Vec::new();
    }
    definitions()
        .into_iter()
        .filter(|(aliases, _, _)| aliases.iter().any(|alias| alias.starts_with(&prefix)))
        .map(|(_, hint, completion)| (hint, completion))
        .collect()
}

pub fn definitions() -> [(&'static [&'static str], &'static str, &'static str); 7] {
    [
        (&["/t", "/timer", "/focus"][..], "/t 25m · Timer", "/t "),
        (&["/f", "/find"][..], "/f name · Find files and folders", "/f "),
        (&["/w", "/workspace"][..], "/w name · Workspace", "/w "),
        (&["/d", "/todo"][..], "/d task · To-do", "/d "),
        (&["/r", "/remind"][..], "/r 10m · Reminder", "/r "),
        (&["/codex"][..], "/codex · Open Codex tasks", "/codex"),
        (&["/pet"][..], "/pet · Choose pet", "/pet"),
    ]
}
pub fn parse(text: &str) -> Result<Command, String> {
    let text = text.trim();
    if text.len() > 1024 {
        return Err("Command is too long.".into());
    }
    let (verb, args) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    let args = args.trim();
    let verb = verb.to_ascii_lowercase();
    match verb.as_str() {
        "" | "/" | "/help" => Ok(Command::Help),
        "/tools" | "/settings" | "/codex" if args.is_empty() => Ok(Command::Tools),
        "/tools" | "/settings" => Ok(Command::Find(args.into())),
        "/copy" => Ok(Command::Copy),
        "/pet" if args.is_empty() => Ok(Command::Pets),
        "/f" | "/find" => Ok(Command::Find(args.into())),
        "/w" | "/workspace" => Ok(Command::Workspaces(args.into())),
        "/d" | "/todo" if !args.is_empty() && args.len() <= 240 => Ok(Command::Todo(args.into())),
        "/t" | "/timer" | "/focus" | "/r" | "/remind" => {
            let (duration, title) = args.split_once(char::is_whitespace).unwrap_or((args, ""));
            let minutes = duration
                .strip_suffix('m')
                .unwrap_or(duration)
                .parse::<u32>()
                .map_err(|_| "Use /t 25m task or /r 10m reminder.")?;
            if !(1..=1440).contains(&minutes) || title.len() > 240 {
                return Err("Choose 1–1440 minutes and a short title.".into());
            }
            if verb == "/r" || verb == "/remind" {
                Ok(Command::Reminder {
                    minutes,
                    title: if title.is_empty() {
                        "Reminder".into()
                    } else {
                        title.trim().into()
                    },
                })
            } else {
                Ok(Command::Timer {
                    minutes,
                    title: if title.is_empty() {
                        "Focus session".into()
                    } else {
                        title.trim().into()
                    },
                })
            }
        }
        _ => {
            Err("Choose /t timer, /d todo, /r reminder, /f folder, /w workspace, or /tools.".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn autocomplete_browses_empty_input_and_leaves_arguments_alone() {
        assert_eq!(suggestions("").len(), 7);
        assert!(suggestions("/copy").is_empty());
        assert_eq!(parse("/codex"), Ok(Command::Tools));
        assert!(suggestions("/too").is_empty());
        assert_eq!(parse("/pet"), Ok(Command::Pets));
        assert_eq!(suggestions("/ti"), vec![("/t 25m · Timer", "/t ")]);
        assert!(suggestions("/t ").is_empty());
        assert!(suggestions("/t 15m review").is_empty());
        assert!(suggestions("ordinary text").is_empty());
    }
    #[test]
    fn only_explicit_commands_run() {
        assert!(parse("delete files").is_err());
        assert!(parse("/t 25m task; calc.exe").is_ok());
        assert_eq!(
            parse("/w demo").unwrap(),
            Command::Workspaces("demo".into())
        );
    }
    #[test]
    fn durations_and_titles_are_bounded() {
        assert!(parse("/t 0m").is_err());
        assert!(parse("/t 1441m").is_err());
        assert!(parse("/t 2h").is_err());
        assert!(parse("/d").is_err());
        assert_eq!(
            parse("/r 2m tea").unwrap(),
            Command::Reminder {
                minutes: 2,
                title: "tea".into()
            }
        );
    }
}
