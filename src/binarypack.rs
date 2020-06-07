use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::size_of;

use byteorder::{BigEndian, ByteOrder};
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

const MAP_MASK: u8 = 0x80;
const ARR_MASK: u8 = 0x90;
const RAW_MASK: u8 = 0xa0;
const STR_MASK: u8 = 0xb0;
const INT_MASK: u8 = 0xe0;

const PACKED_NULL: u8 = 0xc0;
const PACKED_UNDEFINED: u8 = 0xc1;
const PACKED_FALSE: u8 = 0xc2;
const PACKED_TRUE: u8 = 0xc3;
const PACKED_FLOAT: u8 = 0xca;
const PACKED_DOUBLE: u8 = 0xcb;
const PACKED_UINT8: u8 = 0xcc;
const PACKED_UINT16: u8 = 0xcd;
const PACKED_UINT32: u8 = 0xce;
const PACKED_UINT64: u8 = 0xcf;
const PACKED_INT8: u8 = 0xd0;
const PACKED_INT16: u8 = 0xd1;
const PACKED_INT32: u8 = 0xd2;
const PACKED_INT64: u8 = 0xd3;
const PACKED_STR_U16: u8 = 0xd8;
const PACKED_STR_U32: u8 = 0xd9;
const PACKED_RAW_U16: u8 = 0xda;
const PACKED_RAW_U32: u8 = 0xdb;
const PACKED_ARR_U16: u8 = 0xdc;
const PACKED_ARR_U32: u8 = 0xdd;
const PACKED_MAP_U16: u8 = 0xde;
const PACKED_MAP_U32: u8 = 0xdf;

pub struct Unpacker<'a> {
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
        let mut raw = vec![];
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
        let mut arr = vec![];
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
        let i = self.unpack_uint32()?;
        let mut bytes = [0u8; 4];
        BigEndian::write_u32(&mut bytes, i);
        Ok(BigEndian::read_f32(&mut bytes))
    }

    fn unpack_double(&mut self) -> Result<f64> {
        let i = self.unpack_uint64()?;
        let mut bytes = [0u8; 8];
        BigEndian::write_u64(&mut bytes, i);
        Ok(BigEndian::read_f64(&mut bytes))
    }

    fn unpack(&mut self) -> Result<Unpacked> {
        let type_ = self.unpack_uint8()?;
        if type_ < MAP_MASK {
            return Ok(Unpacked::Uint8(type_));
        } else if (type_ ^ INT_MASK) < 0x20 {
            return Ok(Unpacked::Int8((type_ ^ INT_MASK) as i8 - 0x20));
        }

        let size = type_ ^ MAP_MASK;
        if size <= 0x0f {
            return Ok(Unpacked::Map(self.unpack_map(size as usize)?));
        }

        let size = type_ ^ ARR_MASK;
        if size <= 0x0f {
            return Ok(Unpacked::Array(self.unpack_array(size as usize)?));
        }

        let size = type_ ^ RAW_MASK;
        if size <= 0x0f {
            return Ok(Unpacked::Raw(self.unpack_raw(size as usize)?));
        }
        let size = type_ ^ STR_MASK;
        if size <= 0x0f {
            return Ok(Unpacked::String(self.unpack_string(size as usize)?));
        }

        Ok(match type_ {
            PACKED_NULL => Unpacked::Null,
            PACKED_FALSE => Unpacked::Bool(false),
            PACKED_TRUE => Unpacked::Bool(true),
            PACKED_FLOAT => Unpacked::Float(self.unpack_float()?),
            PACKED_DOUBLE => Unpacked::Double(self.unpack_double()?),
            PACKED_UINT8 => Unpacked::Uint8(self.unpack_uint8()?),
            PACKED_UINT16 => Unpacked::Uint16(self.unpack_uint16()?),
            PACKED_UINT32 => Unpacked::Uint32(self.unpack_uint32()?),
            PACKED_UINT64 => Unpacked::Uint64(self.unpack_uint64()?),
            PACKED_INT8 => Unpacked::Int8(self.unpack_int8()?),
            PACKED_INT16 => Unpacked::Int16(self.unpack_int16()?),
            PACKED_INT32 => Unpacked::Int32(self.unpack_int32()?),
            PACKED_INT64 => Unpacked::Int64(self.unpack_int64()?),
            PACKED_STR_U16 => {
                let size = self.unpack_uint16()? as usize;
                Unpacked::String(self.unpack_string(size)?)
            }
            PACKED_STR_U32 => {
                let size = self.unpack_uint32()? as usize;
                Unpacked::String(self.unpack_string(size)?)
            }
            PACKED_RAW_U16 => {
                let size = self.unpack_uint16()? as usize;
                Unpacked::Raw(self.unpack_raw(size)?)
            }
            PACKED_RAW_U32 => {
                let size = self.unpack_uint32()? as usize;
                Unpacked::Raw(self.unpack_raw(size)?)
            }
            PACKED_ARR_U16 => {
                let size = self.unpack_uint16()? as usize;
                Unpacked::Array(self.unpack_array(size)?)
            }
            PACKED_ARR_U32 => {
                let size = self.unpack_uint32()? as usize;
                Unpacked::Array(self.unpack_array(size)?)
            }
            PACKED_MAP_U16 => {
                let size = self.unpack_uint16()? as usize;
                Unpacked::Map(self.unpack_map(size)?)
            }
            PACKED_MAP_U32 => {
                let size = self.unpack_uint32()? as usize;
                Unpacked::Map(self.unpack_map(size)?)
            }

            _ => Unpacked::Undefined,
        })
    }
}

