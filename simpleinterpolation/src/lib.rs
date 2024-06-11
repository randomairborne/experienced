//! SimpleInterpolation
//!
//! A dead simple interpolation format
//! `this is an {interpolated} string`
//! Variable names may have `-` `_`, `a-z`, and `A-Z`, any other characters will cause errors.
//!
use std::{collections::HashMap, fmt::Formatter};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Interpolation {
    // The first value is the raw value that will be appended
    // to the final string. The second value will go AFTER this string,
    // but it is dynamic
    parts: Vec<(String, String)>,
    // The value which is placed after the otherwise rendered interpolation
    end: String,
}

impl Interpolation {
    const REASONABLE_INTERPOLATION_PREALLOC_BYTES: usize = 128;

    pub fn new(input: impl AsRef<str>) -> Result<Self, Error> {
        InterpolationCompiler::compile(input.as_ref())
    }

    fn output_string(&self) -> String {
        String::with_capacity(
            self.parts
                .iter()
                .map(|v| v.0.len() + Self::REASONABLE_INTERPOLATION_PREALLOC_BYTES)
                .sum(),
        )
    }

    pub fn render(&self, args: &HashMap<String, String>) -> String {
        let mut output = self.output_string();
        for (raw, interpolation_key) in &self.parts {
            output.push_str(raw);
            let interpolation_value = args.get(interpolation_key);
            output.push_str(interpolation_value.unwrap_or(&String::new()));
        }
        output.push_str(&self.end);
        output
    }

    pub fn variables_used(&self) -> impl Iterator<Item = &str> {
        UsedVariablesIterator {
            inner: self.parts.as_slice(),
            current: 0,
        }
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
    fn compile(input: &str) -> Result<Interpolation, Error> {
        let mut compiler = Self {
            chars: input.chars().collect(),
            parts: Vec::new(),
            index: 0,
            next: String::new(),
            escaped: false,
        };

        // for each character, check if the character is a
        while let Some(character) = compiler.chars.get(compiler.index).copied() {
            compiler.handle_char(character)?;
        }

        compiler.parts.shrink_to_fit();

        Ok(Interpolation {
            parts: compiler.parts,
            end: compiler.next,
        })
    }

    fn handle_char(&mut self, ch: char) -> Result<(), Error> {
        if self.escaped && ch != '{' && ch != '\\' {
            return Err(Error::InvalidEscape(ch, self.index));
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
        };
        self.index += 1;
        Ok(())
    }

    #[inline]
    fn valid_ident_char(ch: char) -> bool {
        matches!(ch, 'A'..='Z' | 'a'..='z' | '_' | '-')
    }

    fn make_identifier(&mut self) -> Result<String, Error> {
        let mut identifier = String::new();
        let start = self.index;
        while let Some(identifier_part) = self.chars.get(self.index).copied() {
            if identifier_part == '}' {
                break;
            }
            if self.index >= self.chars.len() {
                return Err(Error::UnclosedIdentifier(start));
            }
            if !Self::valid_ident_char(identifier_part) {
                return Err(Error::InvalidCharInIdentifier(identifier_part, self.index));
            }
            identifier.push(identifier_part);
            self.index += 1;
        }
        Ok(identifier)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    UnclosedIdentifier(usize),
    InvalidCharInIdentifier(char, usize),
    InvalidEscape(char, usize),
}

impl std::fmt::Display for Error {
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

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn get_example_args() -> HashMap<String, String> {
        let mut hm = HashMap::new();
        hm.insert("interpolation".to_string(), "Interpolation".to_string());
        hm.insert("unused".to_string(), "ERROR".to_string());
        hm
    }
    #[test]
    fn basic() {
        let interpolation =
            Interpolation::new("This is an example string for {interpolation}!").unwrap();
        println!("{interpolation:?}");
        let rendered = interpolation.render(&get_example_args());
        assert_eq!("This is an example string for Interpolation!", rendered)
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
    fn no_interpolation() {
        let unchanged = "This is an example string for a lack of interpolation!";
        let interpolation = Interpolation::new(unchanged).unwrap();
        println!("{interpolation:?}");
        assert_eq!(unchanged, interpolation.render(&HashMap::new()));
    }
}
