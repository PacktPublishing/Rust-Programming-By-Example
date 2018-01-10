fn main() {
    let tuple = (24, 42);
    println!("({}, {})", tuple.0, tuple.1);

    let (hello, world) = "helloworld".split_at(5);
    println!("{}, {}!", hello, world);
}
