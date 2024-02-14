#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap().to_string()
    }};
}

pub fn approx_eq<T>(l: &T, r: &T) -> bool {
    std::mem::discriminant::<T>(l) == std::mem::discriminant::<T>(r)
}
