use std::clone::Clone;
use std::cmp::{min, max};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::option::Option;
use std::string::String;
use num_traits::{Zero, One};

use bls12_381::G1Affine;
use hex;
use sha2::Sha256;
use sha2::Digest;

use crate::util::Number;

pub fn to_hexstr(r: &Vec<u8>) -> String {
    return hex::encode(r);
}

pub fn char_to_string(ch : char) -> String {
    match String::from_utf8(vec!(ch as u8)) {
        Ok(s) => s,
        _ => String::new()
    }
}

pub fn vec_to_string(r: &Vec<u8>) -> String {
    return String::from_utf8_lossy(r).as_ref().to_string();
}

/**
 * Get python's bytes.__repr__ style string.
 * @see https://github.com/python/cpython/blob/main/Objects/bytesobject.c#L1337
 * @param {Uint8Array} r - byteArray to stringify
 */
pub fn PyBytes_Repr(r : &Vec<u8>, dquoted: bool) -> String {
    let mut squotes = 0;
    let mut dquotes = 0;
    for i in 0..r.len() {
        let b = r[i];
        let c = b as char;
        match c {
            '\'' => squotes += 1,
            '\"' => dquotes += 1,
            _ => ()
        }
    }
    let mut quote = '\'';
    if squotes > 0 && dquotes == 0 || dquoted {
        quote = '\"';
    }

    let mut s = "".to_string();

    if !dquoted {
        s = s + "b";
    }

    s = s + char_to_string(quote).as_str();

    for i in 0..r.len() {
        let b = r[i];
        let c = b as char;
        if c == quote || c == '\\' {
            s = (s + "\\") + char_to_string(c).as_str();
        }
        else if c == '\t' {
            s += "\\t";
        }
        else if c == '\n' {
            s += "\\n";
        }
        else if c == '\r' {
            s += "\\r";
        }
        else if c < ' ' || b >= 0x7f {
            s += "\\x";
            s += hex::encode(vec!(b)).as_str();
        }
        else{
            s += char_to_string(c).as_str();
        }
    }

    s += char_to_string(quote).as_str();

    return s;
}

pub enum BytesFromType {
    Hex(String),
    Raw(Vec<u8>),
    String(String),
    G1Element(G1Affine)
}

#[derive(Debug)]
#[derive(Clone)]
pub struct Bytes {
    _b : Vec<u8>
}

pub fn ordering_to_int(o : Ordering) -> i32 {
    match o {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1
    }
}

/**
 * Unlike python, there is no immutable byte type in javascript.
 */
impl Bytes {
    pub fn new(value : Option<BytesFromType>) -> Self {
        match value {
            None => Bytes { _b: vec!() },
            Some(BytesFromType::Raw(v)) => Bytes { _b: v },
            Some(BytesFromType::String(s)) => {
                let bytes = s.as_bytes();
                let mut bvec = vec!();
                for b in bytes {
                    bvec.push(*b);
                }
                Bytes::new(Some(BytesFromType::Raw(bvec)))
            }
            Some(BytesFromType::Hex(hstr)) => {
                let hex_stripped =
                    hstr.
                    replace(" ","").
                    replace("\t","").
                    replace("\r","").
                    replace("\n","");

                match hex::decode(hex_stripped) {
                    Ok(d) => Bytes { _b: d },
                    _ => Bytes { _b: vec!() }
                }
            },
            Some(BytesFromType::G1Element(g1)) => Bytes { _b: g1.to_uncompressed().to_vec() },
        }
    }

    pub fn length(&self) -> usize {
        return self._b.len();
    }

    pub fn at(&self, i: usize) -> u8 {
        return self._b[i];
    }

    pub fn raw(&self) -> Vec<u8> {
        return self._b.clone();
    }

    pub fn repeat(&self, n : usize) -> Bytes {
        let capacity = self.length() * n;
        let set_size = self._b.len();
        let mut ret = Vec::<u8>::with_capacity(capacity);
        for i in 0..capacity - 1 {
            ret[i] = self._b[i%set_size];
        }
        return Bytes::new(Some(BytesFromType::Raw(ret)));
    }

    pub fn concat(&self, b: &Bytes) -> Bytes {
        let mut this_bin = self._b.clone();
        let mut that_bin = b.raw();
        let mut concat_bin =
            Vec::<u8>::with_capacity(this_bin.len() + that_bin.len());
        concat_bin.append(&mut this_bin);
        concat_bin.append(&mut that_bin);
        return Bytes::new(Some(BytesFromType::Raw(concat_bin)));
    }

