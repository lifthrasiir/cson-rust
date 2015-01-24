// This is a part of CSON-rust.
// Written by Kang Seonghoon. See README.md for details.

//! An internal representation of CSON data.

use std::fmt;
use std::borrow::{Cow, IntoCow};
use std::ops::Deref;
use std::string::CowString;
use std::collections::BTreeMap;
use serialize::json::{Json, ToJson};

pub use self::Atom::{Null, True, False, I64, U64, F64, OwnedString, Array, Object};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slice<'a>(&'a str);

impl<'a> Slice<'a> {
    pub fn new(base: &'a str, start: usize, end: usize) -> Slice<'a> {
        Slice(&base[start..end])
    }
}

impl<'a> Str for Slice<'a> {
    fn as_slice<'b>(&'b self) -> &'b str {
        let Slice(slice) = *self;
        slice
    }
}

#[derive(PartialEq, Show, Clone)]
pub enum Atom<'a> {
    Null,
    True,
    False,
    //UnparsedF64(Slice<'a>),
    I64(i64),
    U64(u64),
    F64(f64),
    //UnparsedString(Slice<'a>),
    //ParsedString(Slice<'a>),
    OwnedString(String),
    Array(AtomArray<'a>),
    Object(AtomObject<'a>),
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Key<'a>(pub CowString<'a>);

impl<'a> Key<'a> {
    pub fn new<T:IntoCow<'a,String,str>>(s: T) -> Key<'a> { Key(s.into_cow()) }
}

impl<'a> Deref for Key<'a> {
    type Target = str;
    fn deref<'b>(&'b self) -> &'b str { let Key(ref s) = *self; s.deref() }
}

impl<'a> Str for Key<'a> {
    fn as_slice<'b>(&'b self) -> &'b str { let Key(ref s) = *self; s.as_slice() }
}

impl<'a> Clone for Key<'a> {
    fn clone(&self) -> Key<'a> {
        match *self {
            Key(Cow::Borrowed(s)) => Key(Cow::Borrowed(s)),
            Key(Cow::Owned(ref s)) => Key(Cow::Owned(s.clone())),
        }
    }
}

impl<'a> fmt::Debug for Key<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { let Key(ref s) = *self; s.fmt(f) }
}

pub type AtomArray<'a> = Vec<Atom<'a>>;
pub type AtomObject<'a> = BTreeMap<Key<'a>, Atom<'a>>;

impl<'a> Atom<'a> {
    pub fn from_json<T: ToJson>(jsonlike: &T) -> Atom<'a> {
        Atom::from_owned_json(jsonlike.to_json())
    }

    pub fn from_owned_json(json: Json) -> Atom<'a> {
        match json {
            Json::I64(v) => I64(v),
            Json::U64(v) => U64(v),
            Json::F64(v) => F64(v),
            Json::String(s) => OwnedString(s),
            Json::Boolean(true) => True,
            Json::Boolean(false) => False,
            Json::Array(l) => Array(l.into_iter().map(Atom::from_owned_json).collect()),
            Json::Object(o) =>
                Object(o.into_iter().map(|(k,v)| (Key::new(k),
                                                  Atom::from_owned_json(v))).collect()),
            Json::Null => Null,
        }
    }

    pub fn into_parsed(self) -> Atom<'a> {
        match self {
            Null => Null,
            True => True,
            False => False,
            I64(v) => I64(v),
            U64(v) => U64(v),
            F64(v) => F64(v),
            OwnedString(s) => OwnedString(s),
            Array(l) => Array(l.into_iter().map(|e| e.into_parsed()).collect()),
            Object(o) => Object(o.into_iter().map(|(k,v)| (k,v.into_parsed())).collect()),
        }
    }

    pub fn into_owned(self) -> Atom<'static> {
        match self {
            Null => Null,
            True => True,
            False => False,
            I64(v) => I64(v),
            U64(v) => U64(v),
            F64(v) => F64(v),
            OwnedString(s) => OwnedString(s),
            Array(l) => Array(l.into_iter().map(|e| e.into_owned()).collect()),
            Object(o) => Object(o.into_iter().map(|(k,v)| (Key::new(k.to_string()),
                                                           v.into_owned())).collect()),
        }
    }
}

impl<'a> ToJson for Atom<'a> {
    fn to_json(&self) -> Json {
        match *self {
            Null => Json::Null,
            True => Json::Boolean(true),
            False => Json::Boolean(false),
            I64(v) => Json::I64(v),
            U64(v) => Json::U64(v),
            F64(v) => Json::F64(v),
            OwnedString(ref s) => Json::String(s.clone()),
            Array(ref l) => Json::Array(l.iter().map(|e| e.to_json()).collect()),
            Object(ref o) => Json::Object(o.iter().map(|(k,v)| (k.to_string(),
                                                                v.to_json())).collect()),
        }
    }
}