pub fn unpack(data: &[u8]) -> Result<Unpacked> {
    Unpacker::new(data).unpack()
}

impl Unpacked {
    fn _pack_len(packed: &mut Vec<u8>, size: usize, u16_type: u8, u32_type: u8) {
        if size <= (u16::max_value() as usize) {
            packed.push(u16_type);

            let mut size_bytes = [0u8; 2];
            BigEndian::write_u16(&mut size_bytes, size as u16);
            for b in size_bytes.iter() {
                packed.push(*b);
            }
        } else {
            packed.push(u32_type);

            let mut size_bytes = [0u8; 4];
            BigEndian::write_u32(&mut size_bytes, size as u32);
            for b in size_bytes.iter() {
                packed.push(*b);
            }
        }
    }

    fn _pack(&self, packed: &mut Vec<u8>) {
        match self {
            Unpacked::Uint8(a) => {
                if *a < MAP_MASK {
                    packed.push(*a);
                } else {
                    packed.push(PACKED_UINT8);
                    packed.push(*a);
                }
            }
            Unpacked::Uint16(a) => {
                packed.push(PACKED_UINT16);
                let mut bytes = [0u8; 2];
                BigEndian::write_u16(&mut bytes, *a);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Uint32(a) => {
                packed.push(PACKED_UINT32);
                let mut bytes = [0u8; 4];
                BigEndian::write_u32(&mut bytes, *a);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Uint64(a) => {
                packed.push(PACKED_UINT64);
                let mut bytes = [0u8; 8];
                BigEndian::write_u64(&mut bytes, *a);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Int8(a) => {
                if *a < 0 && *a > -0x20 {
                    packed.push(((a + 0x20) as u8) ^ INT_MASK)
                } else {
                    packed.push(PACKED_INT8);
                    packed.push(*a as u8);
                }
            }
            Unpacked::Int16(a) => {
                packed.push(PACKED_INT16);
                let mut bytes = [0u8; 2];
                BigEndian::write_u16(&mut bytes, *a as u16);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Int32(a) => {
                packed.push(PACKED_INT32);
                let mut bytes = [0u8; 4];
                BigEndian::write_u32(&mut bytes, *a as u32);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Int64(a) => {
                packed.push(PACKED_INT64);
                let mut bytes = [0u8; 8];
                BigEndian::write_u64(&mut bytes, *a as u64);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Float(f) => {
                let mut bytes = [0u8; 4];
                BigEndian::write_f32(&mut bytes, *f);

                packed.push(PACKED_FLOAT);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Double(f) => {
                let mut bytes = [0u8; 8];
                BigEndian::write_f64(&mut bytes, *f);

                packed.push(PACKED_DOUBLE);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::Bool(b) => {
                match b {
                    true => packed.push(PACKED_TRUE),
                    false => packed.push(PACKED_FALSE),
                };
            }
            Unpacked::Raw(bytes) => {
                Unpacked::_pack_len(packed, bytes.len(), PACKED_RAW_U16, PACKED_RAW_U32);
                for b in bytes.iter() {
                    packed.push(*b);
                }
            }
            Unpacked::String(s) => {
                let bytes = s.bytes();
                Unpacked::_pack_len(packed, bytes.len(), PACKED_STR_U16, PACKED_STR_U32);
                for b in bytes {
                    packed.push(b);
                }
            }
            Unpacked::Null => {
                packed.push(PACKED_NULL);
            }
            Unpacked::Undefined => packed.push(PACKED_UNDEFINED),
            Unpacked::Array(v) => {
                Unpacked::_pack_len(packed, v.len(), PACKED_ARR_U16, PACKED_ARR_U32);
                for element in v {
                    element._pack(packed);
                }
            }
            Unpacked::Map(m) => {
                Unpacked::_pack_len(packed, m.len(), PACKED_MAP_U16, PACKED_MAP_U32);
                for (key, value) in m {
                    key._pack(packed);
                    value._pack(packed);
                }
            }
        }
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut packed = vec![];
        self._pack(&mut packed);
        packed
    }
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
        let a = [
            0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x21,
        ];
        assert_eq!(
            Unpacker::new(&a).unpack_string(a.len()).unwrap(),
            "hello world!"
        );
    }

    #[test]
    fn test_unpack_array() {
        let a = [1, 2, 3, 4, 5];
        assert_eq!(
            Unpacker::new(&a).unpack_array(a.len()).unwrap(),
            vec!(
                Unpacked::Uint8(1),
                Unpacked::Uint8(2),
                Unpacked::Uint8(3),
                Unpacked::Uint8(4),
                Unpacked::Uint8(5)
            )
        );
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
        let a = [
            0b00111111, 0b11010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101,
            0b01010101,
        ];
        assert_eq!(
            Unpacker::new(&a).unpack_double().unwrap(),
            0.3333333333333333
        );
    }

    #[test]
    fn test_unpack() {
        let packed = [1];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Uint8(1));

        let packed = [1 ^ 0xe0];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Int8(-31)
        );

        let packed = [2 ^ 0xa0, 1, 2];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Raw(vec!(1, 2))
        );

        let packed = [2 ^ 0xb0, 65, 66];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::String("AB".to_string())
        );

        let packed = [2 ^ 0x90, 2 ^ 0xb0, 65, 66, 1];
        let v = vec![Unpacked::String("AB".to_string()), Unpacked::Uint8(1)];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Array(v));

        let packed = [2 ^ 0x80, 1 ^ 0xb0, 65, 1, 1 ^ 0xb0, 66, 2];
        let mut m = HashMap::new();
        m.insert(Unpacked::String("A".to_string()), Unpacked::Uint8(1));
        m.insert(Unpacked::String("B".to_string()), Unpacked::Uint8(2));
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Map(m));

        let packed = [0xc0];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Null);

        let packed = [0xc2];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Bool(false)
        );

