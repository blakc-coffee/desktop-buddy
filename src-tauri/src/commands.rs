#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]

pub enum Destination {
    Activity,
    Codex,
    Fill,
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

pub fn definitions() -> [(&'static [&'static str], &'static str, &'static str); 20] {
    [
        (
            &["/fill"][..],
            "/fill \u{b7} Fill current browser page",
            "/fill",
        ),
        (&["/t", "/timer", "/focus"][..], "/t 25m · Timer", "/t "),
        (
            &["/f", "/find"][..],
            "/f name · Find files and folders",
            "/f ",
        ),
        (&["/w", "/workspace"][..], "/w name · Workspace", "/w "),
        (&["/d", "/todo"][..], "/d task · To-do", "/d "),
        (&["/r", "/remind"][..], "/r 10m · Reminder", "/r "),
        (&["/codex"][..], "/codex · Open Codex tasks", "/codex"),
        (&["/resume"][..], "/resume · Pick up a project", "/resume"),
        (&["/ask"][..], "/ask \u{b7} Ask about files", "/ask"),
        (&["/todos"][..], "/todos \u{b7} Todo checklist", "/todos"),
        (&["/timers"][..], "/timers \u{b7} Manage timers", "/timers"),
        (
            &["/reminders"][..],
            "/reminders \u{b7} Manage reminders",
            "/reminders",
        ),
        (
            &["/projects"][..],
            "/projects \u{b7} Saved projects",
            "/projects",
        ),
        (&["/shelf"][..], "/shelf \u{b7} Pinned references", "/shelf"),
        (&["/held"][..], "/held \u{b7} Files in the pouch", "/held"),
        (
            &["/imports"][..],
            "/imports \u{b7} Open imported files folder",
            "/imports",
        ),
        (
            &["/settings"][..],
            "/settings \u{b7} Settings and backups",
            "/settings",
        ),
        (&["/music"][..], "/music \u{b7} Music tools", "/music"),
        (
            &["/actions"][..],
            "/actions \u{b7} Open tools panel",
            "/actions",
        ),
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

        "/tools" | "/actions" if args.is_empty() => Ok(Command::Tools),

        "/fill" if args.is_empty() => Ok(Command::Panel(Destination::Fill)),
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
        assert_eq!(suggestions("").len(), 20);

        assert_eq!(parse("/resume"), Ok(Command::Panel(Destination::Resume)));

        assert!(suggestions("/copy").is_empty());

        assert_eq!(parse("/codex"), Ok(Command::Panel(Destination::Codex)));

        assert_eq!(suggestions("/too").len(), 0);

        assert_eq!(parse("/pet"), Ok(Command::Pets));

        assert_eq!(suggestions("/ti").len(), 2);

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
        assert!(parse("/ask extra").is_err());
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
