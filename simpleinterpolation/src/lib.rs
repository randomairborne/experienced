#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
//! # `SimpleInterpolation`
//!
//! A dead simple interpolation format.
//!
//! for example: `this is an {interpolated} string`
//!
//! Variable names may have `-`, `_`, `0-9`, `a-z`, and `A-Z`, any other characters will be raised as errors.
//!
use std::{borrow::Cow, collections::HashMap, fmt::Formatter};

/// The main entrypoint for this crate.
/// Created with [`Interpolation::new`], this represents
/// a template that can be supplied variables to render.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Interpolation {
    /// The first value is the raw value that will be appended
    /// to the final string. The second value will go AFTER this string,
    /// but it is dynamic
    parts: Vec<(String, String)>,
    /// The value which is placed after the otherwise rendered interpolation
    end: String,
}

impl Interpolation {
    const REASONABLE_INTERPOLATION_PREALLOC_BYTES: usize = 128;

    /// Create a new [`Interpolation`].
    /// # Errors
    /// This function usually errors when there is a syntax error
    /// in the interpolation compiler, e.g. an unclosed identifier or an invalid escape.
    pub fn new(input: impl AsRef<str>) -> Result<Self, ParseError> {
        InterpolationCompiler::compile(input.as_ref())
    }

    /// Create a new string with capacity to be reasonably rendered into.
    fn output_string(&self) -> String {
        String::with_capacity(
            self.parts
                .iter()
                .map(|v| v.0.len() + Self::REASONABLE_INTERPOLATION_PREALLOC_BYTES)
                .sum(),
        )
    }

    /// Renders this template, using the `args` hashmap to fetch
    /// interpolation values from. Said values *must* be strings.
    /// If an interpolation value is not found, it is replaced with an empty string.
    #[must_use]
    pub fn render(&self, args: &HashMap<Cow<str>, Cow<str>>) -> String {
        let mut output = self.output_string();
        for (raw, interpolation_key) in &self.parts {
            output.push_str(raw);
            let interpolation_value = args.get(interpolation_key.as_str());
            output.push_str(interpolation_value.unwrap_or(&Cow::Borrowed("")));
        }
        output.push_str(&self.end);
        output
    }

    /// Renders this template, using the `args` hashmap to fetch
    /// interpolation values from. Said values *must* be strings.
    /// # Errors
    /// If an interpolation value is not found, it is added to the [`RenderError`].
    pub fn try_render<'a>(
        &'a self,
        args: &HashMap<Cow<str>, Cow<str>>,
    ) -> Result<String, RenderError<'a>> {
        let mut output = self.output_string();
        for (raw, interpolation_key) in &self.parts {
            output.push_str(raw);
            let Some(interpolation_value) = args.get(interpolation_key.as_str()) else {
                return Err(RenderError::UnknownVariables(
                    self.listify_unknown_args(args),
                ));
            };
            output.push_str(interpolation_value);
        }
        output.push_str(&self.end);
        Ok(output)
    }

    // this is the cold path. intentionally inefficient.
    fn listify_unknown_args<T>(&self, args: &HashMap<Cow<str>, T>) -> Vec<&str> {
        let mut output = Vec::with_capacity(args.len());
        for (_, key) in &self.parts {
            if !args.contains_key(key.as_str()) {
                output.push(key.as_str());
            }
        }
        output
    }

    /// Returns an iterator over all variables used in this interpolation.
    /// Useful if you have a non hashmap item you wish to get items from.
    pub fn variables_used(&self) -> impl Iterator<Item = &str> {
        UsedVariablesIterator {
            inner: self.parts.as_slice(),
            current: 0,
        }
    }

    // Rebuilds the value you put into the interpolation.
    #[must_use]
    pub fn input_value(&self) -> String {
        fn push_escape(s: &mut String, txt: &str) {
            for next in txt.chars() {
                if next == '{' || next == '\\' {
                    s.push('\\');
                }
                s.push(next);
            }
        }

        let mut output = self.output_string();
        for (text, key) in &self.parts {
            push_escape(&mut output, text);
            output.push('{');
            output.push_str(key);
            output.push('}');
        }
        push_escape(&mut output, &self.end);
        output
    }
}

struct UsedVariablesIterator<'a> {
    inner: &'a [(String, String)],
    current: usize,
}

