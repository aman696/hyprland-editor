//! Bounded sealed raw RGBA input. Pixel data never confers compositor readiness.
//! No pathname, image decoder, mmap, DMA-BUF or host display is involved.
use std::{
    collections::VecDeque,
    fs::File,
    io::{self, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{FileExt, MetadataExt},
    },
    time::{Duration, Instant},
};
const MAGIC: &[u8; 8] = b"HYEFRA01";
const HEADER: usize = 64;
pub const MAX_PIXELS: usize = 1280 * 720;
pub const MAX_FRAME_BYTES: usize = MAX_PIXELS * 4;
pub const MAX_RETAINED_FRAMES: usize = 2;
const SEALS: i32 = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
const INTERVAL: Duration = Duration::from_nanos(66_666_667);
fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
/// Correlation for one live preview session; not a saved readiness certificate.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Session([u8; 16]);
impl Session {
    pub fn new() -> io::Result<Self> {
        let mut bytes = [0; 16];
        // SAFETY: fixed live output buffer; no retained pointers.
        let count = unsafe { libc::getrandom(bytes.as_mut_ptr().cast(), bytes.len(), 0) };
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        if count != bytes.len() as isize {
            return Err(invalid("Incomplete frame session identity"));
        }
        Ok(Self(bytes))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}
impl Dimensions {
    fn byte_len(self) -> io::Result<usize> {
        // Both orientations are supported within the initial 720p pixel budget.
        if self.width == 0 || self.height == 0 || self.width > 1280 || self.height > 1280 {
            return Err(invalid("Frame dimensions exceed the initial profile"));
        }
        let pixels = (self.width as usize)
            .checked_mul(self.height as usize)
            .filter(|v| *v <= MAX_PIXELS)
            .ok_or_else(|| invalid("Frame pixel budget exceeded"))?;
        pixels
            .checked_mul(4)
            .ok_or_else(|| invalid("Frame size overflow"))
    }
}
fn encode(
    session: Session,
    generation: u64,
    sequence: u64,
    size: Dimensions,
) -> io::Result<[u8; HEADER]> {
    let bytes = size.byte_len()?;
    if generation == 0 || sequence == 0 {
        return Err(invalid("Frame generation and sequence must be nonzero"));
    }
    let mut header = [0; HEADER];
    header[..8].copy_from_slice(MAGIC);
    header[8..24].copy_from_slice(&session.0);
    header[24..32].copy_from_slice(&generation.to_le_bytes());
    header[32..40].copy_from_slice(&sequence.to_le_bytes());
    header[40..44].copy_from_slice(&size.width.to_le_bytes());
    header[44..48].copy_from_slice(&size.height.to_le_bytes());
    header[48..52].copy_from_slice(&(size.width * 4).to_le_bytes());
    // Format 1: tightly packed straight-alpha RGBA8888, sRGB encoded channels.
    header[52..56].copy_from_slice(&1u32.to_le_bytes());
    header[56..64].copy_from_slice(&(bytes as u64).to_le_bytes());
    Ok(header)
}
/// Trusted broker-side builder. Capture format conversion is a separate obligation.
/// The returned descriptor has immutable contents and immutable length.
pub fn seal_rgba(
    session: Session,
    generation: u64,
    sequence: u64,
    size: Dimensions,
    pixels: &[u8],
) -> io::Result<OwnedFd> {
    let header = encode(session, generation, sequence, size)?;
    if pixels.len() != size.byte_len()? {
        return Err(invalid("Raw frame byte count differs from its dimensions"));
    }
    // SAFETY: static nul-terminated name, fixed flags, no input path.
    let raw = unsafe {
        libc::memfd_create(
            c"hye-frame".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful memfd_create returned a fresh descriptor.
    let mut file = unsafe { File::from_raw_fd(raw) };
    file.write_all(&header)?;
    file.write_all(pixels)?;
    // SAFETY: fcntl uses a live descriptor and scalar sealing flags.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, SEALS) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file.into())
}
#[derive(Debug)]
pub struct Frame {
    pub generation: u64,
    pub sequence: u64,
    pub size: Dimensions,
    pixels: Vec<u8>,
}
impl Frame {
    /// Straight-alpha, tightly packed RGBA8888 with sRGB encoded channels.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}
/// Two owned pixel buffers, for one session and source generation. Construct a
/// new receiver when the projection changes. Clearing pixels does not reset the
/// monotonic sequence guard. No report here can certify a rendered lab profile.
pub struct Receiver {
    session: Session,
    generation: u64,
    sequence: u64,
    last: Option<Instant>,
    frames: VecDeque<Frame>,
}
impl Receiver {
    pub fn new(session: Session, generation: u64) -> io::Result<Self> {
        if generation == 0 {
            return Err(invalid("Frame generation must be nonzero"));
        }
        Ok(Self {
            session,
            generation,
            sequence: 0,
            last: None,
            frames: VecDeque::with_capacity(MAX_RETAINED_FRAMES),
        })
    }
    pub fn latest(&self) -> Option<&Frame> {
        self.frames.back()
    }
    pub fn retained_bytes(&self) -> usize {
        self.frames.iter().map(|f| f.pixels.len()).sum()
    }
    pub fn clear(&mut self) {
        self.frames.clear();
    }
    /// Consume a received descriptor, including every refusal path. The caller
    /// must separately authenticate its private channel and bound queued FDs.
    /// A saturated 15 FPS receiver refuses before allocating another pixel buffer.
    pub fn accept(&mut self, descriptor: OwnedFd) -> io::Result<()> {
        self.accept_at(descriptor, Instant::now())
    }
    fn accept_at(&mut self, descriptor: OwnedFd, now: Instant) -> io::Result<()> {
        let file = File::from(descriptor);
        // Check immutable data/length *before* sampling metadata. A producer
        // could otherwise change the length between an early fstat and sealing.
        // SAFETY: fcntl reads seal flags from a live descriptor.
        let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
        if seals < 0 || seals & SEALS != SEALS {
            return Err(invalid("Frame data and length must be permanently sealed"));
        }
        let info = file.metadata()?;
        if info.mode() & libc::S_IFMT != libc::S_IFREG
            || info.len() < HEADER as u64
            || info.len() > (HEADER + MAX_FRAME_BYTES) as u64
        {
            return Err(invalid(
                "Frame descriptor type or length is outside its bound",
            ));
        }
        let mut header = [0; HEADER];
        file.read_exact_at(&mut header, 0)?;
        let generation = u64::from_le_bytes(header[24..32].try_into().expect("Fixed slice"));
        let sequence = u64::from_le_bytes(header[32..40].try_into().expect("Fixed slice"));
        let size = Dimensions {
            width: u32::from_le_bytes(header[40..44].try_into().expect("Fixed slice")),
            height: u32::from_le_bytes(header[44..48].try_into().expect("Fixed slice")),
        };
        let count = size.byte_len()?;
        if header != encode(self.session, self.generation, sequence, size)?
            || generation != self.generation
            || sequence <= self.sequence
            || info.len() != (HEADER + count) as u64
        {
            return Err(invalid(
                "Frame header, session, generation, sequence or exact length differs",
            ));
        }
        if self.last.is_some_and(|last| {
            now.checked_duration_since(last)
                .is_none_or(|elapsed| elapsed < INTERVAL)
        }) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "Preview frame rate is saturated",
            ));
        }
        // Discard the oldest before allocation, retaining the latest valid frame
        // if allocation/read fails. Never retain three full owned pixel buffers.
        if self.frames.len() == MAX_RETAINED_FRAMES {
            self.frames.pop_front();
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| io::Error::other("Frame allocation failed"))?;
        pixels.resize(count, 0);
        file.read_exact_at(&mut pixels, HEADER as u64)?;
        self.frames.push_back(Frame {
            generation,
            sequence,
            size,
            pixels,
        });
        self.sequence = sequence;
        self.last = Some(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_deadline_and_backwards_clock_refusals_do_not_consume_sequence_or_pixels() {
        let session = Session::new().unwrap();
        let size = Dimensions {
            width: 1,
            height: 1,
        };
        let mut receiver = Receiver::new(session, 1).unwrap();
        let start = Instant::now();
        receiver
            .accept_at(seal_rgba(session, 1, 1, size, &[1; 4]).unwrap(), start)
            .unwrap();
        for at in [
            start,
            start + INTERVAL - Duration::from_nanos(1),
            start - Duration::from_nanos(1),
        ] {
            assert_eq!(
                receiver
                    .accept_at(seal_rgba(session, 1, 2, size, &[2; 4]).unwrap(), at)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::WouldBlock
            );
            assert_eq!(receiver.latest().unwrap().sequence, 1);
            assert_eq!(receiver.latest().unwrap().pixels(), &[1; 4]);
        }
        receiver
            .accept_at(
                seal_rgba(session, 1, 2, size, &[2; 4]).unwrap(),
                start + INTERVAL,
            )
            .unwrap();
        assert_eq!(receiver.latest().unwrap().sequence, 2);
        assert_eq!(receiver.retained_bytes(), 8);
    }
}
