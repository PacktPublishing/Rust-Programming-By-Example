fn main() {
    let number1 = 24;
    let number2 = 42;
    let minimum =
        if number1 < number2 {
            number1
        } else {
            number2
        };
    println!("{}", minimum);
    if number1 > number2 {
        println!("{} > {}", number1, number2);
    }
    else {
        println!("{} <= {}", number1, number2);
    }

    let mut a = 15;
    let mut b = 40;
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    println!("Greatest common divisor of 15 and 40 is: {}", a);
}
