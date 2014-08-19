// This is a part of CSON-rust.
// Written by Kang Seonghoon. See README.md for details.

use std::{char, str, fmt};
use std::str::MaybeOwned;
use std::io::{BufReader, IoError, IoResult, EndOfFile};
use std::collections::TreeMap;
use super::repr;

#[deriving(PartialEq)]
pub struct ReaderError {
    pub cause: MaybeOwned<'static>,
    pub ioerr: Option<IoError>,
}

impl fmt::Show for ReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.ioerr {
            Some(ref ioerr) => write!(f, "{} ({})", self.cause, *ioerr),
            None => write!(f, "{}", self.cause),
        }
    }
}

pub type ReaderResult<T> = Result<T, ReaderError>;

fn is_id_start(c: char) -> bool {
    match c {
        '\u0024' |
        '\u002D' |
        '\u0041'..'\u005A' |
        '\u005F' |
        '\u0061'..'\u007A' |
        '\u00AA' |
        '\u00B5' |
        '\u00BA' |
        '\u00C0'..'\u00D6' |
        '\u00D8'..'\u00F6' |
        '\u00F8'..'\u02FF' |
        '\u0370'..'\u037D' |
        '\u037F'..'\u1FFF' |
        '\u200C'..'\u200D' |
        '\u2070'..'\u218F' |
        '\u2C00'..'\u2FEF' |
        '\u3001'..'\uD7FF' |
        '\uF900'..'\uFDCF' |
        '\uFDF0'..'\uFFFD' |
        '\U00010000'..'\U000EFFFF' => true,
        _ => false
    }
}

fn is_id_start_byte(b: u8) -> bool {
    match b {
        0x24 |       // %x24 /
        0x2d |       // %x2D /
        0x41..0x5a | // %x41-5A /
        0x5f |       // %x5F /
        0x61..0x7a | // %x61-7A /
        0xc2 |       // %xAA / %xB5 / %xBA /
        0xc3 |       // %xC0-D6 / %xD8-F6 / %xF8-FF /
        0xc4..0xcb | // %x0100-02FF /
        0xcd |       // %x0370-037D / %x037F /
        0xce..0xe1 | // %x0380-1FFF /
        0xe2 |       // %x200C-200D / %x2070-218F / %x2C00-2FEF /
        0xe3..0xed | // %x3001-D7FF /
        0xef |       // %xF900-FDCF / %xFDF0-FFFD /
        0xf0..0xf3   // %x10000-EFFFF
            => true,
        _ => false
    }
}

#[test]
fn test_is_id_start() {
    let mut present = [false, ..256];
    for c in range(0u32, 0x110000).filter_map(char::from_u32).filter(|&c| is_id_start(c)) {
        assert!(is_id_end(c), "is_id_end('{}' /*{:x}*/) is false", c, c as u32);
        let mut buf = [0u8, ..4];
        c.encode_utf8(buf);
        present[buf[0] as uint] = true;
    }
    for b in range(0u, 256) {
        assert!(is_id_start_byte(b as u8) == present[b],
                "is_id_start_byte({}): expected {}, get {}",
                b, is_id_start_byte(b as u8), present[b]);
    }
}

fn is_id_end(c: char) -> bool {
    match c {
        '\u0024' |
        '\u002D'..'\u002E' |
        '\u0030'..'\u0039' |
        '\u0041'..'\u005A' |
        '\u005F' |
        '\u0061'..'\u007A' |
        '\u00AA' |
        '\u00B5' |
        '\u00B7' |
        '\u00BA' |
        '\u00C0'..'\u00D6' |
        '\u00D8'..'\u00F6' |
        '\u00F8'..'\u037D' |
        '\u037F'..'\u1FFF' |
        '\u200C'..'\u200D' |
        '\u203F'..'\u2040' |
        '\u2070'..'\u218F' |
        '\u2C00'..'\u2FEF' |
        '\u3001'..'\uD7FF' |
        '\uF900'..'\uFDCF' |
        '\uFDF0'..'\uFFFD' |
        '\U00010000'..'\U000EFFFF' => true,
        _ => false
    }
}

