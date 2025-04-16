#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
// We allow cast precision loss because we will never be messing with integers bigger then 52 bits realistically
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#![no_std]
//! A library to calculate mee6 levels.
//! This can be calculated using the `LevelInfo` struct.

/// `LevelInfo` stores all of the data calculated when using `LevelInfo::new`(), so it can be cheaply
/// gotten with getters.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct LevelInfo {
    xp: u64,
    level: u64,
    percentage: f64,
}

impl LevelInfo {
    /// Create a new `LevelInfo` struct. This operation calculates the current percentage and level
    /// immediately, rather then when the getter is called.
    #[must_use]
    pub fn new(xp: u64) -> Self {
        // The operation used to calculate how many XP a given level is is (5 / 6) * level * (2 * level * level + 27 * level + 91),
        // but it is not really possible to cleanly calculate the inverse of this
        let level = {
            let mut testxp = 0;
            let mut level = 0;
            while xp >= testxp {
                level += 1;
                testxp = xp_needed_for_level(level);
            }
            level - 1
        };
        let last_level_xp_requirement = xp_needed_for_level(level);
        let next_level_xp_requirement = xp_needed_for_level(level + 1);
        Self {
            xp,
            level,
            percentage: ((xp as f64 - last_level_xp_requirement as f64)
                / (next_level_xp_requirement as f64 - last_level_xp_requirement as f64)),
        }
    }

    /// Get the xp that was input into this `LevelInfo`.
    #[must_use]
    #[inline]
    pub const fn xp(&self) -> u64 {
        self.xp
    }

    /// Get the level that this `LevelInfo` represents.
    #[must_use]
    #[inline]
    pub const fn level(&self) -> u64 {
        self.level
    }

    /// Get the percentage of the way this `LevelInfo` is to gaining a level, from the last level.
    #[must_use]
    #[inline]
    pub const fn percentage(&self) -> f64 {
        self.percentage
    }
}

#[must_use]
pub fn xp_needed_for_level(level: u64) -> u64 {
    let level = level as f64;
    ((5.0 / 6.0) * level * (2.0 * level * level + 27.0 * level + 91.0)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn level() {
        let inf = LevelInfo::new(3255);
        assert_eq!(inf.level(), 8);
    }
    #[test]
    fn xp() {
        let inf = LevelInfo::new(3255);
        assert_eq!(inf.xp(), 3255);
    }
    #[test]
    fn percentage() {
        let inf = LevelInfo::new(3255);
        assert!((inf.percentage() - 0.43).abs() > f64::EPSILON);
    }
}
