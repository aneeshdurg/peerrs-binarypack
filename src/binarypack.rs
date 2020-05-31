use std::boxed::Box;
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
    Double(f32),
    String(String),
    Null,
    Array(Box<[Unpacked]>),
    Map(HashMap<Unpacked, Unpacked>),
}

impl PartialEq for Unpacked {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
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

    fn unpack_uint16(&mut self) -> Result<u16> {
        self.unpack_unsigned()
    }

    fn unpack_uint32(&mut self) -> Result<u32> {
        self.unpack_unsigned()
    }

    fn unpack_uint64(&mut self) -> Result<u64> {
        self.unpack_unsigned()
    }

    fn unpack(&mut self) -> Result<Unpacked> {
        let type_ = self.unpack_uint8()?;
        if type_ < 0x80 {
            return Ok(Unpacked::Uint8(type_));
        } else if (type_ ^ 0xe0) < 0x20 {
            return Ok(Unpacked::Int8((type_ ^ 0xe0) as i8 - 0x20));
        }

        panic!("!")
    }
}

pub fn unpack(data: &[u8]) -> Unpacked {
    Unpacker::new(data).unpack().unwrap()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unpack_uint8() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(Unpacker::new(&a).unpack_uint8().unwrap(), 1);
        assert_eq!(Unpacker::new(&a).unpack().expect("!"), Unpacked::Uint8(1));
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
}

