// var BufferBuilderExports = require('./bufferbuilder');
//
// window.BufferBuilder = BufferBuilderExports.BufferBuilder;
// window.binaryFeatures = BufferBuilderExports.binaryFeatures;
// window.BlobBuilder = BufferBuilderExports.BlobBuilder;
// window.BinaryPack = require('./binarypack');
//
extern crate num;

pub mod binarypack;
pub mod error;

#[cfg(test)]
mod tests {
    use crate::binarypack;
    #[test]
    fn binarypack_unpack() {
        let a = [1, 2, 3];
        match binarypack::unpack(&a) {
            binarypack::Unpacked::Uint8(s) => {
                println!("u8: {}", s);
            }
            _ => {}
        }

        panic!("!");
    }
}
