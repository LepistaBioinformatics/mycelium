use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub struct MappedErrors {
    msg: String,
}

impl Display for MappedErrors {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.msg)
    }
}

impl MappedErrors {
    pub fn new(
        msg: String,
        exp: Option<bool>,
        prev: Option<MappedErrors>,
    ) -> MappedErrors {
        if !exp.unwrap_or(true) {
            panic!("Unexpected error: {}", &msg);
        }

        if prev.is_some() {
            println!("Previous error: {:?}", &prev);
        }

        MappedErrors { msg }
    }
}
