use std::{
    process::{Command, ExitStatus},
    string::FromUtf8Error,
};

fn main() {
    let commit_msg = match get_sha() {
        Ok(v) => v,
        Err(err) => {
            println!("cargo::warning={err:?}");
            "(Failed to get version)".to_owned()
        }
    };

    println!("cargo:rustc-env=GIT_HASH_EXPERIENCED={}", commit_msg);
}

fn get_sha() -> Result<String, Error> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .spawn()?
        .wait_with_output()?;
    if !output.status.success() {
        return Err(Error::BadStatus(output.status));
    }
    let output = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(output)
}

#[derive(Debug)]
enum Error {
    TryFromString,
    BadStatus(ExitStatus),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TryFromString => write!(f, "Invalid UTF-8 in `git rev-parse HEAD` output")?,
            Error::BadStatus(exit_status) => write!(
                f,
                "`git rev-parse HEAD` exited with non-zero status {exit_status}"
            )?,
            Error::Io(error) => write!(f, "I/O error trying to run `git rev-parse HEAD`: {error}")?,
        }
        Ok(())
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_: FromUtf8Error) -> Self {
        Self::TryFromString
    }
}

impl From<ExitStatus> for Error {
    fn from(value: ExitStatus) -> Self {
        Self::BadStatus(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
