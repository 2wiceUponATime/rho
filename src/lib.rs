pub mod ast;
pub mod interner;
pub mod parser;
pub mod session;
pub mod span;

macro_rules! assert_size {
    ($ty:ty, $size:expr) => {
        #[cfg(target_pointer_width = "64")]
        const _: () = assert!(
            std::mem::size_of::<$ty>() == $size,
            concat!("size of `", stringify!($ty), "` changed")
        );
    };
}
pub(crate) use assert_size;