impl<'a> Iterator for UsedVariablesIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.get(self.current)?.1.as_str();
        self.current += 1;
        Some(next)
    }
}

struct InterpolationCompiler {
    chars: Vec<char>,
    parts: Vec<(String, String)>,
    index: usize,
    next: String,
    escaped: bool,
}

impl InterpolationCompiler {
    fn compile(input: &str) -> Result<Interpolation, ParseError> {
        let mut compiler = Self {
            chars: input.chars().collect(),
            parts: Vec::new(),
            index: 0,
            next: String::new(),
            escaped: false,
        };

        // for each character, check if the character exists, then
        // feed it into the compiler
        while let Some(character) = compiler.chars.get(compiler.index).copied() {
            compiler.handle_char(character)?;
        }

        compiler.shrink();

        Ok(Interpolation {
            parts: compiler.parts,
            end: compiler.next,
        })
    }

    fn handle_char(&mut self, ch: char) -> Result<(), ParseError> {
        if self.escaped && ch != '{' && ch != '\\' {
            return Err(ParseError::InvalidEscape(ch, self.index));
        } else if self.escaped {
            self.next.push(ch);
            self.escaped = false;
        } else if ch == '\\' {
            self.escaped = true;
        } else if ch == '{' {
            self.index += 1;
            let mut ident = self.make_identifier()?;
            let mut to_push = std::mem::take(&mut self.next);
            ident.shrink_to_fit();
            to_push.shrink_to_fit();
            self.parts.push((to_push, ident));
        } else {
            self.next.push(ch);
        }
        self.index += 1;
        Ok(())
    }

