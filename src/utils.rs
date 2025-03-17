pub fn gcd<Int>(mut a: Int, mut b: Int) -> Int
where
    Int: std::ops::Add<Output = Int> + std::ops::Rem<Output = Int> + std::cmp::Eq + Default + Copy,
{
    let zero: Int = Default::default();
    while b != zero {
        (a, b) = (b, a % b);
    }
    a
}