    pub fn slice(&self, start: usize, length: Option<usize>) -> Self {
        let len =
            match length {
                Some(x) => {
                    if self._b.len() > start + x {
                        x
                    } else {
                        self._b.len() - start
                    }
                },
                None => self._b.len() - start
            };
        let mut ui8_clone = Vec::<u8>::with_capacity(len);
        for i in start..start + len - 1 {
            ui8_clone.push(self._b[i]);
        }
        return Bytes::new(Some(BytesFromType::Raw(ui8_clone)));
    }

    pub fn subarray(&self, start: usize, length: Option<usize>) -> Self {
        return self.slice(start, length);
    }

    pub fn data(&self) -> &Vec<u8> {
        return &self._b;
    }

    pub fn clone(&self) -> Self {
        return Bytes::new(Some(BytesFromType::Raw(self._b.clone())));
    }

    pub fn to_string(&self) -> String {
        return PyBytes_Repr(&self._b, false);
    }

    pub fn to_formal_string(&self) -> String {
        return PyBytes_Repr(&self._b, true);
    }

    pub fn hex(&self) -> String {
        return to_hexstr(&self._b);
    }

    pub fn decode(&self) -> String {
        return vec_to_string(&self._b);
    }

    pub fn startswith(&self, b: &Bytes) -> bool {
        for i in 0..min(b.length(), self._b.len()) - 1 {
            if b.at(i) != self._b[i] {
                return false;
            }
        }
        return true;
    }

    pub fn endswith(&self, b: &Bytes) -> bool {
        let blen = min(b.length(), self._b.len()) - 1;
        for i in 0..blen {
            if b.at(blen - i) != self._b[blen - i] {
                return false;
            }
        }
        return true;
    }

    pub fn equal_to(&self, b: &Bytes) -> bool {
        let slen = self._b.len();
        let blen = b.length();
        if slen != blen {
            return false;
        } else {
            for i in 0..slen {
                if b.at(i) != self._b[i] {
                    return false;
                }
            }
        }
        return true;
    }

    /**
     * Returns:
     *   +1 if argument is smaller
     *   0 if this and argument is the same
     *   -1 if argument is larger
     * @param other
     */
    pub fn compare(&self, other: Bytes) -> Ordering {
        let slen = min(self._b.len(), other.length());

        for i in 0..slen - 1 {
            let diff : i32 = other.at(i) as i32 - self._b[i] as i32;
            if diff < 0 {
                return Ordering::Less;
            } else if diff > 0 {
                return Ordering::Greater;
            }
        }
        if self._b.len() < slen {
            return Ordering::Less;
        } else if slen < other.length() {
            return Ordering::Greater;
        }
        return Ordering::Equal;
    }
}

pub fn sha256(value: Bytes) -> Bytes {
    let hashed = Sha256::digest(&value.data()[..]);
    let hashed_iter = hashed.into_iter();
    let newvec : Vec<u8> = hashed_iter.collect();
    return Bytes::new(Some(BytesFromType::Raw(newvec)));
}

pub fn list<E, I>(vals : I) -> Vec<E>
where
    I: Iterator<Item = E>,
    E: Clone
{
    return vals.map(|v| v.clone()).collect();
}

pub trait PythonStr {
    fn py_str(&self) -> String;
}

pub fn str<T>(thing : T) -> String
where
    T : PythonStr
{
    return thing.py_str();
}

pub trait PythonRepr {
    fn py_repr(&self) -> String;
}

pub fn repr<T>(thing : T) -> String
where
    T : PythonRepr
{
    return thing.py_repr();
}

#[derive(Debug)]
pub enum Tuple<T1, T2> {
    Tuple(T1,T2)
}

impl<T1, T2> Tuple<T1, T2> {
    pub fn first(&self) -> &T1 {
        return match self {
            Tuple::Tuple(f,_) => f
        };
    }

    pub fn rest(&self) -> &T2 {
        return match self {
            Tuple::Tuple(_,r) => r
        };
    }

    pub fn to_string(&self) -> String
    where
        T1 : PythonStr,
        T2 : PythonStr
    {
        return
            "(".to_owned() +
            self.first().py_str().as_str() +
            ", " +
            self.rest().py_str().as_str() +
            ")";
    }
}

