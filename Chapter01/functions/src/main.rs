fn main() {
    println!("{}", gcd(10, 15));
}

fn max(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}

fn max2(a: i32, b: i32) -> i32 {
    if a > b {
        return a;
    }
    b
}

fn gcd(mut a: i32, mut b: i32) -> i32 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

fn min_max(a: i32, b: i32) -> (i32, i32) {
    if a > b {
        (b, a)
    } else {
        (a, b)
    }
}