    #[inline]
    const fn valid_ident_char(ch: char) -> bool {
        matches!(ch, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-')
    }

    fn make_identifier(&mut self) -> Result<String, ParseError> {
        let mut identifier = String::new();
        let start = self.index;
        loop {
            let identifier_part = self
                .chars
                .get(self.index)
                .copied()
                .ok_or(ParseError::UnclosedIdentifier(start))?;
            if identifier_part == '}' {
                break;
            }
            if !Self::valid_ident_char(identifier_part) {
                return Err(ParseError::InvalidCharInIdentifier(
                    identifier_part,
                    self.index,
                ));
            }
            identifier.push(identifier_part);
            self.index += 1;
        }
        Ok(identifier)
    }

    fn shrink(&mut self) {
        self.parts.shrink_to_fit();

        for (a, b) in &mut self.parts {
            a.shrink_to_fit();
            b.shrink_to_fit();
        }

        self.next.shrink_to_fit();
    }
}

/// Error returned in the parsing stage.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Unclosed identifier found at a specific spot.
    UnclosedIdentifier(usize),
    /// Invalid char (.0) in identifier, located at .1
    InvalidCharInIdentifier(char, usize),
    /// Invalid value (.0) escaped at usize (.1)
    InvalidEscape(char, usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnclosedIdentifier(at) => {
                write!(f, "Unclosed identifier (mismatched pair at {})", at + 1)
            }
            Self::InvalidCharInIdentifier(c, at) => {
                write!(f, "Invalid character `{c:?}` in identifier at {}", at + 1)
            }
            Self::InvalidEscape(c, at) => {
                write!(
                    f,
                    "`{c:?}` at position {} cannot be escaped, only `{{` and `\\` can",
                    at + 1
                )
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Errors returned by the [`Interpolation::try_render`] function.
#[derive(Debug, PartialEq, Eq)]
pub enum RenderError<'a> {
    /// Unknown variables used in the interpolation. Contains a list of them.
    UnknownVariables(Vec<&'a str>),
}

impl std::fmt::Display for RenderError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVariables(vars) => {
                write!(f, "Unknown variables used: ")?;
                for (idx, item) in vars.iter().enumerate() {
                    if idx == vars.len() - 1 {
                        write!(f, "{item}")?;
                    } else {
                        write!(f, "{item}, ")?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for RenderError<'_> {}

#[cfg(test)]
mod tests {
    #![allow(clippy::literal_string_with_formatting_args)]
    use std::collections::HashMap;

    use super::*;

    fn get_example_args() -> HashMap<Cow<'static, str>, Cow<'static, str>> {
        let mut hm = HashMap::new();
        hm.insert(
            Cow::Borrowed("interpolation"),
            Cow::Borrowed("Interpolation"),
        );
        hm.insert(Cow::Borrowed("unused"), Cow::Borrowed("ERROR"));
        hm
    }
    #[test]
    fn basic() {
        let interpolation =
            Interpolation::new("This is an example string for {interpolation}!").unwrap();
        println!("{interpolation:?}");
        let rendered = interpolation.render(&get_example_args());
        assert_eq!("This is an example string for Interpolation!", rendered);
    }
    #[test]
    fn escapes() {
        let initial = "This is an example string for \\{interpolation} escapes!";
        let target = "This is an example string for {interpolation} escapes!";
        let interpolation = Interpolation::new(initial).unwrap();
        println!("{interpolation:?}");
        assert_eq!(target, interpolation.render(&HashMap::new()));
    }
    #[test]
    fn recursive_escapes() {
        let initial = "This is an example string for \\\\{interpolation} recursive escapes!";
        let target = "This is an example string for \\Interpolation recursive escapes!";
        let interpolation = Interpolation::new(initial).unwrap();
        println!("{interpolation:?}");
        assert_eq!(target, interpolation.render(&get_example_args()));
    }
    #[test]
    fn variables_are_right() {
        let interpolation =
            Interpolation::new("This is an example string for {interpolation} variable listing!")
                .unwrap();
        println!("{interpolation:?}");
        assert_eq!(
            interpolation.variables_used().collect::<Vec<&str>>(),
            vec!["interpolation"]
        );
    }
    #[test]
    fn basic_roundtrip() {
        let roundtrip = "This is an example string for {interpolation}!";
        let interpolation = Interpolation::new(roundtrip).unwrap();
        println!("{interpolation:?}");
        assert_eq!(roundtrip, interpolation.input_value());
    }
    #[test]
    fn escapes_roundtrip() {
        let roundtrip = "This is an example string for \\{interpolation} escapes!";
        let interpolation = Interpolation::new(roundtrip).unwrap();
        println!("{interpolation:?}");
        assert_eq!(roundtrip, interpolation.input_value());
    }
    #[test]
    fn recursive_escapes_roundtrip() {
        let roundtrip = "This is an example string for \\\\{interpolation} recursive escapes!";
        let interpolation = Interpolation::new(roundtrip).unwrap();
        println!("{interpolation:?}");
        assert_eq!(roundtrip, interpolation.input_value());
    }
    #[test]
    fn no_interpolation() {
        let unchanged = "This is an example string for a lack of interpolation!";
        let interpolation = Interpolation::new(unchanged).unwrap();
        println!("{interpolation:?}");
        assert_eq!(unchanged, interpolation.render(&HashMap::new()));
    }
    #[test]
    fn error_nonexistents_found() {
        let one_interp = "{nonexistent}";
        let interpolation = Interpolation::new(one_interp).unwrap();
        println!("{interpolation:?}");
        assert_eq!(
            Err(RenderError::UnknownVariables(vec!["nonexistent"])),
            interpolation.try_render(&HashMap::new())
        );
    }
    #[test]
    fn error_nonexistents_found_2() {
        let one_interp = "{nonexistent} {nonexistent2}";
        let interpolation = Interpolation::new(one_interp).unwrap();
        println!("{interpolation:?}");
        assert_eq!(
            Err(RenderError::UnknownVariables(vec![
                "nonexistent",
                "nonexistent2"
            ])),
            interpolation.try_render(&HashMap::new())
        );
    }
    #[test]
    fn error_bad_ident() {
        let bad_template = "{a)";
        let interpolation = Interpolation::new(bad_template);
        assert_eq!(
            interpolation,
            Err(ParseError::InvalidCharInIdentifier(')', 2))
        );
    }
    #[test]
    fn error_unclosed() {
        let bad_template = "{a";
        let interpolation = Interpolation::new(bad_template);
        assert_eq!(interpolation, Err(ParseError::UnclosedIdentifier(1)));
    }
    #[test]
    fn error_bad_escape() {
        let bad_template = "\\a";
        let interpolation = Interpolation::new(bad_template);
        assert_eq!(interpolation, Err(ParseError::InvalidEscape('a', 1)));
    }
}
