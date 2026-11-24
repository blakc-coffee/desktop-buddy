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

    Agent,

    Find(String),

    Workspaces(String),

    Timer { minutes: u32, title: String },

    Reminder { minutes: u32, title: String },
    /// A clock time; `twelve_hour` marks "5:30" without am/pm, resolved to its next occurrence.
    ReminderAt { hour: u32, minute: u32, twelve_hour: bool, title: String },

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

pub fn definitions() -> [(&'static [&'static str], &'static str, &'static str); 9] {
    [
        (&["/sc", "/screen", "/fill"], "/sc question · Capture and fill", "/sc "),
        (&["/exp", "/explain"], "/exp question · Explain screen", "/exp "),
        (&["/t", "/timer", "/focus"], "/t 25m task · Timer", "/t "),
        (&["/r", "/remind"], "/r 10m message · Reminder", "/r "),
        (&["/d", "/todo"], "/d task · Add todo", "/d "),
        (&["/todos"], "/todos · Checklist", "/todos"),
        (&["/f", "/find"], "/f name · Find files", "/f "),
        (&["/tools", "/settings"], "/tools · Manage tools and settings", "/tools"),
        (&["/agent"], "/agent · Connect an AI agent", "/agent"),
    ]
}

pub fn parse(text: &str) -> Result<Command, String> {
    let text = text.trim();

    if text.len() > 1024 {
        return Err("Command is too long.".into());
    }

    if !text.is_empty() && !text.starts_with('/') {
        return Ok(natural(text).unwrap_or_else(|| Command::Question(text.into())));
    }

    let (verb, args) = text.split_once(char::is_whitespace).unwrap_or((text, ""));

    let args = args.trim();

    let verb = verb.to_ascii_lowercase();

    match verb.as_str() {
        "" | "/" | "/help" => Ok(Command::Help),

        "/tools" | "/actions" if args.is_empty() => Ok(Command::Tools),

        "/codex" | "/ask" | "/resume" | "/projects" | "/w" | "/workspace" => Err("This feature is archived for now.".into()),
        "/sc" | "/screen" | "/fill" => Ok(Command::Screen { question: args.into(), fill: true }),
        "/exp" | "/explain" => Ok(Command::Screen { question: args.into(), fill: false }),








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

        "/agent" if args.is_empty() => Ok(Command::Agent),

        "/f" | "/find" => Ok(Command::Find(args.into())),



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

/// Plain-language reminders and timers, recognised locally so they cost no AI round trip.
/// Anything this does not recognise falls through to the chat, which may still return an action.
pub fn natural(text: &str) -> Option<Command> {
    let raw: Vec<&str> = text.split_whitespace().collect();
    let words: Vec<String> = raw
        .iter()
        .map(|w| w.trim_matches(|c: char| ",.!?;".contains(c)).to_lowercase())
        .collect();
    let first = words.first()?.as_str();
    let remind_at = words
        .iter()
        .take(5)
        .position(|w| w == "remind")
        .filter(|&i| words.get(i + 1).is_some_and(|w| w == "me"));
    let reminder = remind_at.is_some() || matches!(first, "reminder" | "remind");
    let span = duration(&words);
    let timer = !reminder
        && words
            .iter()
            .any(|w| matches!(w.as_str(), "timer" | "pomodoro" | "focus"))
        && (matches!(first, "timer" | "set" | "start" | "focus" | "pomodoro")
            || span.is_some_and(|(_, _, at)| at == 0));
    if !reminder && !timer {
        return None;
    }
    let mut used = vec![false; words.len()];
    let mark = |used: &mut Vec<bool>, from: usize, to: usize| {
        used[from..to].iter_mut().for_each(|u| *u = true)
    };
    if let Some(i) = remind_at {
        mark(&mut used, i, i + 2);
    }
    for (i, w) in words.iter().enumerate() {
        if matches!(w.as_str(), "timer" | "pomodoro" | "reminder") || (timer && w == "focus") {
            used[i] = true;
        }
    }
    if let Some((_, len, at)) = span {
        let from = if at > 0 && matches!(words[at - 1].as_str(), "in" | "for" | "after") {
            at - 1
        } else {
            at
        };
        mark(&mut used, from, at + len);
    }
    let mut clock = None;
    if reminder && span.is_none() {
        for i in 0..words.len() {
            if words[i] == "at" {
                if let Some((hour, minute, twelve_hour, len)) = clock_at(&words[i + 1..]) {
                    mark(&mut used, i, i + 1 + len);
                    clock = Some((hour, minute, twelve_hour));
                    break;
                }
            }
        }
    }
    const FILLER: &[&str] = &[
        "set", "start", "a", "an", "me", "to", "for", "about", "that", "please", "on", "in",
        "at", "can", "you", "could", "remind", "and",
    ];
    let mut kept: Vec<usize> = (0..words.len()).filter(|&i| !used[i]).collect();
    while kept
        .first()
        .is_some_and(|&i| FILLER.contains(&words[i].as_str()))
    {
        kept.remove(0);
    }
    while kept
        .last()
        .is_some_and(|&i| FILLER.contains(&words[i].as_str()))
    {
        kept.pop();
    }
    let title: String = kept
        .iter()
        .map(|&i| raw[i])
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(|c: char| ",.!?;".contains(c))
        .chars()
        .take(240)
        .collect();
    let named = |fallback: &str| {
        if title.is_empty() {
            fallback.to_string()
        } else {
            title.clone()
        }
    };
    match (reminder, span, clock) {
        (true, Some((minutes, _, _)), _) => Some(Command::Reminder {
            minutes,
            title: named("Reminder"),
        }),
        (true, None, Some((hour, minute, twelve_hour))) => Some(Command::ReminderAt {
            hour,
            minute,
            twelve_hour,
            title: named("Reminder"),
        }),
        (false, Some((minutes, _, _)), _) => Some(Command::Timer {
            minutes,
            title: named("Focus session"),
        }),
        _ => None,
    }
}
/// First duration in the words as (minutes, word count, start index):
/// "25m", "25 min", "2 hours", "an hour", "half an hour".
fn duration(words: &[String]) -> Option<(u32, usize, usize)> {
    let unit = |u: &str| match u {
        "m" | "min" | "mins" | "minute" | "minutes" => Some(1),
        "h" | "hr" | "hrs" | "hour" | "hours" => Some(60),
        _ => None,
    };
    for i in 0..words.len() {
        let w = words[i].as_str();
        let next = |k: usize, value: &str| words.get(i + k).is_some_and(|x| x == value);
        let found = if w == "half" && next(1, "an") && next(2, "hour") {
            Some((30, 3))
        } else if matches!(w, "an" | "a") && next(1, "hour") {
            Some((60, 2))
        } else if let Ok(n) = w.parse::<u32>() {
            words
                .get(i + 1)
                .and_then(|u| unit(u))
                .map(|m| (n.saturating_mul(m), 2))
        } else {
            w.find(|c: char| !c.is_ascii_digit())
                .filter(|&at| at > 0)
                .and_then(|at| Some((w[..at].parse::<u32>().ok()?.saturating_mul(unit(&w[at..])?), 1)))
        };
        if let Some((minutes, len)) = found {
            return (1..=1440).contains(&minutes).then_some((minutes, len, i));
        }
    }
    None
}
/// A clock time after "at": "5", "5pm", "5 pm", "5:30", "17:30".
/// Returns (hour, minute, ambiguous 12-hour time, word count).
fn clock_at(words: &[String]) -> Option<(u32, u32, bool, usize)> {
    let w = words.first()?.as_str();
    let (body, mut meridiem) = match w.strip_suffix("pm").or_else(|| w.strip_suffix("p.m.")) {
        Some(body) => (body, Some(true)),
        None => match w.strip_suffix("am").or_else(|| w.strip_suffix("a.m.")) {
            Some(body) => (body, Some(false)),
            None => (w, None),
        },
    };
    let mut len = 1;
    if meridiem.is_none() {
        match words.get(1).map(String::as_str) {
            Some("pm" | "p.m.") => (meridiem, len) = (Some(true), 2),
            Some("am" | "a.m.") => (meridiem, len) = (Some(false), 2),
            _ => {}
        }
    }
    let (h, m) = body.split_once([':', '.']).unwrap_or((body, "0"));
    let (mut hour, minute) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    if minute > 59 || hour > 23 || (meridiem.is_some() && !(1..=12).contains(&hour)) {
        return None;
    }
    match meridiem {
        Some(true) if hour < 12 => hour += 12,
        Some(false) if hour == 12 => hour = 0,
        _ => {}
    }
    Some((hour, minute, meridiem.is_none() && hour <= 12, len))
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]

    fn autocomplete_browses_empty_input_and_leaves_arguments_alone() {
        assert_eq!(suggestions("").len(), 9);

        assert!(parse("/resume").is_err());

        assert!(suggestions("/copy").is_empty());

        assert_eq!(parse("/sc"), Ok(Command::Screen { question: String::new(), fill: true }));
        assert_eq!(parse("/exp"), Ok(Command::Screen { question: String::new(), fill: false }));
        assert!(parse("/codex").is_err());

        assert_eq!(suggestions("/too").len(), 1);

        assert_eq!(parse("/pet"), Ok(Command::Pets));
        assert_eq!(parse("/agent"), Ok(Command::Agent));

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
        assert!(parse("/ask").is_err());
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

        assert!(parse("/w demo").is_err());
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
    #[test]
    fn plain_language_reminders_and_timers_need_no_slash() {
        let r = |minutes, title: &str| Some(Command::Reminder { minutes, title: title.into() });
        let t = |minutes, title: &str| Some(Command::Timer { minutes, title: title.into() });
        let at = |hour, minute, twelve_hour, title: &str| {
            Some(Command::ReminderAt { hour, minute, twelve_hour, title: title.into() })
        };
        assert_eq!(natural("remind me to drink water in 20 minutes"), r(20, "drink water"));
        assert_eq!(natural("Remind me in 2 hours to stretch."), r(120, "stretch"));
        assert_eq!(natural("can you remind me in half an hour about the oven"), r(30, "the oven"));
        assert_eq!(natural("remind me in 10m"), r(10, "Reminder"));
        assert_eq!(natural("remind me at 5pm to call Saran"), at(17, 0, false, "call Saran"));
        assert_eq!(natural("remind me to submit DBMS at 5:30"), at(5, 30, true, "submit DBMS"));
        assert_eq!(natural("remind me at 17:45 to leave"), at(17, 45, false, "leave"));
        assert_eq!(natural("remind me at 12 am to sleep"), at(0, 0, false, "sleep"));
        assert_eq!(natural("set a timer for 25 minutes for reading"), t(25, "reading"));
        assert_eq!(natural("25 min timer study"), t(25, "study"));
        assert_eq!(natural("timer 45m"), t(45, "Focus session"));
        assert_eq!(natural("focus for an hour on the essay"), t(60, "the essay"));
        // Questions that only mention timers or reminders stay questions.
        assert_eq!(natural("how does a timer work in javascript for 5 minutes"), None);
        assert_eq!(natural("what is a reminder email"), None);
        assert_eq!(natural("remind me to call mom"), None);
        assert_eq!(natural("timer 2000m"), None);
        assert_eq!(
            parse("remind me in 5 min to check the build"),
            Ok(Command::Reminder { minutes: 5, title: "check the build".into() })
        );
        assert_eq!(
            parse("What is a binary tree?"),
            Ok(Command::Question("What is a binary tree?".into()))
        );
    }
}
