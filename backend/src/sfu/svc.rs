use std::sync::atomic::{AtomicBool, AtomicU16, Ordering::Relaxed};

#[derive(Debug, Clone, Copy, Default)]
pub struct Vp9Desc {
    pub i: bool,
    pub p: bool,
    pub l: bool,
    pub f: bool,
    pub b: bool,
    pub e: bool,
    pub v: bool,
    pub picture_id: Option<u16>,
    pub tid: u8,
    pub u: bool,
    pub sid: u8,
    pub d: bool,
    pub tl0picidx: Option<u8>,
}

pub fn parse_vp9(payload: &[u8]) -> Option<Vp9Desc> {
    let (&b0, mut rest) = payload.split_first()?;
    let bit = |m: u8| b0 & m != 0;

    let mut d = Vp9Desc {
        i: bit(0x80),
        p: bit(0x40),
        l: bit(0x20),
        f: bit(0x10),
        b: bit(0x08),
        e: bit(0x04),
        v: bit(0x02),
        ..Default::default()
    };

    if d.i {
        let (&b, r) = rest.split_first()?;
        if b & 0x80 != 0 {
            let (&b2, r) = r.split_first()?;
            d.picture_id = Some(u16::from_be_bytes([b & 0x7f, b2]));
            rest = r;
        } else {
            d.picture_id = Some(b as u16);
            rest = r;
        }
    }

    if d.l {
        let (&b, r) = rest.split_first()?;
        d.tid = b >> 5;
        d.u = b & 0x10 != 0;
        d.sid = (b >> 1) & 0x07;
        d.d = b & 0x01 != 0;
        if !d.f {
            d.tl0picidx = Some(*r.first()?);
        }
    }

    Some(d)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Drop,
    DropNeedKeyframe,
    Forward { seq: u16, marker: bool },
}

#[inline]
fn pack(sid: u8, tid: u8) -> u16 {
    u16::from_be_bytes([sid, tid])
}

#[inline]
fn unpack(v: u16) -> (u8, u8) {
    let [sid, tid] = v.to_be_bytes();
    (sid, tid)
}

pub struct LayerSelector {
    target: AtomicU16,
    current: AtomicU16,
    started: AtomicBool,
    dropped: AtomicU16,
}

impl LayerSelector {
    pub fn new(sid: u8, tid: u8) -> Self {
        let v = pack(sid, tid);
        Self {
            target: AtomicU16::new(v),
            current: AtomicU16::new(v),
            started: AtomicBool::new(false),
            dropped: AtomicU16::new(0),
        }
    }

    pub fn base_only() -> Self {
        Self::new(0, 0)
    }

    pub fn set_target(&self, sid: u8, tid: u8) {
        self.target.store(pack(sid, tid), Relaxed);
    }

    pub fn target(&self) -> (u8, u8) {
        unpack(self.target.load(Relaxed))
    }

    pub fn current(&self) -> (u8, u8) {
        unpack(self.current.load(Relaxed))
    }

    pub fn wants_keyframe(&self) -> bool {
        !self.started.load(Relaxed) || self.target().0 > self.current().0
    }

    pub fn skip(&self) {
        if self.started.load(Relaxed) {
            self.dropped.fetch_add(1, Relaxed);
        }
    }

    pub fn select(&self, seq: u16, marker: bool, d: &Vp9Desc) -> Verdict {
        let (sid, tid) = if d.l { (d.sid, d.tid) } else { (0, 0) };
        let (t_sid, t_tid) = self.target();

        if !self.started.load(Relaxed) {
            if !(d.b && !d.p && sid == 0) {
                return Verdict::DropNeedKeyframe;
            }
            self.dropped.store(0, Relaxed);
            self.current.store(pack(t_sid, t_tid), Relaxed);
            self.started.store(true, Relaxed);
        }

        if d.b && sid == 0 {
            let (mut c_sid, mut c_tid) = self.current();

            if t_sid < c_sid || (t_sid > c_sid && !d.p) {
                c_sid = t_sid;
            }

            if t_tid < c_tid {
                c_tid = t_tid;
            } else if t_tid > c_tid && d.u && tid > c_tid && tid <= t_tid {
                c_tid = tid;
            }

            self.current.store(pack(c_sid, c_tid), Relaxed);
        }

        let (c_sid, c_tid) = self.current();
        if sid > c_sid || tid > c_tid {
            self.skip();
            return Verdict::Drop;
        }

        Verdict::Forward {
            seq: seq.wrapping_sub(self.dropped.load(Relaxed)),
            marker: marker || (d.e && sid == c_sid),
        }
    }
}
