fn main() {
    let quotient = Expr::Div { dividend: 10, divisor: 2 };
    let sum = Expr::Add(40, 2);
    match eval(sum) {
        Some(value) => println!("{}", value),
        None => println!("No value"),
    }
    println!("{}", uppercase(b'a') as char);
    println!("{}", uppercase(b'S') as char);
    println!("{}", is_alphanumeric('-'));

    let tuple = (24, 42);
    let (a, b) = tuple;
    println!("{}, {}", a, b);
}

fn print_expr(expr: Expr) {
    match expr {
        Expr::Null => println!("No value"),
        Expr::Add(x, y) => println!("{}", x + y),
        Expr::Sub(x, y) => println!("{}", x - y),
        Expr::Mul(x, y) => println!("{}", x * y),
        Expr::Div { dividend: x, divisor: 0 } => println!("Divisor is zero"),
        Expr::Div { dividend: x, divisor: y } => println!("{}", x / y),
        Expr::Val(x) => println!("{}", x),
    }
}

fn eval(expr: Expr) -> Option<i32> {
    match expr {
        Expr::Null => None,
        Expr::Add(x, y) => Some(x + y),
        Expr::Sub(x, y) => Some(x - y),
        Expr::Mul(x, y) => Some(x * y),
        Expr::Div { divisor: 0, .. } => None,
        Expr::Div { dividend, divisor } => Some(dividend / divisor),
        Expr::Val(x) => Some(x),
    }
}

enum Expr {
    Null,
    Add(i32, i32),
    Sub(i32, i32),
    Mul(i32, i32),
    Div { dividend: i32, divisor: i32 },
    Val(i32),
}

fn uppercase2(c: u8) -> u8 {
    match c {
        b'a'...b'z' => c - 32,
        _ => c,
    }
}

fn uppercase(c: u8) -> u8 {
    if let b'a'...b'z' = c {
        c - 32
    } else {
        c
    }
}

fn is_alphanumeric(c: char) -> bool {
    match c {
        'a'...'z' | 'A'...'Z' | '0'...'9' => true,
        _ => false,
    }
}

/*fn is_null(expr: Expr) -> bool {
    if let Expr::Null = expr
}*/