        let packed = [0xc3];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Bool(true)
        );

        let packed = [0xca, 0b00111110, 0b00100000, 0b00000000, 0b00000000];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Float(0.15625)
        );

        let packed = [
            0xcb, 0b00111111, 0b11010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101,
            0b01010101, 0b01010101,
        ];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Double(0.3333333333333333)
        );

        let packed = [0xcc, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Uint8(255)
        );

        let packed = [0xcd, 255, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Uint16(u16::max_value())
        );

        let packed = [0xce, 255, 255, 255, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Uint32(u32::max_value())
        );

        let packed = [0xcf, 255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Uint64(u64::max_value())
        );

        let packed = [0xd0, 255];
        assert_eq!(Unpacker::new(&packed).unpack().unwrap(), Unpacked::Int8(-1));

        let packed = [0xd1, 255, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Int16(-1)
        );

        let packed = [0xd2, 255, 255, 255, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Int32(-1)
        );

        let packed = [0xd3, 255, 255, 255, 255, 255, 255, 255, 255];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::Int64(-1)
        );

        let packed = [0xd8, 0, 1, 65];
        assert_eq!(
            Unpacker::new(&packed).unpack().unwrap(),
            Unpacked::String("A".to_string())
        );

        let packed = [0xc1];
        assert!(Unpacker::new(&packed).unpack().unwrap().is_undefined());
    }

    #[test]
    fn pack_uint8() {
        assert_eq!(Unpacked::Uint8(0x79).pack(), vec!(0x79));
        assert_eq!(Unpacked::Uint8(0x80).pack(), vec!(0xcc, 0x80));

        for i in 0..u8::max_value() {
            let expected = Unpacked::Uint8(i);
            assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
        }
    }

    #[test]
    fn pack_uint16() {
        assert_eq!(Unpacked::Uint16(258).pack(), vec!(0xcd, 0x1, 0x2));

        for i in 0..u16::max_value() {
            let expected = Unpacked::Uint16(i);
            assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
        }
    }

    #[test]
    fn pack_uint32() {
        assert_eq!(
            Unpacked::Uint32(16909060).pack(),
            vec!(0xce, 0x1, 0x2, 0x3, 0x4)
        );

        let expected = Unpacked::Uint32(16909060);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_uint64() {
        assert_eq!(
            Unpacked::Uint64(72623859790382856).pack(),
            vec!(0xcf, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8)
        );

        let expected = Unpacked::Uint64(72623859790382856);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_int8() {
        assert_eq!(Unpacked::Int8(-31).pack(), vec!(0xe1));
        assert_eq!(Unpacked::Int8(-100).pack(), vec!(0xd0, 0x9c));

        for i in 0..u8::max_value() {
            let expected = Unpacked::Int8(i as i8);
            assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
        }
    }

    #[test]
    fn pack_int16() {
        for i in 0..u16::max_value() {
            let expected = Unpacked::Int16(i as i16);
            assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
        }
    }

    #[test]
    fn pack_int32() {
        for i in 0..u16::max_value() {
            let expected = Unpacked::Int32(-(i as i32));
            assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
        }
    }

    #[test]
    fn pack_int64() {
        for i in 0..u16::max_value() {
            let expected = Unpacked::Int64(-(i as i64));
            assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
        }
    }

    #[test]
    fn pack_float() {
        assert_eq!(
            Unpacked::Float(0.15625).pack(),
            vec!(0xca, 0b00111110, 0b00100000, 0b00000000, 0b00000000)
        );

        let expected = Unpacked::Float(0.15625);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_double() {
        assert_eq!(
            Unpacked::Double(0.3333333333333333).pack(),
            vec!(
                0xcb, 0b00111111, 0b11010101, 0b01010101, 0b01010101, 0b01010101, 0b01010101,
                0b01010101, 0b01010101
            )
        );

        let expected = Unpacked::Double(0.3333333333333333);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_bool() {
        let expected = Unpacked::Bool(true);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);

        let expected = Unpacked::Bool(false);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_raw() {
        let mut raw_vec = vec![];
        for _ in 0..u16::max_value() {
            raw_vec.push(0);
        }
        let expected = Unpacked::Raw(raw_vec);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);

        let mut raw_vec = vec![];
        for _ in 0..u16::max_value() {
            raw_vec.push(0);
        }
        raw_vec.push(0);
        let expected = Unpacked::Raw(raw_vec);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_str() {
        let mut s = String::from("");
        for _ in 0..u16::max_value() {
            s.push('a');
        }
        let expected = Unpacked::String(s);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);

        let mut s = String::from("");
        for _ in 0..u16::max_value() {
            s.push('a');
        }
        s.push('a');
        let expected = Unpacked::String(s);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_arr() {
        let mut v = vec![];
        for _ in 0..u16::max_value() {
            v.push(Unpacked::Null);
        }
        let expected = Unpacked::Array(v);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);

        let mut v = vec![];
        for _ in 0..u16::max_value() {
            v.push(Unpacked::Null);
        }
        v.push(Unpacked::Null);
        let expected = Unpacked::Array(v);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_map() {
        let mut m = HashMap::new();
        for i in 0..u16::max_value() {
            m.insert(Unpacked::Uint32(i as u32), Unpacked::Null);
        }
        let expected = Unpacked::Map(m);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);

        let mut m = HashMap::new();
        for i in 0..u16::max_value() {
            m.insert(Unpacked::Uint32(i as u32), Unpacked::Null);
        }
        m.insert(Unpacked::Uint32(u16::max_value as u32 + 1), Unpacked::Null);
        let expected = Unpacked::Map(m);
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_null() {
        let expected = Unpacked::Null;
        assert_eq!(Unpacker::new(&expected.pack()).unpack().unwrap(), expected);
    }

    #[test]
    fn pack_undefined() {
        let expected = Unpacked::Undefined;
        assert!(Unpacker::new(&expected.pack())
            .unpack()
            .unwrap()
            .is_undefined());
    }
}
