#[macro_export]
macro_rules! hash {
    ($( $key:expr => $value:expr ),* $(,)* ) => {{
        //let keys = [$($key),*];
        let mut hashmap = ::std::collections::HashMap::new();
        $(hashmap.insert($key, $value);)*
        hashmap
    }};
}
