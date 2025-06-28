use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use flate2::write::DeflateEncoder;
use flate2::Compression;
use crc32fast::Hasher;

/// Write a gzip-compressed file with a custom header, including mtime and original filename.
///
/// # Arguments
/// * `output_path` - Path to write the `.gz` file to.
/// * `mtime` - UNIX timestamp (seconds since epoch) for the `mtime` field.
/// * `data` - The raw (uncompressed) data to be written.
///
/// # Returns
/// `io::Result<()>`
pub fn write_gzip_file(
    output_path: &str,
    mtime: u32,
    data: &[u8],
) -> io::Result<()> {
    let mut out = File::create(output_path)?;

    // === GZIP HEADER ===
    out.write_all(&[0x1f, 0x8b, 0x08])?; // ID1, ID2, Compression method: DEFLATE (8)

    let filename = Path::new(&output_path).file_name().unwrap().to_str().unwrap();
    let mut flags = 0u8;

    flags |= 0b0000_1000; // FNAME

    out.write_all(&[flags])?;                  // FLG
    out.write_all(&mtime.to_le_bytes())?;      // MTIME
    out.write_all(&[0x00, 0x03])?;             // XFL, OS = Unix (3)

    out.write_all(filename.as_bytes())?;
    out.write_all(&[0x00])?; // Null terminator


    // === COMPRESSED BODY ===
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed_data = encoder.finish()?;

    out.write_all(&compressed_data)?;

    // === FOOTER: CRC32 + ISIZE ===
    let mut hasher = Hasher::new();
    hasher.update(data);
    let crc = hasher.finalize();
    out.write_all(&crc.to_le_bytes())?;

    let isize = (data.len() as u32).to_le_bytes();
    out.write_all(&isize)?;

    Ok(())
}
