fn main() {
    println!("{}", max('a', 'z'));
}

enum Option<T> {
    Some(T),
    None,
}

fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}
