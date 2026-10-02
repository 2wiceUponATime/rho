use super::*;

pub enum Type {
    Variable(Symbol),
}

assert_size!(Type, 4);
