//! A small ZIP writer: what an EPUB needs and no more. The first entry may
//! be stored (the EPUB `mimetype`), the rest are deflated.

use std::io::Write;

pub struct Zip {
    out: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl Zip {
    pub fn new() -> Self {
        Zip { out: Vec::new(), central: Vec::new(), count: 0 }
    }

    /// Adds a file stored as it is.
    pub fn stored(&mut self, name: &str, data: &[u8]) {
        self.entry(name, data, data.to_vec(), 0);
    }

    /// Adds a file compressed with deflate.
    pub fn deflated(&mut self, name: &str, data: &[u8]) {
        let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        let packed = match enc.write_all(data).and_then(|_| enc.finish()) {
            Ok(p) if p.len() < data.len() => p,
            _ => return self.stored(name, data),
        };
        self.entry(name, data, packed, 8);
    }

    fn entry(&mut self, name: &str, data: &[u8], packed: Vec<u8>, method: u16) {
        let crc = crc32fast::hash(data);
        let offset = self.out.len() as u32;
        // A fixed timestamp (1 Jan 2020), so builds are reproducible.
        let (time, date) = (0u16, ((2020 - 1980) << 9 | 1 << 5 | 1) as u16);
        let mut local = Vec::new();
        local.extend(0x04034b50u32.to_le_bytes());
        local.extend(20u16.to_le_bytes()); // version needed
        local.extend(0u16.to_le_bytes()); // flags
        local.extend(method.to_le_bytes());
        local.extend(time.to_le_bytes());
        local.extend(date.to_le_bytes());
        local.extend(crc.to_le_bytes());
        local.extend((packed.len() as u32).to_le_bytes());
        local.extend((data.len() as u32).to_le_bytes());
        local.extend((name.len() as u16).to_le_bytes());
        local.extend(0u16.to_le_bytes()); // no extra field
        local.extend(name.as_bytes());
        self.out.extend(&local);
        self.out.extend(&packed);

        self.central.extend(0x02014b50u32.to_le_bytes());
        self.central.extend(20u16.to_le_bytes()); // made by
        self.central.extend(20u16.to_le_bytes()); // needed
        self.central.extend(0u16.to_le_bytes());
        self.central.extend(method.to_le_bytes());
        self.central.extend(time.to_le_bytes());
        self.central.extend(date.to_le_bytes());
        self.central.extend(crc.to_le_bytes());
        self.central.extend((packed.len() as u32).to_le_bytes());
        self.central.extend((data.len() as u32).to_le_bytes());
        self.central.extend((name.len() as u16).to_le_bytes());
        self.central.extend([0u8; 8]); // extra and comment lengths, disk, internal attributes
        self.central.extend(0u32.to_le_bytes()); // external attributes
        self.central.extend(offset.to_le_bytes());
        self.central.extend(name.as_bytes());
        self.count += 1;
    }

    pub fn finish(mut self) -> Vec<u8> {
        let start = self.out.len() as u32;
        let size = self.central.len() as u32;
        self.out.extend(&self.central);
        self.out.extend(0x06054b50u32.to_le_bytes());
        self.out.extend([0u8; 4]); // disk numbers
        self.out.extend(self.count.to_le_bytes());
        self.out.extend(self.count.to_le_bytes());
        self.out.extend(size.to_le_bytes());
        self.out.extend(start.to_le_bytes());
        self.out.extend(0u16.to_le_bytes()); // no comment
        self.out
    }
}
