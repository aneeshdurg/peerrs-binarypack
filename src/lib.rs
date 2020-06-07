pub mod binarypack;
pub mod error;

#[cfg(test)]
mod tests {
    use crate::binarypack;
    #[test]
    fn binarypack_unpack() {
        let a = [1, 2, 3];
        match binarypack::unpack(&a).unwrap() {
            binarypack::Unpacked::Uint8(s) => {
                println!("u8: {}", s);
            }
            _ => {}
        }
    }
}
