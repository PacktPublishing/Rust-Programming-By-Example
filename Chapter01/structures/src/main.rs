fn main() {
    let point = Point {
        x: 24,
        y: 42,
    };
    println!("({}, {})", point.x, point.y);
    println!("{:?}", point);
    println!("{:#?}", point);
    println!("{}", point.dist_from_origin());
    println!("{}", Point { x: 3, y: 4 }.dist_from_origin());
}

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new2(x: i32, y: i32) -> Self {
        Self { x: x, y: y }
    }

    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn origin() -> Self {
        Point { x: 0, y: 0 }
    }

    fn translate(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    fn dist_from_origin(&self) -> f64 {
        let sum_of_squares = self.x.pow(2) + self.y.pow(2);
        (sum_of_squares as f64).sqrt()
    }
}
