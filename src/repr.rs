// This is a part of CSON-rust.
// Written by Kang Seonghoon. See README.md for details.

//! An internal representation of CSON data.

use std::str::MaybeOwned;
use std::collections::TreeMap;

#[deriving(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slice<'a>(&'a str);

impl<'a> Slice<'a> {
    pub fn new(base: &'a str, start: uint, end: uint) -> Slice<'a> {
        Slice(base.slice(start, end))
    }
}

impl<'a> Str for Slice<'a> {
    fn as_slice<'a>(&'a self) -> &'a str {
        let Slice(slice) = *self;
        slice
    }
}

#[deriving(PartialEq, Show, Clone)]
pub enum Atom<'a> {
    Null,
    True,
    False,
    //UnparsedNumber(Slice<'a>),
    IntegralNumber(i64),
    Number(f64),
    //UnparsedString(Slice<'a>),
    //ParsedString(Slice<'a>),
    OwnedString(String),
    List(List<'a>),
    Object(Object<'a>),
}

impl<'a> Atom<'a> {
    pub fn into_parsed(self) -> Atom<'a> {
        match self {
            Null => Null,
            True => True,
            False => False,
            //UnparsedNumber(s) => Number(from_str(s.as_slice()).unwrap()),
            IntegralNumber(v) => IntegralNumber(v),
            Number(v) => Number(v),
            //UnparsedString(s) => parse
            OwnedString(s) => OwnedString(s),
            List(l) => List(l.move_iter().map(|e| e.into_parsed()).collect()),
            Object(o) => Object(o.move_iter().map(|(k,v)| (k,v.into_parsed())).collect()),
        }
    }

    pub fn into_owned(self) -> Atom<'static> {
        match self {
            Null => Null,
            True => True,
            False => False,
            //UnparsedNumber(s) => Number(from_str(s.as_slice()).unwrap()),
            IntegralNumber(v) => IntegralNumber(v),
            Number(v) => Number(v),
            //UnparsedString(s) => parse
            OwnedString(s) => OwnedString(s),
            List(l) => List(l.move_iter().map(|e| e.into_owned()).collect()),
            Object(o) => Object(o.move_iter().map(|(k,v)| (k.into_string().into_maybe_owned(),
                                                           v.into_owned())).collect()),
        }
    }
}

pub type Key<'a> = MaybeOwned<'a>;
pub type List<'a> = Vec<Atom<'a>>;
pub type Object<'a> = TreeMap<Key<'a>, Atom<'a>>;

