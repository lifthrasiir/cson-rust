pub mod io {
    use std::io::{BufRead, Error, ErrorKind, Read};
    use std::io::Result as IoResult;

    pub const NO_PROGRESS_LIMIT: usize = 1000;

    pub enum ReadBytes { NotEnough(usize), Enough(usize) }

    /// This function was imported from `std::old_io::Reader`.
    pub fn read_at_least<R: Read>(r: &mut R, min: usize, buf: &mut [u8]) -> IoResult<ReadBytes> {
        if min > buf.len() {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "the buffer is too short"));
        }
        let mut read = 0;
        while read < min {
            let mut zeroes = 0;
            loop {
                match try!(r.read(&mut buf[read..])) {
                    0 => {
                        zeroes += 1;
                        if zeroes >= NO_PROGRESS_LIMIT {
                            break;
                        }
                    }
                    n => {
                        read += n;
                        break;
                    }
                }
            }
        }
        if read < min {
            Ok(ReadBytes::NotEnough(read))
        } else {
            Ok(ReadBytes::Enough(read))
        }
    }

    pub fn read_byte<R: Read>(r: &mut R) -> IoResult<Option<u8>> {
        let mut buf = [0];
        match try!(read_at_least(r, 1, &mut buf)) {
            ReadBytes::NotEnough(_) => Ok(None),
            ReadBytes::Enough(_) => Ok(Some(buf[0])),
        }
    }

    /// Reads the next utf8-encoded character from the underlying stream.
    ///
    /// # Error
    ///
    /// If an I/O error occurs, or EOF, then this function will return `Err`.
    /// This function will also return error if the stream does not contain a
    /// valid utf-8 encoded codepoint as the next few bytes in the stream.
    pub fn read_char<B: BufRead>(b: &mut B) -> IoResult<Option<char>> {
        fn invalid_input() -> Error {
            Error::new(ErrorKind::InvalidInput, "invalid input")
        }

        let first_byte = match try!(read_byte(b)) {
            Some(b) => b,
            None => return Ok(None),
        };
        let width = super::char::utf8_char_width(first_byte);
        if width == 1 { return Ok(Some(first_byte as char)) }
        if width == 0 { return Err(invalid_input()) } // not utf8
        let mut buf = [first_byte, 0, 0, 0];
        {
            let mut start = 1;
            while start < width {
                match try!(b.read(&mut buf[start .. width])) {
                    n if n == width - start => break,
                    n if n < width - start => { start += n; }
                    _ => return Err(invalid_input()),
                }
            }
        }
        match ::std::str::from_utf8(&buf[..width]).ok() {
            Some(s) => Ok(s.chars().nth(0)),
            None => Err(invalid_input())
        }
    }

}

pub mod char {
    // UTF-8 ranges and tags for encoding characters
    const TAG_CONT: u8    = 0b1000_0000;
    const TAG_TWO_B: u8   = 0b1100_0000;
    const TAG_THREE_B: u8 = 0b1110_0000;
    const TAG_FOUR_B: u8  = 0b1111_0000;
    const MAX_ONE_B: u32   =     0x80;
    const MAX_TWO_B: u32   =    0x800;
    const MAX_THREE_B: u32 =  0x10000;

    // https://tools.ietf.org/html/rfc3629
    static UTF8_CHAR_WIDTH: [u8; 256] = [
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x1F
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x3F
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x5F
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x7F
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0x9F
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0xBF
        0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
        2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, // 0xDF
        3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3, // 0xEF
        4,4,4,4,4,0,0,0,0,0,0,0,0,0,0,0, // 0xFF
        ];

    /// Given a first byte, determine how many bytes are in this UTF-8 character
    #[inline]
    pub fn utf8_char_width(b: u8) -> usize {
        return UTF8_CHAR_WIDTH[b as usize] as usize;
    }

    /// Encodes a raw u32 value as UTF-8 into the provided byte buffer,
    /// and then returns the number of bytes written.
    ///
    /// If the buffer is not large enough, nothing will be written into it
    /// and a `None` will be returned.
    #[inline]
    pub fn encode_utf8_raw(code: u32, dst: &mut [u8]) -> Option<usize> {
        // Marked #[inline] to allow llvm optimizing it away
        if code < MAX_ONE_B && dst.len() >= 1 {
            dst[0] = code as u8;
            Some(1)
        } else if code < MAX_TWO_B && dst.len() >= 2 {
            dst[0] = (code >> 6 & 0x1F) as u8 | TAG_TWO_B;
            dst[1] = (code & 0x3F) as u8 | TAG_CONT;
            Some(2)
        } else if code < MAX_THREE_B && dst.len() >= 3  {
            dst[0] = (code >> 12 & 0x0F) as u8 | TAG_THREE_B;
            dst[1] = (code >>  6 & 0x3F) as u8 | TAG_CONT;
            dst[2] = (code & 0x3F) as u8 | TAG_CONT;
            Some(3)
        } else if dst.len() >= 4 {
            dst[0] = (code >> 18 & 0x07) as u8 | TAG_FOUR_B;
            dst[1] = (code >> 12 & 0x3F) as u8 | TAG_CONT;
            dst[2] = (code >>  6 & 0x3F) as u8 | TAG_CONT;
            dst[3] = (code & 0x3F) as u8 | TAG_CONT;
            Some(4)
        } else {
            None
        }
    }
}
