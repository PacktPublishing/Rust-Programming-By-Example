fn main() {
    let mut p1 = Point { x: 1, y: 2 };
    let p2 = p1.clone();
    print_point(p1.clone());
    inc_x(&mut p1);
    println!("{}", p1.x);

    let num1 = 42;
    let num2 = num1;
    println!("{}", num1);
}

fn print_point(point: Point) {
    println!("x: {}, y: {}", point.x, point.y);
}

fn inc_x(point: &mut Point) {
    point.x += 1;
}

#[derive(Clone)]
struct Point {
    x: i32,
    y: i32,
}
