#[macro_use]
mod module;

fn main() {
    let p1 = Point { x: 1, y: 2 };
    let p2 = Point { x: 2, y: 3 };
    let p3 = p1 + p2;
    println!("{:?}", p3);

    let p1 = Point { x: 1, y: 2 };
    let p2 = Point { x: 2, y: 3 };
    let p4 = p1 - p2;
    println!("{:?}", p4);

    let hashmap = hash! {
        "one" => 1,
        "two" => 2,
    };
    println!("{:?}", hashmap);

    /*let hashmap = {
        let mut hashmap = ::std::collections::HashMap::new();
        hashmap.insert("one", 1);
        hashmap.insert("two", 2);
        hashmap
    };*/
}

trait BitSet {
    fn clear(&mut self, index: usize);
    fn is_set(&self, index: usize) -> bool;
    fn set(&mut self, index: usize);
}

use std::ops::{BitAnd, BitAndAssign, BitOrAssign, Not, Shl, Shr};

impl<T: BitAnd<Output=T> + BitAndAssign + BitOrAssign + Not + PartialEq + Shl<T> + Shr<usize, Output=T>> BitSet for T {
    fn clear(&mut self, index: usize) {
        *self &= !(1 << index);
    }

    fn is_set(&self, index: usize) -> bool {
        (*self >> index) & 1 == 1
    }

    fn set(&mut self, index: usize) {
        *self |= 1 << index;
    }
}

/*macro_rules! int_bitset {
    ($ty:ty) => {
        impl BitSet for $ty {
            fn clear(&mut self, index: usize) {
                *self &= !(1 << index);
            }

            fn is_set(&self, index: usize) -> bool {
                (*self >> index) & 1 == 1
            }

            fn set(&mut self, index: usize) {
                *self |= 1 << index;
            }
        }
    };
}*/

/*int_bitset!(i32);
int_bitset!(u8);
int_bitset!(u64);*/

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

macro_rules! op {
    (+ $_self:ident : $self_type:ty, $other:ident $expr:expr) => {
        impl ::std::ops::Add for $self_type {
            type Output = $self_type;

            fn add($_self, $other: $self_type) -> $self_type {
                $expr
            }
        }
    };
    (- $_self:ident : $self_type:ty, $other:ident $expr:expr) => {
        impl ::std::ops::Sub for $self_type {
            type Output = $self_type;

            fn sub($_self, $other: $self_type) -> $self_type {
                $expr
            }
        }
    };
}

op!(+ self:Point, other {
    Point {
        x: self.x + other.x,
        y: self.y + other.y,
    }
});

op!(- self:Point, other {
    Point {
        x: self.x - other.x,
        y: self.y - other.y,
    }
});
