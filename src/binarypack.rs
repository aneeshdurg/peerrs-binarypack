use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::size_of;

use num::{NumCast, Unsigned};

use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub enum Unpacked {
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Bool(bool),
    Raw(Vec<u8>),
    String(String),
    Null,
    Undefined,
    Array(Vec<Unpacked>),
    Map(HashMap<Unpacked, Unpacked>),
}

impl PartialEq for Unpacked {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Unpacked::Bool(a), Unpacked::Bool(b)) => a == b,
            (Unpacked::Uint8(a), Unpacked::Uint8(b)) => a == b,
            (Unpacked::Uint16(a), Unpacked::Uint16(b)) => a == b,
            (Unpacked::Uint32(a), Unpacked::Uint32(b)) => a == b,
            (Unpacked::Uint64(a), Unpacked::Uint64(b)) => a == b,
            (Unpacked::Int8(a), Unpacked::Int8(b)) => a == b,
            (Unpacked::Int16(a), Unpacked::Int16(b)) => a == b,
            (Unpacked::Int32(a), Unpacked::Int32(b)) => a == b,
            (Unpacked::Int64(a), Unpacked::Int64(b)) => a == b,
            (Unpacked::Float(a), Unpacked::Float(b)) => a == b,
            (Unpacked::Double(a), Unpacked::Double(b)) => a == b,
            (Unpacked::Raw(a), Unpacked::Raw(b)) => a == b,
            (Unpacked::String(a), Unpacked::String(b)) => a == b,
            (Unpacked::Null, Unpacked::Null) => true,
            (Unpacked::Array(a), Unpacked::Array(b)) => {
                if a.len() != b.len() {
                    return false;
                }

                for i in 0..a.len() {
                    if a[i] != b[i] {
                        return false;
                    }
                }

                true
            }
            (Unpacked::Map(a), Unpacked::Map(b)) => {
                if a.len() != b.len() {
                    return false;
                }

                for (k, a_v) in a.iter() {
                    if let Some(b_v) = b.get(k) {
                        if b_v == a_v {
                            continue;
                        }
                    }

                    return false;
                }

                return true;
            }
            (_, _) => false,
        }
    }
}

impl Eq for Unpacked {}

impl Hash for Unpacked {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(format!("{:?}", self).as_bytes());
        state.finish();
    }
}

struct Unpacker<'a> {
    data: &'a [u8],
}

impl<'a> Unpacker<'a> {
    fn new(data: &[u8]) -> Unpacker {
        Unpacker { data }
    }

    fn unpack_unsigned<T: Copy + Unsigned + NumCast>(&mut self) -> Result<T> {
        let length = size_of::<T>();
        if self.data.len() < length {
            return Err(Error::EndOfData);
        }

        let mut digits = vec![];
        for i in 0..length {
            digits.push(T::from(self.data[i]).unwrap());
        }
        self.data = &self.data[length..];

        let mut val: T = T::zero();
        // If the cast of 256 fails, then T must be u8, so we know there's only one digit to
        // worry about.
        let shift = T::from(256).unwrap_or(T::zero());
        for d in digits {
            val = (val * shift) + d;
        }

        Ok(val)
    }

    fn unpack_uint8(&mut self) -> Result<u8> {
        self.unpack_unsigned()
    }

    fn unpack_int8(&mut self) -> Result<i8> {
        self.unpack_unsigned().map(|x: u8| x as i8)
    }

    fn unpack_uint16(&mut self) -> Result<u16> {
        self.unpack_unsigned()
    }

    fn unpack_int16(&mut self) -> Result<i16> {
        self.unpack_unsigned().map(|x: u16| x as i16)
    }

    fn unpack_uint32(&mut self) -> Result<u32> {
        self.unpack_unsigned()
    }

    fn unpack_int32(&mut self) -> Result<i32> {
        self.unpack_unsigned().map(|x: u32| x as i32)
    }

    fn unpack_uint64(&mut self) -> Result<u64> {
        self.unpack_unsigned()
    }

    fn unpack_int64(&mut self) -> Result<i64> {
        self.unpack_unsigned().map(|x: u64| x as i64)
    }

    fn unpack_raw(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut raw = vec!();
        if self.data.len() < size {
            return Err(Error::EndOfData);
        }

        for i in 0..size {
            raw.push(self.data[i]);
        }
        self.data = &self.data[size..];

        Ok(raw)
    }

