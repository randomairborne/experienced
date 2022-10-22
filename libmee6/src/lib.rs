#![cfg_attr(test, feature(test))]
#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic)]
// We allow cast precision loss because we will never be messing with integers bigger then 52 bits realistically
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
#![no_std]
//! A library to calculate mee6 levels.

pub struct LevelInfo {
    xp: i64,
    level: i64,
    percentage: u8,
}

impl LevelInfo {
    #[must_use]
    pub fn new(xp: i64) -> Self {
        // The operation used to calculate how many XP a given level is is (5 / 6) * level * (2 * level * level + 27 * level + 91), but it's optimized here.
        let level = {
            let xp = xp as f64;
            let mut testxp = 0.0;
            let mut level = 0;
            while xp >= testxp {
                level += 1;
                testxp = Self::xp_to_level(f64::from(level));
            }
            level - 1
        };
        let last_level_xp_requirement = Self::xp_to_level(f64::from(level));
        let next_level_xp_requirement = Self::xp_to_level(f64::from(level + 1));
        Self {
            xp,
            level: i64::from(level),
            percentage: (last_level_xp_requirement / next_level_xp_requirement * 100.0) as u8,
        }
    }
    #[must_use]
    pub const fn xp(&self) -> i64 {
        self.xp
    }
    #[must_use]
    pub const fn level(&self) -> i64 {
        self.level
    }
    #[must_use]
    pub const fn percentage(&self) -> u8 {
        self.percentage
    }
    // mul_add is not no-std
    #[allow(clippy::suboptimal_flops)]
    #[inline]
    fn xp_to_level(level: f64) -> f64 {
        (5.0 / 6.0) * level * (2.0 * level * level + 27.0 * level + 91.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;
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
        assert_eq!(inf.percentage(), 77);
    }

    #[bench]
    fn create_levelinfo(b: &mut test::Bencher) {
        b.iter(|| {
            for i in 1..1_000_000 {
                test::black_box(LevelInfo::new(i));
            }
        })
    }
}
