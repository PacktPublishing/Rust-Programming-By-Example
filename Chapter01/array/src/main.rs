fn main() {
    let array = [1, 2, 3, 4];
    let array: [i16; 4] = [1, 2, 3, 4];
    let array = [1u8, 2, 3, 4];
    //println!("{}", array[4]);
    println!("{}", first(&array));
    println!("{}", first(&array[2..]));

    let array = [1, 2, 3, 4];
    let mut sum = 0;
    for element in &array {
        sum += *element;
    }
    println!("Sum: {}", sum);

    println!("{:?}", index(&array, &3));
    println!("{:?}", index(&array, &6));
    println!("{:?}", min_max(&[]));
    println!("{:?}", min_max(&[5, 2, 7, 9, 3, 1]));
}

fn first<T>(slice: &[T]) -> &T {
    &slice[0]
}

fn index<T: PartialEq>(slice: &[T], target: &T) -> Option<usize> {
    for (index, element) in slice.iter().enumerate() {
        if element == target {
            return Some(index);
        }
    }
    None
}

fn min_max(slice: &[i32]) -> Option<(i32, i32)> {
    if slice.is_empty() {
        return None;
    }
    let mut min = slice[0];
    let mut max = slice[0];
    for &element in slice {
        if element < min {
            min = element;
        }
        if element > max {
            max = element;
        }
    }
    Some((min, max))
}
