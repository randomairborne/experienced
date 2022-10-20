pub struct Level {
    level: u64,
    xp: u64,
    needed: u64,
    percentage: u8
}

impl Level {
    pub fn new(xp: u64) -> Self {

    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
