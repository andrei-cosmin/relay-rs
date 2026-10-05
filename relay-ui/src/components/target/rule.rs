#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Verb {
    Set,
    Remove,
    Cors,
    Block,
    Key,
    Unknown,
}

impl Verb {
    const ALL: [Self; 5] = [Self::Set, Self::Remove, Self::Cors, Self::Block, Self::Key];

    fn word(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Remove => "remove",
            Self::Cors => "cors",
            Self::Block => "block",
            Self::Key => "key",
            Self::Unknown => "",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Set => "SET",
            Self::Remove => "REMOVE",
            Self::Cors => "CORS",
            Self::Block => "BLOCK",
            Self::Key => "KEY",
            Self::Unknown => "RULE",
        }
    }

    pub(crate) fn tone(self) -> &'static str {
        match self {
            Self::Set => "set",
            Self::Remove => "remove",
            Self::Cors => "cors",
            Self::Block => "block",
            Self::Key => "key",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn hint(self) -> &'static str {
        match self {
            Self::Set => "header value",
            Self::Remove => "header names or globs",
            Self::Cors | Self::Block => "",
            Self::Key => "secret the client must send",
            Self::Unknown => "set <header> <value> · remove <names> · cors · block · key <secret>",
        }
    }

    pub(crate) fn takes_args(self) -> bool {
        !matches!(self, Self::Cors | Self::Block)
    }
}

pub(crate) struct Rule<'a> {
    pub(crate) verb: Verb,
    pub(crate) args: &'a str,
}

impl<'a> Rule<'a> {
    const SECRET_HEADERS: [&'static str; 5] = [
        "authorization",
        "proxy-authorization",
        "x-api-key",
        "api-key",
        "cookie",
    ];

    pub(crate) fn parse(line: &'a str) -> Self {
        let (word, args) = line.split_once(' ').unwrap_or((line, ""));
        match Verb::ALL
            .into_iter()
            .find(|verb| word.eq_ignore_ascii_case(verb.word()))
        {
            Some(verb) => Self { verb, args },
            None => Self {
                verb: Verb::Unknown,
                args: line,
            },
        }
    }

    pub(crate) fn secret(&self) -> Option<(&'a str, &'a str)> {
        let (name, value) = self.args.split_once(' ').unwrap_or((self.args, ""));
        let secret = Self::SECRET_HEADERS
            .iter()
            .any(|header| name.eq_ignore_ascii_case(header));
        (self.verb == Verb::Set && secret).then_some((name, value))
    }

    pub(crate) fn hidden(&self) -> bool {
        self.verb == Verb::Key
    }

    pub(crate) fn line(verb: Verb, args: &str) -> String {
        match (verb, args.is_empty()) {
            (Verb::Unknown, _) => args.to_owned(),
            (verb, true) => verb.word().to_owned(),
            (verb, false) => format!("{} {args}", verb.word()),
        }
    }
}
