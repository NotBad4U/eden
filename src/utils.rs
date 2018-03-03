#![allow(dead_code)]
/// NOTE: The purpose of this methods is to avoid to use `std::mem::transmute` which is unsafe.

/// Transform in BIG ENDIAN the usize to an array of [u8; 8]
fn usize_to_array_of_u8(v: usize) -> [u8; 8] {
    u64_to_array_of_u8(v as u64)
}

/// Transform in BIG ENDIAN the u64 to an array of [u8; 8]
fn u64_to_array_of_u8(v: u64) -> [u8; 8] {
    let b1 : u8 = ((v >> 56) & 0xff) as u8;
    let b2 : u8 = ((v >> 48) & 0xff) as u8;
    let b3 : u8 = ((v >> 40) & 0xff) as u8;
    let b4 : u8 = ((v >> 32) & 0xff) as u8;
    let b5 : u8 = ((v >> 24) & 0xff) as u8;
    let b6 : u8 = ((v >> 16) & 0xff) as u8;
    let b7 : u8 = ((v >> 8) & 0xff) as u8;
    let b8 : u8 = (v & 0xff) as u8;
    return [b8, b7, b6, b5, b4, b3, b2, b1]
}

/// Transform in BIG ENDIAN the u32 to an array of [u8; 8]
fn u32_to_array_of_u8(x:u32) -> [u8; 4] {
    let b1 : u8 = ((x >> 24) & 0xff) as u8;
    let b2 : u8 = ((x >> 16) & 0xff) as u8;
    let b3 : u8 = ((x >> 8) & 0xff) as u8;
    let b4 : u8 = (x & 0xff) as u8;
    return [b4, b3, b2, b1]
}

/// Transform in BIG ENDIAN the u16 to an array of [u8; 2]
fn u16_to_array_of_u8(x:u32) -> [u8; 2] {
    let b1 : u8 = ((x >> 8) & 0xff) as u8;
    let b2 : u8 = (x & 0xff) as u8;
    return [b2, b1]
}

/// Transform a BIG ENDIAN u8 slice to a u64
fn array_of_u8_to_usize(val: [u8; 8]) -> usize {
    array_of_u8_to_u64(val) as usize
}

/// Transform a BIG ENDIAN u8 slice to a u64
fn array_of_u8_to_u64(val: [u8; 8]) -> u64 {
    let mut s = 0;
    s = s | (val[0] as u64);
    s = s | ((val[1] as u64) << 8);
    s = s | ((val[2] as u64) << 16);
    s = s | ((val[2] as u64) << 24);
    s = s | ((val[2] as u64) << 32);
    s = s | ((val[2] as u64) << 40);
    s = s | ((val[2] as u64) << 48);
    s = s | ((val[2] as u64) << 56);
    s
}

/// Transform a BIG ENDIAN u8 slice to a u32
fn array_of_u8_to_u32(val: [u8; 4]) -> u32 {
    let mut s = 0;
    s = s | (val[0] as u32);
    s = s | ((val[1] as u32) << 8);
    s = s | ((val[2] as u32) << 16);
    s = s | ((val[2] as u32) << 24);
    s
}

/// Transform a BIG ENDIAN u8 slice to a u16
fn array_of_u8_to_16(val: [u8; 2]) -> u16 {
    let mut s = 0;
    s = s | (val[0] as u16);
    s = s | ((val[1] as u16) << 8);
    s
}

#[cfg(test)]
mod test_transform {

    use super::*;

    #[test]
    fn test_transform_u64_to_array_of_u8() {
        let v = 18446744073709550415;
        let expected = [79, 251, 255, 255, 255, 255, 255, 255];

        assert_eq!(expected, u64_to_array_of_u8(v));
    }

    #[test]
    fn test_transform_u32_to_array_of_u8() {
        let v = 4294966095;
        let expected = [79, 251, 255, 255];

        assert_eq!(expected, u32_to_array_of_u8(v));
    }

    #[test]
    fn test_transform_u16_to_array_of_u8() {
        let v = 65335;
        let expected = [55, 255];

        assert_eq!(expected, u16_to_array_of_u8(v));
    }

    #[test]
    fn test_transform_array_of_u8_to_u64() {
        let v = [79, 251, 255, 255, 255, 255, 255, 255];
        let expected = 18446744073709550415;

        assert_eq!(expected, array_of_u8_to_u64(v));
    }

    #[test]
    fn test_transform_array_of_u8_to_u32() {
        let v = [79, 251, 255, 255];
        let expected = 4294966095;

        assert_eq!(expected, array_of_u8_to_u32(v));
    }

    #[test]
    fn test_transform_array_of_u8_to_u16() {
        let v = [55, 255];
        let expected = 65335;

        assert_eq!(expected, array_of_u8_to_16(v));
    }
}