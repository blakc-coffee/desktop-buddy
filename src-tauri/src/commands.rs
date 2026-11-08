#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]

pub enum Destination {
    Activity,
    Codex,
    Fill,
    ScreenCapture,
    ScreenExplain,
    Ask,
    Resume,
    Projects,
    Shelf,
    Timers,
    Reminders,
    Settings,
    Music,
}

// Shared, explicit command contract for the native launcher. No shell evaluation.

#[derive(Debug, Clone, PartialEq)]

pub enum Command {
    Help,

    Tools,

    Panel(Destination),

    Todos,

    Held,

    Imports,

    Copy,

    Pets,

    Find(String),

    Workspaces(String),

    Timer { minutes: u32, title: String },

    Reminder { minutes: u32, title: String },

    Todo(String),

    Question(String),
    Screen { question: String, fill: bool },
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

pub fn definitions() -> [(&'static [&'static str], &'static str, &'static str); 10] {
    [
        (&["/sc", "/screen", "/fill"], "/sc question · Capture and fill", "/sc "),
        (&["/exp", "/explain"], "/exp question · Explain screen", "/exp "),
        (&["/codex"], "/codex · Recent chats", "/codex"),
        (&["/t", "/timer", "/focus"], "/t 25m task · Timer", "/t "),
        (&["/r", "/remind"], "/r 10m message · Reminder", "/r "),
        (&["/d", "/todo"], "/d task · Add todo", "/d "),
        (&["/todos"], "/todos · Checklist", "/todos"),
        (&["/f", "/find"], "/f name · Find files", "/f "),
        (&["/w", "/workspace"], "/w name · Open workspace", "/w "),
        (&["/tools", "/settings"], "/tools · Manage tools and settings", "/tools"),
    ]
}

pub fn parse(text: &str) -> Result<Command, String> {
    let text = text.trim();

    if text.len() > 1024 {
        return Err("Command is too long.".into());
    }

    if !text.is_empty() && !text.starts_with('/') { return Ok(Command::Question(text.into())); }

    let (verb, args) = text.split_once(char::is_whitespace).unwrap_or((text, ""));

    let args = args.trim();

    let verb = verb.to_ascii_lowercase();

    match verb.as_str() {
        "" | "/" | "/help" => Ok(Command::Help),

        "/tools" | "/actions" if args.is_empty() => Ok(Command::Tools),

        "/sc" | "/screen" | "/fill" => Ok(Command::Screen { question: args.into(), fill: true }),
        "/exp" | "/explain" => Ok(Command::Screen { question: args.into(), fill: false }),
        "/codex" if args.is_empty() => Ok(Command::Panel(Destination::Codex)),

        "/ask" if args.is_empty() => Ok(Command::Panel(Destination::Ask)),

        "/resume" if args.is_empty() => Ok(Command::Panel(Destination::Resume)),

        "/projects" if args.is_empty() => Ok(Command::Panel(Destination::Projects)),

        "/shelf" if args.is_empty() => Ok(Command::Panel(Destination::Shelf)),

        "/timers" if args.is_empty() => Ok(Command::Panel(Destination::Timers)),

        "/reminders" if args.is_empty() => Ok(Command::Panel(Destination::Reminders)),

        "/settings" if args.is_empty() => Ok(Command::Panel(Destination::Settings)),

        "/music" if args.is_empty() => Ok(Command::Panel(Destination::Music)),

        "/todos" if args.is_empty() => Ok(Command::Todos),

        "/held" if args.is_empty() => Ok(Command::Held),

        "/imports" if args.is_empty() => Ok(Command::Imports),

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
        assert_eq!(suggestions("").len(), 10);

        assert_eq!(parse("/resume"), Ok(Command::Panel(Destination::Resume)));

        assert!(suggestions("/copy").is_empty());

        assert_eq!(parse("/sc"), Ok(Command::Screen { question: String::new(), fill: true }));
        assert_eq!(parse("/exp"), Ok(Command::Screen { question: String::new(), fill: false }));
        assert_eq!(parse("/codex"), Ok(Command::Panel(Destination::Codex)));

        assert_eq!(suggestions("/too").len(), 1);

        assert_eq!(parse("/pet"), Ok(Command::Pets));

        assert_eq!(suggestions("/ti").len(), 1);

        assert!(suggestions("/t ").is_empty());

        assert!(suggestions("/t 15m review").is_empty());

        assert!(suggestions("ordinary text").is_empty());
    }

    #[test]

    fn all_catalogue_destinations_are_reachable_and_s_is_not_a_command() {
        for (_, _, completion) in definitions() {
            if !completion.ends_with(' ') {
                assert!(parse(completion).is_ok(), "{completion}");
            }
        }
        assert_eq!(parse("/ask"), Ok(Command::Panel(Destination::Ask)));
        assert_eq!(parse("/todos"), Ok(Command::Todos));
        assert_eq!(parse("/held"), Ok(Command::Held));
        assert!(parse("/s").is_err());
        assert_eq!(parse("/exp why this error?"), Ok(Command::Screen { question: "why this error?".into(), fill: false }));
        assert!(parse("/ask extra").is_err());
    }
    #[test]
    fn plain_questions_never_become_local_actions_and_slash_commands_stay_explicit() {
        assert_eq!(parse("delete files"), Ok(Command::Question("delete files".into())));
        assert_eq!(parse("What is a binary tree?"), Ok(Command::Question("What is a binary tree?".into())));
        assert_eq!(parse("/d buy milk"), Ok(Command::Todo("buy milk".into())));
        assert!(parse("/unknown").is_err());

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