fn is_id_end_byte(b: u8) -> bool {
    match b {
        0x24 |       // %x24 /
        0x2d..0x2e | // %x2D-2E /
        0x30..0x39 | // %x30-39 /
        0x41..0x5a | // %x41-5A /
        0x5f |       // %x5F /
        0x61..0x7a | // %x61-7A /
        0xc2 |       // %xAA / %xB5 / %xB7 / %xBA /
        0xc3 |       // %xC0-D6 / %xD8-F6 / %xF8-FF /
        0xc4..0xe1 | // %x0100-037D / %x037F-1FFF /
        0xe2 |       // %x200C-200D / %x203F-2040 / %x2070-218F / %x2C00-2FEF /
        0xe3..0xed | // %x3001-D7FF /
        0xef |       // %xF900-FDCF / %xFDF0-FFFD /
        0xf0..0xf3   // %x10000-EFFFF
            => true,
        _ => false
    }
}

#[test]
fn test_is_id_end() {
    let mut present = [false, ..256];
    for c in range(0u32, 0x110000).filter_map(char::from_u32).filter(|&c| is_id_end(c)) {
        let mut buf = [0u8, ..4];
        c.encode_utf8(buf);
        present[buf[0] as uint] = true;
    }
    for b in range(0u, 256) {
        assert!(is_id_end_byte(b as u8) == present[b],
                "is_id_end_byte({}): expected {}, get {}",
                b, is_id_end_byte(b as u8), present[b]);
    }
}

fn into_reader_result<T>(res: IoResult<T>) -> ReaderResult<Option<T>> {
    match res {
        Ok(v) => Ok(Some(v)),
        Err(ref err) if err.kind == EndOfFile => Ok(None),
        Err(err) => Err(ReaderError { cause: "I/O error".into_maybe_owned(), ioerr: Some(err) })
    }
}

fn reader_err<T, Cause:IntoMaybeOwned<'static>>(cause: Cause) -> ReaderResult<T> {
    Err(ReaderError { cause: cause.into_maybe_owned(), ioerr: None })
}

struct Newline;

