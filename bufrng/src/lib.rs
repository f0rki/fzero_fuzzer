pub use rand;
pub use rand::RngCore;

pub struct BufRng<'a> {
    buf: &'a [u8],
}

impl<'a> BufRng<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
}

impl<'a> RngCore for BufRng<'a> {
    fn next_u32(&mut self) -> u32 {
        let bl = self.buf.len();
        const SZ: usize = core::mem::size_of::<u32>();
        if bl >= SZ {
            unsafe {
                let p = self.buf.as_ptr();
                self.buf = &self.buf[SZ..];
                (p as *const u32).read_unaligned()
            }
        } else if bl > 0 {
            let mut ibuf = [0u8; SZ];
            let xl = core::cmp::min(bl, SZ);
            ibuf[..xl].copy_from_slice(&self.buf[..xl]);
            self.buf = &self.buf[xl..];
            u32::from_le_bytes(ibuf)
        } else {
            0
        }
    }

    fn next_u64(&mut self) -> u64 {
        let bl = self.buf.len();
        const SZ: usize = core::mem::size_of::<u64>();
        if bl >= SZ {
            unsafe {
                let p = self.buf.as_ptr();
                self.buf = &self.buf[SZ..];
                (p as *const u64).read_unaligned()
            }
        } else if bl > 0 {
            let mut ibuf = [0u8; SZ];
            let xl = core::cmp::min(bl, SZ);
            ibuf[..xl].copy_from_slice(&self.buf[..xl]);
            self.buf = &self.buf[xl..];
            u64::from_le_bytes(ibuf)
        } else {
            0
        }
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let dl = dest.len();
        let l = core::cmp::min(dl, self.buf.len());
        dest[..l].copy_from_slice(&self.buf[..l]);
        self.buf = &self.buf[l..];
        if l < dl {
            dest[l..dl].fill(0);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn it_works() {
        let buf = [
            1u8, 1u8, 0u8, 0u8, // first next_u32()
            1u8, 1u8, 1u8, 1u8, 3,
        ];

        let mut rng = BufRng::new(&buf);

        let i: u32 = rng.next_u32();
        assert_eq!(i, (1 << 8) + 1);

        let i: u8 = rng.gen();
        assert_eq!(i, 1);

        let b: bool = rng.gen_bool(0.5);
        assert_eq!(b, true);

        for _ in 0..16 {
            let i: u16 = rng.gen();
            assert_eq!(i, 0);
            let i: u16 = rng.gen();
            assert_eq!(i, 0);
            let i: u16 = rng.gen();
            assert_eq!(i, 0);
        }
        let i: u64 = rng.gen();
        assert_eq!(i, 0);
    }
}