// var BufferBuilder = require('./bufferbuilder').BufferBuilder;
// var binaryFeatures = require('./bufferbuilder').binaryFeatures;
//
// var BinaryPack = {
//   unpack: function (data) {
//     var unpacker = new Unpacker(data);
//     return unpacker.unpack();
//   },
//   pack: function (data) {
//     var packer = new Packer();
//     packer.pack(data);
//     var buffer = packer.getBuffer();
//     return buffer;
//   }
// };
//
// module.exports = BinaryPack;
//
// function Unpacker (data) {
//   // Data is ArrayBuffer
//   this.index = 0;
//   this.dataBuffer = data;
//   this.dataView = new Uint8Array(this.dataBuffer);
//   this.length = this.dataBuffer.byteLength;
// }
//
// Unpacker.prototype.unpack = function () {
//   var type = this.unpack_uint8();
//   if (type < 0x80) {
//     return type;
//   } else if ((type ^ 0xe0) < 0x20) {
//     return (type ^ 0xe0) - 0x20;
//   }
//
//   var size;
//   if ((size = type ^ 0xa0) <= 0x0f) {
//     return this.unpack_raw(size);
//   } else if ((size = type ^ 0xb0) <= 0x0f) {
//     return this.unpack_string(size);
//   } else if ((size = type ^ 0x90) <= 0x0f) {
//     return this.unpack_array(size);
//   } else if ((size = type ^ 0x80) <= 0x0f) {
//     return this.unpack_map(size);
//   }
//
//   switch (type) {
//     case 0xc0:
//       return null;
//     case 0xc1:
//       return undefined;
//     case 0xc2:
//       return false;
//     case 0xc3:
//       return true;
//     case 0xca:
//       return this.unpack_float();
//     case 0xcb:
//       return this.unpack_double();
//     case 0xcc:
//       return this.unpack_uint8();
//     case 0xcd:
//       return this.unpack_uint16();
//     case 0xce:
//       return this.unpack_uint32();
//     case 0xcf:
//       return this.unpack_uint64();
//     case 0xd0:
//       return this.unpack_int8();
//     case 0xd1:
//       return this.unpack_int16();
//     case 0xd2:
//       return this.unpack_int32();
//     case 0xd3:
//       return this.unpack_int64();
//     case 0xd4:
//       return undefined;
//     case 0xd5:
//       return undefined;
//     case 0xd6:
//       return undefined;
//     case 0xd7:
//       return undefined;
//     case 0xd8:
//       size = this.unpack_uint16();
//       return this.unpack_string(size);
//     case 0xd9:
//       size = this.unpack_uint32();
//       return this.unpack_string(size);
//     case 0xda:
//       size = this.unpack_uint16();
//       return this.unpack_raw(size);
//     case 0xdb:
//       size = this.unpack_uint32();
//       return this.unpack_raw(size);
//     case 0xdc:
//       size = this.unpack_uint16();
//       return this.unpack_array(size);
//     case 0xdd:
//       size = this.unpack_uint32();
//       return this.unpack_array(size);
//     case 0xde:
//       size = this.unpack_uint16();
//       return this.unpack_map(size);
//     case 0xdf:
//       size = this.unpack_uint32();
//       return this.unpack_map(size);
//   }
// };
//
// Unpacker.prototype.unpack_int8 = function () {
//   var uint8 = this.unpack_uint8();
//   return (uint8 < 0x80) ? uint8 : uint8 - (1 << 8);
// };
//
// Unpacker.prototype.unpack_int16 = function () {
//   var uint16 = this.unpack_uint16();
//   return (uint16 < 0x8000) ? uint16 : uint16 - (1 << 16);
// };
//
// Unpacker.prototype.unpack_int32 = function () {
//   var uint32 = this.unpack_uint32();
//   return (uint32 < Math.pow(2, 31)) ? uint32
//     : uint32 - Math.pow(2, 32);
// };
//
// Unpacker.prototype.unpack_int64 = function () {
//   var uint64 = this.unpack_uint64();
//   return (uint64 < Math.pow(2, 63)) ? uint64
//     : uint64 - Math.pow(2, 64);
// };
//
// Unpacker.prototype.unpack_raw = function (size) {
//   if (this.length < this.index + size) {
//     throw new Error('BinaryPackFailure: index is out of range' +
//       ' ' + this.index + ' ' + size + ' ' + this.length);
//   }
//   var buf = this.dataBuffer.slice(this.index, this.index + size);
//   this.index += size;
//
//   // buf = util.bufferToString(buf);
//
//   return buf;
// };
//
// Unpacker.prototype.unpack_string = function (size) {
//   var bytes = this.read(size);
//   var i = 0;
//   var str = '';
//   var c;
//   var code;
//
//   while (i < size) {
//     c = bytes[i];
//     if (c < 128) {
//       str += String.fromCharCode(c);
//       i++;
//     } else if ((c ^ 0xc0) < 32) {
//       code = ((c ^ 0xc0) << 6) | (bytes[i + 1] & 63);
//       str += String.fromCharCode(code);
//       i += 2;
//     } else {
//       code = ((c & 15) << 12) | ((bytes[i + 1] & 63) << 6) |
//         (bytes[i + 2] & 63);
//       str += String.fromCharCode(code);
//       i += 3;
//     }
//   }
//
//   this.index += size;
//   return str;
// };
//
// Unpacker.prototype.unpack_array = function (size) {
//   var objects = new Array(size);
//   for (var i = 0; i < size; i++) {
//     objects[i] = this.unpack();
//   }
//   return objects;
// };
//
// Unpacker.prototype.unpack_map = function (size) {
//   var map = {};
//   for (var i = 0; i < size; i++) {
//     var key = this.unpack();
//     var value = this.unpack();
//     map[key] = value;
//   }
//   return map;
// };
//
// Unpacker.prototype.unpack_float = function () {
//   var uint32 = this.unpack_uint32();
//   var sign = uint32 >> 31;
//   var exp = ((uint32 >> 23) & 0xff) - 127;
//   var fraction = (uint32 & 0x7fffff) | 0x800000;
//   return (sign === 0 ? 1 : -1) *
//     fraction * Math.pow(2, exp - 23);
// };
//
// Unpacker.prototype.unpack_double = function () {
//   var h32 = this.unpack_uint32();
//   var l32 = this.unpack_uint32();
//   var sign = h32 >> 31;
//   var exp = ((h32 >> 20) & 0x7ff) - 1023;
//   var hfrac = (h32 & 0xfffff) | 0x100000;
//   var frac = hfrac * Math.pow(2, exp - 20) +
//     l32 * Math.pow(2, exp - 52);
//   return (sign === 0 ? 1 : -1) * frac;
// };
//
// Unpacker.prototype.read = function (length) {
//   var j = this.index;
//   if (j + length <= this.length) {
//     return this.dataView.subarray(j, j + length);
//   } else {
//     throw new Error('BinaryPackFailure: read index out of range');
//   }
// };
//
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
