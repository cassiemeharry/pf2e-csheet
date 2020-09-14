#[macro_export]
macro_rules! format {
    ($($tt:tt)+) => {{
        use ::std::fmt::Write as _;
        let mut s = ::smartstring::alias::String::new();
        write!(s, $($tt)+).unwrap();
        s
    }};
}

#[macro_export]
macro_rules! try_from_str {
    ($t:ty) => {
        impl ::std::convert::TryFrom<::smartstring::alias::String> for $t {
            type Error = <$t as ::std::str::FromStr>::Err;

            #[inline]
            fn try_from(
                s: ::smartstring::alias::String,
            ) -> ::std::result::Result<Self, Self::Error> {
                let s_ref: &str = &s;
                <$t as ::std::str::FromStr>::from_str(s_ref)
            }
        }
    };
}