    fn unpack_string(&mut self, size: usize) -> Result<String> {
        Ok(String::from_utf8(self.unpack_raw(size)?)?)
    }

    fn unpack_array(&mut self, size: usize) -> Result<Vec<Unpacked>> {
        let mut arr = vec!();
        for _i in 0..size {
            arr.push(self.unpack()?);
        }

        Ok(arr)
    }

    fn unpack_map(&mut self, size: usize) -> Result<HashMap<Unpacked, Unpacked>> {
        let mut map = HashMap::new();
        for _i in 0..size {
            map.insert(self.unpack()?, self.unpack()?);
        }

        Ok(map)
    }

    fn unpack_float(&mut self) -> Result<f32> {
        let uint32 = self.unpack_uint32()?;

        let sign = (if (uint32 >> 31) == 0 { 1 } else { -1 }) as f32;
        let exp = ((uint32 >> 23) as i16 & 0xff) - 127;
        let fraction = ((uint32 & 0x7fffff) | 0x800000) as f32;

        Ok(sign * fraction * 2f32.powf((exp - 23).into()))
    }

    fn unpack_double(&mut self) -> Result<f64> {
          let h32 = self.unpack_uint32()?;
          let l32 = (self.unpack_uint32()?) as f64;

          let sign = (if (h32 >> 31) == 0 { 1 } else { -1 }) as f64;
          let exp = ((h32 >> 20) as i32 & 0x7ff) - 1023;
          let hfrac = ((h32 & 0xfffff) | 0x100000) as f64;
          let frac = hfrac * 2f64.powf((exp - 20).into()) + l32 * 2f64.powf((exp - 52).into());

          Ok(sign * frac)
    }

    fn unpack(&mut self) -> Result<Unpacked> {
        let type_ = self.unpack_uint8()?;
        if type_ < 0x80 {
            return Ok(Unpacked::Uint8(type_));
        } else if (type_ ^ 0xe0) < 0x20 {
            return Ok(Unpacked::Int8((type_ ^ 0xe0) as i8 - 0x20));
        }

        let size = type_ ^ 0xa0;
        if size <= 0x0f {
            return Ok(Unpacked::Raw(self.unpack_raw(size as usize)?));
        }
        let size = type_ ^ 0xb0;
        if size <= 0x0f {
            return Ok(Unpacked::String(self.unpack_string(size as usize)?));
        }
        let size = type_ ^ 0x90;
        if size <= 0x0f {
            return Ok(Unpacked::Array(self.unpack_array(size as usize)?));
        }

        let size = type_ ^ 0x80;
        if size <= 0x0f {
            return Ok(Unpacked::Map(self.unpack_map(size as usize)?));
        }

        Ok(match type_ {
            0xc0 => Unpacked::Null,
            0xc2 => Unpacked::Bool(false),
            0xc3 => Unpacked::Bool(true),
            0xca => Unpacked::Float(self.unpack_float()?),
            0xcb => Unpacked::Double(self.unpack_double()?),
            0xcc => Unpacked::Uint8(self.unpack_uint8()?),
            0xcd => Unpacked::Uint16(self.unpack_uint16()?),
            0xce => Unpacked::Uint32(self.unpack_uint32()?),
            0xcf => Unpacked::Uint64(self.unpack_uint64()?),
            0xd0 => Unpacked::Int8(self.unpack_int8()?),
            0xd1 => Unpacked::Int16(self.unpack_int16()?),
            0xd2 => Unpacked::Int32(self.unpack_int32()?),
            0xd3 => Unpacked::Int64(self.unpack_int64()?),
            0xd8 => {
              let size = self.unpack_uint16()? as usize;
              Unpacked::String(self.unpack_string(size)?)
            },
            0xd9 => {
              let size = self.unpack_uint32()? as usize;
              Unpacked::String(self.unpack_string(size)?)
            },
            0xda => {
              let size = self.unpack_uint16()? as usize;
              Unpacked::Raw(self.unpack_raw(size)?)
            },
            0xdb => {
              let size = self.unpack_uint32()? as usize;
              Unpacked::Raw(self.unpack_raw(size)?)
            },
            0xdc => {
              let size = self.unpack_uint16()? as usize;
              Unpacked::Array(self.unpack_array(size)?)
            },
            0xdd => {
              let size = self.unpack_uint32()? as usize;
              Unpacked::Array(self.unpack_array(size)?)
            },
            0xde => {
              let size = self.unpack_uint16()? as usize;
              Unpacked::Map(self.unpack_map(size)?)
            },
            0xdf => {
              let size = self.unpack_uint32()? as usize;
              Unpacked::Map(self.unpack_map(size)?)
            },

            _ => Unpacked::Undefined,
        })

    }
}

