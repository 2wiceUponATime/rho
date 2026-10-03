use super::*;

pub enum Type {
    Variable(Symbol),
    Infer,
}

assert_size!(Type, 8);
