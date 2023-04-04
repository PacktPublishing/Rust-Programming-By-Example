//Using the path function
use std::path::Path;
//Playing with OpenOptions
use std::fs::OpenOptions;

fn some_func_path<P: AsRef<Path>>(p: P) {
    let p: &Path = p.as_ref();
    
    println!("Do nothing");
}

//slices
fn print_as_ascii(v: &[u8]) {
    for c in v {
        print!("{}", *c as char);
    }
}
//explaining the some function
fn some_func<'a, T: Into<Option<&'a str>>>(arg: T) {
    let arg = arg.into();
    if let Some(a) = arg {
        println!("{}", a);
    } else {
        println!("nothing...");
    }
}

//Using mutable borrows
struct Number(u32);

impl Number {
    fn new(nb: u32) -> Number {
        Number(nb)
    }

    fn add(&mut self, other: u32) -> &mut Number {
        self.0 += other;
        self
    }

    fn sub(&mut self, other: u32) -> &mut Number {
        self.0 -= other;
        self
    }

    fn compute(&self) -> u32 {
        self.0
    }
}

//playing with moves
struct Number2(u32);

impl Number2 {
    fn new(nb: u32) -> Number2 {
        Number2(nb)
    }

    fn add(mut self, other: u32) -> Number2 {
        self.0 += other;
        self
    }

    fn sub(mut self, other: u32) -> Number2 {
        self.0 -= other;
        self
    }
}

//Matching
enum SomeEnum {
    Ok,
    Err,
    Unknown,
}

fn main() {
    
    //slices implementation
    let v = b"salut!";

    print_as_ascii(&v[2..].to_vec());
    println!("");

    //explaining the some function
    some_func("ratatouille");
    some_func(None);

    //using the path function
    some_func_path("tortuga.txt");

    //playing with OpenOptions
    let file = OpenOptions::new()
                       .read(true)
                       .write(true)
                       .create(true)
                       .open("foo.txt");
    
    //using mutable borrows
    let nb = Number::new(0).add(10).sub(5).add(12).compute();
    assert_eq!(nb, 17);

    //playing with moves
    let nb = Number2::new(0).add(10).sub(5).add(12);
    assert_eq!(nb.0, 17);

    //Matching
    let x = SomeEnum::Ok;

    if let SomeEnum::Ok = x {
        print!("This will run only if SomeEnum is ok");
    }
    else{
        print!("This will run if SomeEnum is anything other than ok");
    }
}