pub fn unpack(data: &[u8]) -> Result<Unpacked> {
    Unpacker::new(data).unpack()
}

#[cfg(test)]
mod test {
    use super::*;

    impl Unpacked {
        fn is_undefined(&self) -> bool {
            match self {
                Unpacked::Undefined => true,
                _ => false,
            }
        }

        // fn _pack(&self, packed: &mut Vec<u8>) {
        //     match self {
        //         Uint8(a) => {
        //         },
        //         Uint16(a) => {
        //         },
        //         Uint32(a) => {
        //         },
        //         Uint64(a) => {
        //         },
        //         Int8(a) => {
        //         },
        //         Int16(a) => {
        //         },
        //         Int32(a) => {
        //         },
        //         Int64(a) => {
        //         },
        //         Float(a) => {},
        //         Double(a) => {},
        //         Bool(a) => {
        //         },
        //         Raw(a) => {},
        //         String(a) => {},
        //         Null => {},
        //         Undefined => {},
        //         Array(Vec<Unpacked>) => {},
        //         Map(HashMap<Unpacked => {}, Unpacked>) => {},
        //     }
        // }

        // fn pack(&self) -> Vec<u8> {
        //     let mut packed = vec!();
        //     self._pack(packed);
        //     packed
        // }
    }


    #[test]
    fn test_unpack_uint8() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(Unpacker::new(&a).unpack_uint8().unwrap(), 1);
        assert_eq!(Unpacker::new(&a).unpack().expect("!"), Unpacked::Uint8(1));
    }

    #[test]
    fn test_unpack_int8() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(Unpacker::new(&a).unpack_int8().unwrap(), 1);
        let a = [255];
        assert_eq!(Unpacker::new(&a).unpack_int8().unwrap(), -1);
    }

    #[test]
    fn test_unpack_uint16() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(Unpacker::new(&a).unpack_uint16().unwrap(), 258);
    }

    #[test]
    fn test_unpack_uint32() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(Unpacker::new(&a).unpack_uint32().unwrap(), 16909060);
    }

    #[test]
    fn test_unpack_uint64() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(
            Unpacker::new(&a).unpack_uint64().unwrap(),
            72623859790382856
        );
    }

    #[test]
    fn test_unpack_raw() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(Unpacker::new(&a).unpack_raw(3).unwrap(), vec!(1, 2, 3));
    }

    #[test]
    fn test_unpack_string() {
        let a = [0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x21];
        assert_eq!(Unpacker::new(&a).unpack_string(a.len()).unwrap(), "hello world!");
    }

    #[test]
    fn test_unpack_array() {
        let a = [1, 2, 3, 4, 5];
        assert_eq!(
            Unpacker::new(&a).unpack_array(a.len()).unwrap(),
            vec!(Unpacked::Uint8(1), Unpacked::Uint8(2), Unpacked::Uint8(3), Unpacked::Uint8(4), Unpacked::Uint8(5)));
    }

    #[test]
    fn test_unpack_map() {
        let a = [1, 2, 3, 4];
        let mut expected = HashMap::new();
        expected.insert(Unpacked::Uint8(1), Unpacked::Uint8(2));
        expected.insert(Unpacked::Uint8(3), Unpacked::Uint8(4));
        assert_eq!(Unpacker::new(&a).unpack_map(a.len() / 2).unwrap(), expected);
    }

    #[test]
    fn test_unpack_float() {
        // 0b00111110001000000000000000000000 = 0.15625
        // source: https://en.wikipedia.org/wiki/Single-precision_floating-point_format
        let a = [0b00111110, 0b00100000, 0b00000000, 0b00000000];
        assert_eq!(Unpacker::new(&a).unpack_float().unwrap(), 0.15625);
    }

    #[test]
    fn test_unpack_double() {
        // src: https://en.wikipedia.org/wiki/Double-precision_floating-point_format
        let a = [0b00111111, 0b11010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101];
        assert_eq!(Unpacker::new(&a).unpack_double().unwrap(), 0.3333333333333333);
    }

    #[test]
    fn test_unpack() {
        let packed = [1];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Uint8(1));

        let packed = [1 ^ 0xe0];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Int8(-31));

        let packed = [2 ^ 0xa0, 1, 2];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Raw(vec!(1, 2)));

        let packed = [2 ^ 0xb0, 65, 66];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::String("AB".to_string()));

        let packed = [2 ^ 0x90, 2 ^ 0xb0, 65, 66, 1];
        let v = vec!(Unpacked::String("AB".to_string()), Unpacked::Uint8(1));
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Array(v));

        let packed = [2 ^ 0x80, 1 ^ 0xb0, 65, 1, 1 ^ 0xb0, 66, 2];
        let mut m = HashMap::new();
        m.insert(Unpacked::String("A".to_string()), Unpacked::Uint8(1));
        m.insert(Unpacked::String("B".to_string()), Unpacked::Uint8(2));
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Map(m));

        let packed = [0xc0];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Null);

        let packed = [0xc2];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Bool(false));

        let packed = [0xc3];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Bool(true));

        let packed = [0xca, 0b00111110, 0b00100000, 0b00000000, 0b00000000];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Float(0.15625));

        let packed =
            [0xcb, 0b00111111, 0b11010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Double(0.3333333333333333));

        let packed = [0xcc, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Uint8(255));

        let packed = [0xcd, 255, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Uint16(u16::max_value()));

        let packed = [0xce, 255, 255, 255, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Uint32(u32::max_value()));

        let packed = [0xcf, 255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Uint64(u64::max_value()));

        let packed = [0xd0, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Int8(-1));

        let packed = [0xd1, 255, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Int16(-1));

        let packed = [0xd2, 255, 255, 255, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Int32(-1));

        let packed = [0xd3, 255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Int64(-1));

        let packed = [0xd8, 0, 1, 65];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::String("A".to_string()));

        let packed = [0xc1];
        assert!(Unpacker::new(&packed).unpack().unwrap().is_undefined());
    }
}