pub fn t<T1, T2>(v1 : T1, v2 : T2) -> Tuple<T1, T2> {
    return Tuple::Tuple(v1,v2);
}

impl <T1, T2> PythonStr for Tuple<T1, T2>
where
    T1 : PythonStr,
    T2 : PythonStr
{
    fn py_str(&self) -> String { return self.to_string(); }
}

const BUF_ALLOC_MULTIPLIER : usize = 4;
const STREAM_INITIAL_BUFFER_SIZE : usize = 64*1024;

pub type Record<K,V> = HashMap<K,V>;

#[derive(Debug)]
pub struct Stream {
    seek: usize,
    length: usize,
    buffer: Vec<u8>
}

impl Stream {
    pub fn new(b : Option<Bytes>) -> Self {
        match b {
            None => {
                return Stream {
                    seek: 0,
                    length: 0,
                    buffer: vec!()
                };
            },
            Some(b) => {
                let data = b.data().to_vec();
                let stream = Stream {
                    seek: 0,
                    length: data.len(),
                    buffer: data
                };

                return stream;
            }
        }
    }

    pub fn get_seek(&self) -> usize {
        return self.seek;
    }

    pub fn set_seek(&mut self, value: i64) {
        if value < 0 {
            self.seek = self.length - 1;
        }
        else if value as usize > self.length - 1 {
            self.seek = self.length;
        }
        else {
            self.seek = value as usize;
        }
    }

    pub fn get_length(&self) -> usize {
        return self.length;
    }

    fn re_allocate(&mut self, size: Option<usize>) {
        let mut s =
            match size {
                None => self.buffer.len() * BUF_ALLOC_MULTIPLIER,
                Some(s) => s
            };

        /*
         * @see https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Errors/Invalid_array_length
         */
        if s > 4294967295 { // 4294967295 = 2**32 - 1
            s = 4294967295;
        }

        let mut buf = Vec::<u8>::with_capacity(s);
        for by in &self.buffer {
            buf.push(*by);
        }
        self.buffer = buf;
    }

    pub fn write(&mut self, b: Bytes) -> usize {
        let new_length = max(self.buffer.len(), b.length() + self.seek);
        if new_length > self.buffer.len() {
            self.re_allocate(Some(new_length * BUF_ALLOC_MULTIPLIER));
        }

        if b.length() > 0 {
            self.buffer.resize(max(self.seek + b.length(), self.buffer.len()), 0);

            for i in 0..b.length() {
                self.buffer[i + self.seek] = b.at(i);
            }
        }

        self.length = new_length;
        self.seek += b.length(); // Don't move this line prior to `this._length = newLength`!
        return b.length();
    }

    pub fn read(&mut self, size: usize) -> Bytes {
        if self.seek > self.length || size == 0 {
            return Bytes::new(None); // Return empty byte
        }

        let size =
            if self.seek + size <= self.length {
                size
            } else {
                self.length - self.seek
            };

        let mut u8 = Vec::<u8>::with_capacity(size);
        for i in 0..size {
            u8.push(self.buffer[self.seek + i]);
        }
        self.seek += size;
        return Bytes::new(Some(BytesFromType::Raw(u8)));
    }

    pub fn get_value(&self) -> Bytes {
        return Bytes::new(Some(BytesFromType::Raw(self.buffer.clone())));
    }
}

pub fn bi_zero() -> Number { return Zero::zero(); }
pub fn bi_one() -> Number { return One::one(); }

pub fn get_u32(v: &Vec<u8>, n: usize) -> u32 {
    let p1 = v[n] as u32;
    let p2 = v[n+1] as u32;
    let p3 = v[n+2] as u32;
    let p4 = v[n+3] as u32;
    return p1 | (p2 << 8) | (p3 << 16) | (p4 << 24);
}

pub fn set_u8(vec: &mut Vec<u8>, n: usize, v: u8) {
    vec[n] = v;
}

pub fn set_u32(vec: &mut Vec<u8>, n: usize, v: u32) {
    vec[n] = (v & 0xff) as u8;
    vec[n+1] = ((v >> 8) & 0xff) as u8;
    vec[n+2] = ((v >> 16) & 0xff) as u8;
    vec[n+3] = ((v >> 24) & 0xff) as u8;
}