pub struct Reader<'a> {
    buf: &'a mut Buffer,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a mut Buffer) -> Reader<'a> {
        Reader { buf: buf }
    }

    pub fn parse_document_from_buf(buf: &[u8]) -> ReaderResult<repr::Atom<'static>> {
        Reader::new(&mut BufReader::new(buf)).parse_document()
    }

    pub fn parse_value_from_buf(buf: &[u8]) -> ReaderResult<repr::Atom<'static>> {
        Reader::new(&mut BufReader::new(buf)).parse_value()
    }

    pub fn parse_document(mut self) -> ReaderResult<repr::Atom<'static>> {
        let ret = try!(self.document());
        try!(self.skip_ws());
        try!(self.eof());
        Ok(ret)
    }

    pub fn parse_value(mut self) -> ReaderResult<repr::Atom<'static>> {
        try!(self.skip_ws());
        let ret = try!(self.value());
        try!(self.skip_ws());
        try!(self.eof());
        Ok(ret)
    }

    fn eof(&mut self) -> ReaderResult<()> {
        loop {
            match try!(into_reader_result(self.buf.fill_buf())) {
                Some(buf) => {
                    if !buf.is_empty() {
                        return reader_err("expected end of file");
                    }
                }
                None => {
                    return Ok(());
                }
            }
        }
    }

    fn peek(&mut self) -> ReaderResult<Option<u8>> {
        loop {
            match try!(into_reader_result(self.buf.fill_buf())) {
                Some(buf) => {
                    if !buf.is_empty() {
                        return Ok(Some(buf[0]));
                    }
                }
                None => {
                    return Ok(None);
                }
            }
        }
    }

    fn fixed_token_opt(&mut self, token: &[u8]) -> ReaderResult<Option<()>> {
        static MAX_TOKEN_LEN: uint = 8;
        assert!(token.len() <= MAX_TOKEN_LEN);
        let mut scratch = [0u8, ..MAX_TOKEN_LEN];
        let tokenbuf = scratch.mut_slice_to(token.len());
        try!(into_reader_result(self.buf.read_at_least(token.len(), tokenbuf)));
        if tokenbuf == token { Ok(Some(())) } else { Ok(None) }
    }

    fn loop_with_buffer(&mut self, callback: |&[u8]| -> Option<uint>) -> ReaderResult<bool> {
        let mut used;
        loop {
            {
                let buf = match try!(into_reader_result(self.buf.fill_buf())) {
                    Some(buf) => buf,
                    None => { return Ok(false); }
                };

                match callback(buf) {
                    Some(used_) => { used = used_; break; }
                    None => { used = buf.len(); }
                }
            }

            self.buf.consume(used);
        }
        self.buf.consume(used);
        Ok(true)
    }

    /// Parses `JSON-text` where:
    ///
    /// ~~~~ {.text}
    /// JSON-text = object
    ///           / array
    ///           / ws object-items
    /// ~~~~
    fn document(&mut self) -> ReaderResult<repr::Atom<'static>> {
        try!(self.skip_ws());
        match try!(self.peek()) {
            Some(b'{') => self.object_no_peek().map(repr::Object),
            Some(b'[') => self.array_no_peek().map(repr::List),
            Some(_) => { try!(self.skip_ws()); Ok(repr::Object(try!(self.object_items_opt()))) },
            _ => reader_err("expected document"),
        }
    }

    /// Parses `value-separator` if possible, where:
    ///
    /// ~~~~ {.text}
    /// value-separator = ws %x2C ws    ; , comma
    ///                 / newline ws
    /// newline = *(%x20 / %x09) newline-char
    /// ~~~~
    fn skip_value_separator_opt(&mut self) -> ReaderResult<Option<()>> {
        let newline = try!(self.skip_ws());
        if try!(self.peek()) == Some(b',') {
            self.buf.consume(1);
            try!(self.skip_ws());
        } else {
            if newline.is_none() { return Ok(None); }
        }
        Ok(Some(()))
    }

    /// Parses `ws` or `newline ws` where:
    ///
    /// ~~~~ {.text}
    /// ws = *(
    ///           %x20 /                ; Space
    ///           %x09 /                ; Horizontal tab
    ///           newline-char /
    ///           comment
    ///       )
    ///
    /// newline-char = %x0A             ; Line feed or New line
    ///              / %x0D             ; Carriage return
    /// comment = sharp *non-newline-char
    /// sharp = %x23                    ; # sharp
    /// ~~~~
    ///
    /// Returns true when `ws` contains at least one `newline`.
    fn skip_ws(&mut self) -> ReaderResult<Option<Newline>> {
        let mut newline = None;
        loop {
            let mut comment_chars = false;
            try!(self.loop_with_buffer(|buf| {
                for (i, &v) in buf.iter().enumerate() {
                    match v {
                        0x20 | 0x09 => {}
                        0x0a | 0x0d => { newline = Some(Newline); }
                        0x23 => { comment_chars = true; return Some(i + 1); }
                        _ => { return Some(i); }
                    }
                }
                None
            }));

            if comment_chars {
                try!(self.skip_non_newline_chars());
            } else {
                break;
            }
        }
        Ok(newline)
    }

    /// Parses and discards `*non-newline-char` where:
    ///
    /// ~~~~ {.text}
    /// non-newline-char = %x00-09 / %x0B-0C / %x0E-10FFFF
    /// ~~~~
    fn skip_non_newline_chars(&mut self) -> ReaderResult<()> {
        try!(self.loop_with_buffer(|buf| {
            for (i, &v) in buf.iter().enumerate() {
                if v == 0x0a || v == 0x0d { return Some(i); }
            }
            None
        }));
        Ok(())
    }

    /// Parses `*non-newline-char`.
    ///
    /// It may return an invalid UTF-8 sequence.
    /// The caller is responsible for checking for the valid UTF-8 whenever appropriate.
    fn non_newline_chars(&mut self) -> ReaderResult<Vec<u8>> {
        let mut bytes = Vec::new();
        try!(self.loop_with_buffer(|buf| {
            let mut ret = None;
            for (i, &v) in buf.iter().enumerate() {
                if v == 0x0a || v == 0x0d {
                    ret = Some(i);
                    break;
                }
            }
            bytes.extend(buf.slice_to(ret.unwrap_or(buf.len())).iter().map(|&b| b));
            ret
        }));
        Ok(bytes)
    }

    /// Given every preceding whitespace skipped, parses `value`.
    fn value(&mut self) -> ReaderResult<repr::Atom<'static>> {
        match try!(self.value_opt()) {
            Some(value) => Ok(value),
            _ => reader_err("expected value"),
        }
    }

    /// Given every preceding whitespace skipped, parses `value` if possible, where:
    ///
    /// ~~~~ {.text}
    /// value = false / null / true / object / array / number / string
    ///       / verbatim-string
    ///
    /// false = %x66.61.6c.73.65        ; false
    /// null  = %x6e.75.6c.6c           ; null
    /// true  = %x74.72.75.65           ; true
    /// ~~~~
    fn value_opt(&mut self) -> ReaderResult<Option<repr::Atom<'static>>> {
        match try!(self.peek()) {
            Some(b'f') => match try!(self.fixed_token_opt(b"false")) {
                Some(()) => Ok(Some(repr::False)),
                None => reader_err("expected false"),
            },
            Some(b'n') => match try!(self.fixed_token_opt(b"null")) {
                Some(()) => Ok(Some(repr::Null)),
                None => reader_err("expected null"),
            },
            Some(b't') => match try!(self.fixed_token_opt(b"true")) {
                Some(()) => Ok(Some(repr::True)),
                None => reader_err("expected true"),
            },
            Some(b'{') => self.object_no_peek().map(|v| Some(repr::Object(v))),
            Some(b'[') => self.array_no_peek().map(|v| Some(repr::List(v))),
            Some(b @ b'-') | Some(b @ b'0'..b'9') => self.number_no_peek(b).map(Some),
            Some(quote @ b'"') | Some(quote @ b'\'') =>
                self.string_no_peek(quote).map(|s| Some(repr::OwnedString(s.into_string()))),
            Some(b'|') => {
                let frags = try!(self.verbatim_string_no_peek());
                Ok(Some(repr::OwnedString(frags.connect("\n"))))
            },
            _ => Ok(None),
        }
    }

    /// Given a known lookahead, parses `object` where:
    ///
    /// ~~~~ {.text}
    /// object = begin-object [ object-items ] end-object
    ///
    /// begin-object    = ws %x7B ws    ; { left curly bracket
    /// end-object      = ws %x7D ws    ; } right curly bracket
    /// ~~~~
    fn object_no_peek(&mut self) -> ReaderResult<repr::Object<'static>> {
        assert_eq!(self.peek(), Ok(Some(b'{')));

        self.buf.consume(1);
        try!(self.skip_ws());
        let items = try!(self.object_items_opt());
        if try!(self.peek()) != Some(b'}') {
            return reader_err("expected `}`");
        }
        self.buf.consume(1);
        Ok(items)
    }

    /// Parses `[ object-items ]` where:
    ///
    /// ~~~~ {.text}
    /// object-items = member *( value-separator member ) [ value-separator ]
    ///
    /// value-separator = ws %x2C ws    ; , comma
    ///                 / newline ws
    /// newline = *(%x20 / %x09) newline-char
    /// ~~~~
    fn object_items_opt(&mut self) -> ReaderResult<repr::Object<'static>> {
        let mut items = TreeMap::new();
        let (firstkey, firstvalue) = match try!(self.member_opt()) {
            Some(member) => member,
            None => { return Ok(items); }
        };
        items.insert(firstkey, firstvalue);
        loop {
            if try!(self.skip_value_separator_opt()).is_none() { break; }
            let (key, value) = match try!(self.member_opt()) {
                Some(member) => member,
                None => { break; }
            };
            items.insert(key, value);
        }
        Ok(items)
    }

    /// Parses `member` if possible, where:
    ///
    /// ~~~~ {.text}
    /// member = name name-separator value
    /// ~~~~
    fn member_opt(&mut self) -> ReaderResult<Option<(repr::Key<'static>,
                                                     repr::Atom<'static>)>> {
        let name = match try!(self.name_opt()) {
            Some(name) => name,
            None => { return Ok(None); }
        };
        try!(self.skip_ws());
        match try!(self.peek()) {
            Some(b':') | Some(b'=') => { self.buf.consume(1); }
            _ => { return reader_err("expected `:` or `=`"); }
        }
        try!(self.skip_ws());
        let value = try!(self.value());
        Ok(Some((name, value)))
    }

    /// Parses `name` if possible, where:
    ///
    /// ~~~~ {.text}
    /// name = string / bare-string
    /// bare-string = id-start *id-end
    ///
    /// id-start = %x24 / %x2D / %x41-5A / %x5F / %x61-7A / %xAA / %xB5
    ///          / %xBA / %xC0-D6 / %xD8-F6 / %xF8-02FF / %x0370-037D
    ///          / %x037F-1FFF / %x200C-200D / %x2070-218F / %x2C00-2FEF
    ///          / %x3001-D7FF / %xF900-FDCF / %xFDF0-FFFD / %x10000-EFFFF
    /// id-end = id-start / %x2E / %x30-39 / %xB7 / %x0300-036F / %x203F-2040
    /// ~~~~
    fn name_opt(&mut self) -> ReaderResult<Option<MaybeOwned<'static>>> {
        match try!(self.peek()) {
            Some(quote @ b'"') | Some(quote @ b'\'') =>
                self.string_no_peek(quote).map(|s| Some(s.into_maybe_owned())),
            Some(b) if is_id_start_byte(b) => self.bare_string_no_peek().map(Some),
            _ => Ok(None),
        }
    }

    /// Parses `array` where:
    ///
    /// ~~~~ {.text}
    /// array = begin-array [ array-items ] end-array
    ///
    /// begin-array     = ws %x5B ws    ; [ left square bracket
    /// end-array       = ws %x5D ws    ; ] right square bracket
    /// ~~~~
    fn array_no_peek(&mut self) -> ReaderResult<repr::List<'static>> {
        assert_eq!(self.peek(), Ok(Some(b'[')));

        self.buf.consume(1);
        try!(self.skip_ws());
        let elements = try!(self.array_items_opt());
        if try!(self.peek()) != Some(b']') {
            return reader_err("expected `]`");
        }
        self.buf.consume(1);
        Ok(elements)
    }

    /// Parses `[ array-items ]` where:
    ///
    /// ~~~~ {.text}
    /// array-items = value *( value-separator value ) [ value-separator ]
    /// ~~~~
    fn array_items_opt(&mut self) -> ReaderResult<repr::List<'static>> {
        let mut elements = Vec::new();
        let first = match try!(self.value_opt()) {
            Some(first) => first,
            None => { return Ok(elements); }
        };
        elements.push(first);
        loop {
            if try!(self.skip_value_separator_opt()).is_none() { break; }
            let value = match try!(self.value_opt()) {
                Some(value) => value,
                None => { break; }
            };
            elements.push(value);
        }
        Ok(elements)
    }

    /// Parses and pushes `*DIGITS` into `bytes`.
    fn digits_opt(&mut self, bytes: &mut Vec<u8>) -> ReaderResult<()> {
        try!(self.loop_with_buffer(|buf| {
            let mut ret = None;
            for (i, &v) in buf.iter().enumerate() {
                if v < b'0' || b'9' < v {
                    ret = Some(i);
                    break;
                }
            }
            bytes.extend(buf.slice_to(ret.unwrap_or(buf.len())).iter().map(|&b| b));
            ret
        }));
        Ok(())
    }

    /// Given a known lookahead, parses `number` where:
    ///
    /// ~~~~ {.text}
    /// number = [ minus ] int [ frac ] [ exp ]
    /// decimal-point = %x2E            ; .
    /// digit1-9 = %x31-39              ; 1-9
    /// e = %x65 / %x45                 ; e E
    /// exp = e [ minus / plus ] 1*DIGIT
    /// frac = decimal-point 1*DIGIT
    /// int = zero / ( digit1-9 *DIGIT )
    /// minus = %x2D                    ; -
    /// plus = %x2B                     ; +
    /// zero = %x30                     ; 0
    /// ~~~~
    fn number_no_peek(&mut self, initial: u8) -> ReaderResult<repr::Atom<'static>> {
        assert_eq!(self.peek(), Ok(Some(initial)));

        self.buf.consume(1);

        // special case. both JSON and CSON does not allow a zero-padded non-zero number.
        let next = try!(self.peek());
        if initial == b'0' && next != Some(b'.') && next != Some(b'e') && next != Some(b'E') {
            // as long as it is not followed by `frac` and `exp`, we are free to shortcut
            return Ok(repr::IntegralNumber(0));
        }

        let mut bytes = vec![initial];

        // we need to ensure if this parse would end up with at least one number
        if initial == b'-' {
            match try!(self.peek()) {
                Some(b @ b'0'..b'9') => { bytes.push(b); self.buf.consume(1); }
                _ => { return reader_err("expected a number, got `-`"); }
            }
        }

        // parse up to `[ minus ] int`
        try!(self.digits_opt(&mut bytes));

        // parse up to `[ minus ] int [ frac ]`
        let mut try_integral = true;
        match try!(self.peek()) {
            Some(b'.') => {
                bytes.push(b'.');
                self.buf.consume(1);
                match try!(self.peek()) {
                    Some(b @ b'0'..b'9') => { bytes.push(b); self.buf.consume(1); }
                    _ => { return reader_err("a number cannot have a trailing decimal point"); }
                }
                try!(self.digits_opt(&mut bytes));
                try_integral = false;
            }
            _ => {}
        }

        // parse up to `[ minus ] int [ frac ] [ exp ]`
        match try!(self.peek()) {
            Some(b @ b'e') | Some(b @ b'E') => {
                bytes.push(b);
                self.buf.consume(1);
                match try!(self.peek()) {
                    Some(b @ b'-') | Some(b @ b'+') => { bytes.push(b); self.buf.consume(1); }
                    _ => {}
                }
                match try!(self.peek()) {
                    Some(b @ b'0'..b'9') => { bytes.push(b); self.buf.consume(1); }
                    _ => { return reader_err("a number has an incomplete exponent part"); }
                }
                try!(self.digits_opt(&mut bytes));
                try_integral = false;
            }
            _ => {}
        }

        let s = str::from_utf8(bytes.as_slice()).unwrap();
        if try_integral {
            // try to return as `IntegralNumber` if possible
            match from_str::<i64>(s) {
                Some(v) if (-1<<53) < v && v < (1<<53) => { return Ok(repr::IntegralNumber(v)); }
                _ => {}
            }
        }
        Ok(repr::Number(from_str::<f64>(s).unwrap()))
    }

    /// Given a known lookahead, parses `string` where:
    ///
    /// ~~~~ {.text}
    /// string = quotation-mark *dquoted-char quotation-mark
    ///        / apostrophe-mark *squoted-char apostrophe-mark
    /// ~~~~
    fn string_no_peek(&mut self, quote: u8) -> ReaderResult<MaybeOwned<'static>> {
        self.buf.consume(1);
        self.quoted_chars_then_quote(quote)
    }

    /// Parses `*dquoted-char quotation-mark` (when `quote == '"'`) or
    /// `*squoted-char apostrophe-mark` (when `quote == '\''`) where:
    ///
    /// ~~~~ {.text}
    /// dquoted-char = dquoted-unescaped / escaped
    /// squoted-char = squoted-unescaped / escaped
    /// quotation-mark = %x22           ; "
    /// apostrophe-mark = %x27          ; '
    /// dquoted-unescaped = %x20-21 / %x23-5B / %x5D-10FFFF
    /// squoted-unescaped = %x20-26 / %x28-5B / %x5D-10FFFF
    /// ~~~~
    fn quoted_chars_then_quote(&mut self, quote: u8) -> ReaderResult<MaybeOwned<'static>> {
        let mut bytes = Vec::new();
        loop {
            let mut escaped_follows = false;
            let keepgoing = try!(self.loop_with_buffer(|buf| {
                let mut ret = None;
                for (i, &v) in buf.iter().enumerate() {
                    if v == b'\\' {
                        escaped_follows = true;
                        ret = Some(i + 1);
                        break;
                    } else if v == quote {
                        ret = Some(i + 1); // consume a quote as well
                        break;
                    }
                }
                // `ret`, if set, contains one additional byte which should not be in `bytes`.
                bytes.extend(buf.slice_to(ret.map_or(buf.len(), |i| i-1)).iter().map(|&b| b));
                ret
            }));
            if !keepgoing {
                return reader_err("incomplete string literal");
            }

            if escaped_follows {
                let ch = match try!(self.escaped_minus_escape()) {
                    first @ 0xd800..0xdbff => {
                        // lower surrogate, should be followed by an escaped upper surrogate
                        if try!(self.peek()) != Some(b'\\') {
                            return reader_err(format!("lower surrogate `\\u{:04x}` is not followed \
                                                       with an escaped upper surrogate", first));
                        }
                        self.buf.consume(1);
                        let second = try!(self.escaped_minus_escape());
                        if !(0xdc00 <= second && second <= 0xdfff) {
                            return reader_err(format!("lower surrogate `\\u{:04x}` is not followed \
                                                       with an escaped upper surrogate \
                                                       (got `\\u{:04x}` instead)", first, second));
                        }
                        0x10000 + (((first - 0xd800) as u32 << 10) | ((second - 0xdc00) as u32))
                    },
                    second @ 0xdc00..0xdfff => {
                        // upper surrogate, not allowed
                        return reader_err(format!("upper surrogate `\\u{:04x}` cannot be used \
                                                   independently", second));
                    },
                    ch => ch as u32,
                };

                // append a converted UTF-8 sequence into `bytes`.
                // this wouldn't affect the validness of other raw `bytes` as UTF-8 ensures that
                // no valid sequence can made into invalid one or vice versa.
                let mut charbuf = [0u8, ..4];
                let charbuflen = char::from_u32(ch).unwrap().encode_utf8(charbuf).unwrap();
                bytes.extend(charbuf.slice_to(charbuflen).iter().map(|&b| b));
            } else {
                break;
            }
        }

        match String::from_utf8(bytes) {
            Ok(s) => Ok(s.into_maybe_owned()),
            Err(_) => reader_err("invalid UTF-8 sequence in a quoted string"),
        }
    }

    /// Parses `escaped` excluding an `escape` character, where:
    ///
    /// ~~~~ {.text}
    /// escaped = escape (
    ///            %x27 /               ; '    apostrophe      U+0027
    ///            %x22 /               ; "    quotation mark  U+0022
    ///            %x5C /               ; \    reverse solidus U+005C
    ///            %x2F /               ; /    solidus         U+002F
    ///            %x62 /               ; b    backspace       U+0008
    ///            %x66 /               ; f    form feed       U+000C
    ///            %x6E /               ; n    line feed       U+000A
    ///            %x72 /               ; r    carriage return U+000D
    ///            %x74 /               ; t    tab             U+0009
    ///            %x75 4HEXDIG )       ; uXXXX                U+XXXX
    /// escape = %x5C                   ; \
    /// ~~~~
    ///
    /// Returns an `u16` instead of a `char` since it may return an incomplete surrogate.
    /// The caller is expected to deal with such cases.
    fn escaped_minus_escape(&mut self) -> ReaderResult<u16> {
        match try!(into_reader_result(self.buf.read_byte())) {
            Some(b'\'') => Ok(0x27),
            Some(b'"') => Ok(0x22),
            Some(b'\\') => Ok(0x5c),
            Some(b'/') => Ok(0x2f),
            Some(b'b') => Ok(0x08),
            Some(b'f') => Ok(0x0c),
            Some(b'n') => Ok(0x0a),
            Some(b'r') => Ok(0x0d),
            Some(b't') => Ok(0x09),
            Some(b'u') => {
                let read_hex_digit = || {
                    match try!(into_reader_result(self.buf.read_byte())) {
                        Some(b @ b'0'..b'9') => Ok((b - b'0') as u16 + 0),
                        Some(b @ b'a'..b'f') => Ok((b - b'a') as u16 + 10),
                        Some(b @ b'A'..b'F') => Ok((b - b'A') as u16 + 10),
                        Some(_) => reader_err("invalid hexadecimal digits after `\\u`"),
                        None => reader_err("incomplete escape sequence"),
                    }
                };
                let a = try!(read_hex_digit());
                let b = try!(read_hex_digit());
                let c = try!(read_hex_digit());
                let d = try!(read_hex_digit());
                Ok((a << 12) | (b << 8) | (c << 4) | d)
            },
            Some(ch) => reader_err(format!("unknown escape sequence `\\{}`", ch)),
            None => reader_err("incomplete escape sequence"),
        }
    }

    /// Given a known lookahead, parses `verbatim-string` where:
    ///
    /// ~~~~ {.text}
    /// verbatim-string = verbatim-fragment *(newline ws verbatim-fragment)
    /// verbatim-fragment = pipe *verbatim-char
    /// pipe = %x7C                     ; |
    /// ~~~~
    fn verbatim_string_no_peek(&mut self) -> ReaderResult<Vec<MaybeOwned<'static>>> {
        assert_eq!(self.peek(), Ok(Some(b'|')));

        let mut frags = Vec::new();
        loop {
            self.buf.consume(1);
            match String::from_utf8(try!(self.non_newline_chars())) {
                Ok(bytes) => { frags.push(bytes.into_maybe_owned()); }
                Err(_) => { return reader_err("invalid UTF-8 sequence in a verbatim string"); }
            }
            self.buf.consume(1); // either 0x0a or 0x0d
            try!(self.skip_ws());
            if try!(self.peek()) != Some(b'|') { break; }
        }
        Ok(frags)
    }

    /// Given a known lookahead, parses `bare-string` where:
    ///
    /// ~~~~ {.text}
    /// bare-string = id-start *id-end
    ///
    /// id-start = %x24 / %x2D / %x41-5A / %x5F / %x61-7A / %xAA / %xB5
    ///          / %xBA / %xC0-D6 / %xD8-F6 / %xF8-02FF / %x0370-037D
    ///          / %x037F-1FFF / %x200C-200D / %x2070-218F / %x2C00-2FEF
    ///          / %x3001-D7FF / %xF900-FDCF / %xFDF0-FFFD / %x10000-EFFFF
    /// id-end = id-start / %x2E / %x30-39 / %xB7 / %x0300-036F / %x203F-2040
    /// ~~~~
    fn bare_string_no_peek(&mut self) -> ReaderResult<MaybeOwned<'static>> {
        assert!(self.peek().ok().and_then(|c| c).map_or(false, is_id_start_byte));

        let mut s = String::new();
        match try!(into_reader_result(self.buf.read_char())) {
            Some(ch) if is_id_start(ch) => { s.push_char(ch); }
            Some(_) => { return reader_err("expected a bare string, got an invalid character"); }
            None    => { return reader_err("expected a bare string, got the end of file"); }
        };
        while try!(self.peek()).map_or(false, is_id_end_byte) {
            match try!(into_reader_result(self.buf.read_char())) {
                Some(ch) if is_id_end(ch) => { s.push_char(ch); }
                Some(_) => { return reader_err("expected a bare string, got an invalid \
                                                character"); }
                None    => { return reader_err("expected a bare string, got the end of file"); }
            };
        }
        Ok(s.into_maybe_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;
    use repr;
    use repr::{Null, True, False, Number};
    use Int = repr::IntegralNumber;

    macro_rules! valid(
        ($buf:expr, $repr:expr) => ({
            let parsed = Reader::parse_value_from_buf($buf.as_bytes());
            let expected = Ok($repr);
            assert_eq!(parsed, expected);
        })
    )

    macro_rules! invalid(
        ($buf:expr) => ({
            let parsed = Reader::parse_value_from_buf($buf.as_bytes());
            assert!(parsed.is_err());
        })
    )

    #[allow(non_snake_case_functions)] // make it look like a constructor
    fn String<'a>(s: &'a str) -> repr::Atom<'a> { repr::OwnedString(s.to_string()) }
    macro_rules! list([$($e:expr),*] => (repr::List(vec![$($e),*])))
    macro_rules! object([$($k:expr => $v:expr),*] =>
                        (repr::Object(vec![$(($k.into_maybe_owned(),
                                              $v)),*].move_iter().collect())))

    #[test]
    fn test_simple() {
        valid!("null", Null);
        valid!("true", True);
        valid!("false", False);
        valid!("0", Int(0));
        valid!("42", Int(42));
        valid!("0.0", Number(0.0));
        valid!("42.0", Number(42.0));
        valid!("0e3", Number(0.0));
        valid!("42e3", Number(42000.0));
        valid!("72057594037927936", Number(72057594037927936.0)); // 2^56 exceeds integral range
        valid!("[1, 2, 3]", list![Int(1), Int(2), Int(3)]);
        valid!("[1\n 2\n 3]", list![Int(1), Int(2), Int(3)]);
        valid!("[null]", list![Null]);
        valid!("\"abc\"", String("abc"));
        valid!("'abc'", String("abc"));
        valid!("|abc\n|def", String("abc\ndef"));
        valid!("[|a\n\n |b\n\n |c\n,|d\n]", list![String("a\nb\nc"), String("d")]);
        valid!("{\"f\": 1, 'g': 2}", object!["f" => Int(1), "g" => Int(2)]);
        valid!("{f=1\n g=2}", object!["f" => Int(1), "g" => Int(2)]);
    }
}