// var BufferBuilder = require('./bufferbuilder').BufferBuilder;
// var binaryFeatures = require('./bufferbuilder').binaryFeatures;
//
// var BinaryPack = {
//   pack: function (data) {
//     var packer = new Packer();
//     packer.pack(data);
//     var buffer = packer.getBuffer();
//     return buffer;
//   }
// };
//
// module.exports = BinaryPack;
// function Packer () {
//   this.bufferBuilder = new BufferBuilder();
// }
//
// Packer.prototype.getBuffer = function () {
//   return this.bufferBuilder.getBuffer();
// };
//
// Packer.prototype.pack = function (value) {
//   var type = typeof (value);
//   if (type === 'string') {
//     this.pack_string(value);
//   } else if (type === 'number') {
//     if (Math.floor(value) === value) {
//       this.pack_integer(value);
//     } else {
//       this.pack_double(value);
//     }
//   } else if (type === 'boolean') {
//     if (value === true) {
//       this.bufferBuilder.append(0xc3);
//     } else if (value === false) {
//       this.bufferBuilder.append(0xc2);
//     }
//   } else if (type === 'undefined') {
//     this.bufferBuilder.append(0xc0);
//   } else if (type === 'object') {
//     if (value === null) {
//       this.bufferBuilder.append(0xc0);
//     } else {
//       var constructor = value.constructor;
//       if (constructor == Array) {
//         this.pack_array(value);
//       } else if (constructor == Blob || constructor == File || value instanceof Blob || value instanceof File) {
//         this.pack_bin(value);
//       } else if (constructor == ArrayBuffer) {
//         if (binaryFeatures.useArrayBufferView) {
//           this.pack_bin(new Uint8Array(value));
//         } else {
//           this.pack_bin(value);
//         }
//       } else if ('BYTES_PER_ELEMENT' in value) {
//         if (binaryFeatures.useArrayBufferView) {
//           this.pack_bin(new Uint8Array(value.buffer));
//         } else {
//           this.pack_bin(value.buffer);
//         }
//       } else if ((constructor == Object) || (constructor.toString().startsWith('class'))) {
//         this.pack_object(value);
//       } else if (constructor == Date) {
//         this.pack_string(value.toString());
//       } else if (typeof value.toBinaryPack === 'function') {
//         this.bufferBuilder.append(value.toBinaryPack());
//       } else {
//         throw new Error('Type "' + constructor.toString() + '" not yet supported');
//       }
//     }
//   } else {
//     throw new Error('Type "' + type + '" not yet supported');
//   }
//   this.bufferBuilder.flush();
// };
//
// Packer.prototype.pack_bin = function (blob) {
//   var length = blob.length || blob.byteLength || blob.size;
//   if (length <= 0x0f) {
//     this.pack_uint8(0xa0 + length);
//   } else if (length <= 0xffff) {
//     this.bufferBuilder.append(0xda);
//     this.pack_uint16(length);
//   } else if (length <= 0xffffffff) {
//     this.bufferBuilder.append(0xdb);
//     this.pack_uint32(length);
//   } else {
//     throw new Error('Invalid length');
//   }
//   this.bufferBuilder.append(blob);
// };
//
// Packer.prototype.pack_string = function (str) {
//   var length = utf8Length(str);
//
//   if (length <= 0x0f) {
//     this.pack_uint8(0xb0 + length);
//   } else if (length <= 0xffff) {
//     this.bufferBuilder.append(0xd8);
//     this.pack_uint16(length);
//   } else if (length <= 0xffffffff) {
//     this.bufferBuilder.append(0xd9);
//     this.pack_uint32(length);
//   } else {
//     throw new Error('Invalid length');
//   }
//   this.bufferBuilder.append(str);
// };
//
// Packer.prototype.pack_array = function (ary) {
//   var length = ary.length;
//   if (length <= 0x0f) {
//     this.pack_uint8(0x90 + length);
//   } else if (length <= 0xffff) {
//     this.bufferBuilder.append(0xdc);
//     this.pack_uint16(length);
//   } else if (length <= 0xffffffff) {
//     this.bufferBuilder.append(0xdd);
//     this.pack_uint32(length);
//   } else {
//     throw new Error('Invalid length');
//   }
//   for (var i = 0; i < length; i++) {
//     this.pack(ary[i]);
//   }
// };
//
// Packer.prototype.pack_integer = function (num) {
//   if (num >= -0x20 && num <= 0x7f) {
//     this.bufferBuilder.append(num & 0xff);
//   } else if (num >= 0x00 && num <= 0xff) {
//     this.bufferBuilder.append(0xcc);
//     this.pack_uint8(num);
//   } else if (num >= -0x80 && num <= 0x7f) {
//     this.bufferBuilder.append(0xd0);
//     this.pack_int8(num);
//   } else if (num >= 0x0000 && num <= 0xffff) {
//     this.bufferBuilder.append(0xcd);
//     this.pack_uint16(num);
//   } else if (num >= -0x8000 && num <= 0x7fff) {
//     this.bufferBuilder.append(0xd1);
//     this.pack_int16(num);
//   } else if (num >= 0x00000000 && num <= 0xffffffff) {
//     this.bufferBuilder.append(0xce);
//     this.pack_uint32(num);
//   } else if (num >= -0x80000000 && num <= 0x7fffffff) {
//     this.bufferBuilder.append(0xd2);
//     this.pack_int32(num);
//   } else if (num >= -0x8000000000000000 && num <= 0x7FFFFFFFFFFFFFFF) {
//     this.bufferBuilder.append(0xd3);
//     this.pack_int64(num);
//   } else if (num >= 0x0000000000000000 && num <= 0xFFFFFFFFFFFFFFFF) {
//     this.bufferBuilder.append(0xcf);
//     this.pack_uint64(num);
//   } else {
//     throw new Error('Invalid integer');
//   }
// };
//
// Packer.prototype.pack_double = function (num) {
//   var sign = 0;
//   if (num < 0) {
//     sign = 1;
//     num = -num;
//   }
//   var exp = Math.floor(Math.log(num) / Math.LN2);
//   var frac0 = num / Math.pow(2, exp) - 1;
//   var frac1 = Math.floor(frac0 * Math.pow(2, 52));
//   var b32 = Math.pow(2, 32);
//   var h32 = (sign << 31) | ((exp + 1023) << 20) |
//     (frac1 / b32) & 0x0fffff;
//   var l32 = frac1 % b32;
//   this.bufferBuilder.append(0xcb);
//   this.pack_int32(h32);
//   this.pack_int32(l32);
// };
//
// Packer.prototype.pack_object = function (obj) {
//   var keys = Object.keys(obj);
//   var length = keys.length;
//   if (length <= 0x0f) {
//     this.pack_uint8(0x80 + length);
//   } else if (length <= 0xffff) {
//     this.bufferBuilder.append(0xde);
//     this.pack_uint16(length);
//   } else if (length <= 0xffffffff) {
//     this.bufferBuilder.append(0xdf);
//     this.pack_uint32(length);
//   } else {
//     throw new Error('Invalid length');
//   }
//   for (var prop in obj) {
//     if (obj.hasOwnProperty(prop)) {
//       this.pack(prop);
//       this.pack(obj[prop]);
//     }
//   }
// };
//
// Packer.prototype.pack_uint8 = function (num) {
//   this.bufferBuilder.append(num);
// };
//
// Packer.prototype.pack_uint16 = function (num) {
//   this.bufferBuilder.append(num >> 8);
//   this.bufferBuilder.append(num & 0xff);
// };
//
// Packer.prototype.pack_uint32 = function (num) {
//   var n = num & 0xffffffff;
//   this.bufferBuilder.append((n & 0xff000000) >>> 24);
//   this.bufferBuilder.append((n & 0x00ff0000) >>> 16);
//   this.bufferBuilder.append((n & 0x0000ff00) >>> 8);
//   this.bufferBuilder.append((n & 0x000000ff));
// };
//
// Packer.prototype.pack_uint64 = function (num) {
//   var high = num / Math.pow(2, 32);
//   var low = num % Math.pow(2, 32);
//   this.bufferBuilder.append((high & 0xff000000) >>> 24);
//   this.bufferBuilder.append((high & 0x00ff0000) >>> 16);
//   this.bufferBuilder.append((high & 0x0000ff00) >>> 8);
//   this.bufferBuilder.append((high & 0x000000ff));
//   this.bufferBuilder.append((low & 0xff000000) >>> 24);
//   this.bufferBuilder.append((low & 0x00ff0000) >>> 16);
//   this.bufferBuilder.append((low & 0x0000ff00) >>> 8);
//   this.bufferBuilder.append((low & 0x000000ff));
// };
//
// Packer.prototype.pack_int8 = function (num) {
//   this.bufferBuilder.append(num & 0xff);
// };
//
// Packer.prototype.pack_int16 = function (num) {
//   this.bufferBuilder.append((num & 0xff00) >> 8);
//   this.bufferBuilder.append(num & 0xff);
// };
//
// Packer.prototype.pack_int32 = function (num) {
//   this.bufferBuilder.append((num >>> 24) & 0xff);
//   this.bufferBuilder.append((num & 0x00ff0000) >>> 16);
//   this.bufferBuilder.append((num & 0x0000ff00) >>> 8);
//   this.bufferBuilder.append((num & 0x000000ff));
// };
//
// Packer.prototype.pack_int64 = function (num) {
//   var high = Math.floor(num / Math.pow(2, 32));
//   var low = num % Math.pow(2, 32);
//   this.bufferBuilder.append((high & 0xff000000) >>> 24);
//   this.bufferBuilder.append((high & 0x00ff0000) >>> 16);
//   this.bufferBuilder.append((high & 0x0000ff00) >>> 8);
//   this.bufferBuilder.append((high & 0x000000ff));
//   this.bufferBuilder.append((low & 0xff000000) >>> 24);
//   this.bufferBuilder.append((low & 0x00ff0000) >>> 16);
//   this.bufferBuilder.append((low & 0x0000ff00) >>> 8);
//   this.bufferBuilder.append((low & 0x000000ff));
// };
//
// function _utf8Replace (m) {
//   var code = m.charCodeAt(0);
//
//   if (code <= 0x7ff) return '00';
//   if (code <= 0xffff) return '000';
//   if (code <= 0x1fffff) return '0000';
//   if (code <= 0x3ffffff) return '00000';
//   return '000000';
// }
//
// function utf8Length (str) {
//   if (str.length > 600) {
//     // Blob method faster for large strings
//     return (new Blob([str])).size;
//   } else {
//     return str.replace(/[^\u0000-\u007F]/g, _utf8Replace).length;
//   }
// }
