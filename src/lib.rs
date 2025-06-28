use bytes::Bytes;
use crab_nbt::{Nbt, NbtCompound, NbtTag};
use pyo3::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::time::{SystemTime, UNIX_EPOCH};
mod gzip;

fn parse_key(key: &str) -> Option<(i32, i32, i32)> {
    // Expect a key like "(0, 6, 23)"
    let trimmed = key.trim_matches(|c| c == '(' || c == ')');
    let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();

    if parts.len() == 3 {
        let x = parts[0].parse().ok()?;
        let y = parts[1].parse().ok()?;
        let z = parts[2].parse().ok()?;
        Some((x, y, z))
    } else {
        None
    }
}

pub fn load_json(path: &str) -> HashMap<(i32, i32, i32), String> {
    let data = read_to_string(path).unwrap();
    let raw_map: HashMap<String, String> = serde_json::from_str(&data).unwrap();

    let mut result = HashMap::new();

    for (key, value) in raw_map {
        if let Some(tuple_key) = parse_key(&key) {
            result.insert(tuple_key, value);
        }
    }
    result
}

fn get_block(x: i32, y: i32, z: i32, state: i32) -> NbtTag {
    return NbtTag::from(NbtCompound::from_iter([
        (
            "pos".to_owned(),
            NbtTag::List(vec![NbtTag::from(x), NbtTag::from(y), NbtTag::from(z)]),
        ),
        ("state".to_owned(), NbtTag::from(state)),
    ]));
}

pub fn to_schem(blocks_dict: HashMap<(i32, i32, i32), String>) -> Nbt {
    let xs = blocks_dict.keys().map(|xyz| xyz.0);
    let ys = blocks_dict.keys().map(|xyz| xyz.1);
    let zs = blocks_dict.keys().map(|xyz| xyz.2);

    let min_x = xs.clone().min().unwrap();
    let max_x = xs.max().unwrap();
    let min_y = ys.clone().min().unwrap();
    let max_y = ys.max().unwrap();
    let min_z = zs.clone().min().unwrap();
    let max_z = zs.max().unwrap();

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let length = max_z - min_z + 1;

    let mut palette_map: HashMap<String, i32> = HashMap::new();
    let mut palette_compound = NbtCompound::new();
    palette_map.insert("minecraft:air".to_string(), 0);
    palette_compound.put("minecraft:air".to_string(), 0);
    let mut i = 1;
    for block in blocks_dict.values() {
        if !palette_map.contains_key(block) {
            palette_map.insert(block.to_owned(), i);
            palette_compound.put(block.to_string(), i);
            i += 1;
        }
    }

    let mut blocks: Vec<u8> = vec![0; (width * height * length) as usize];
    for (k, v) in blocks_dict.iter() {
        // k.1 and k.2 swapped y -> z
        let index = k.0 + width * (k.2 + length * k.1);
        blocks[index as usize] = *palette_map.get(v).unwrap() as u8;
    }
    let bytes = Bytes::from(blocks);

    let mut inner = NbtCompound::new();
    inner.put("Version".to_owned(), 3);
    inner.put("DataVersion".to_owned(), 4325);
    inner.put("Width".to_owned(), NbtTag::Short(width as i16));
    inner.put("Height".to_owned(), NbtTag::Short(height as i16));
    inner.put("Length".to_owned(), NbtTag::Short(length as i16));

    let mut blocks_inner = NbtCompound::new();
    blocks_inner.put("BlockEntities".to_owned(), NbtTag::List(vec![]));
    blocks_inner.put("Palette".to_owned(), palette_compound);
    blocks_inner.put("Data".to_owned(), NbtTag::from(bytes));
    inner.put("Blocks".to_owned(), blocks_inner);

    let mut schem = NbtCompound::new();
    schem.put("Schematic".to_owned(), inner);

    Nbt::new("".to_owned(), schem)
}

pub fn to_structure(blocks_dict: HashMap<(i32, i32, i32), String>) -> Nbt {
    let xs = blocks_dict.keys().map(|xyz| xyz.0);
    let ys = blocks_dict.keys().map(|xyz| xyz.1);
    let zs = blocks_dict.keys().map(|xyz| xyz.2);

    let min_x = xs.clone().min().unwrap();
    let max_x = xs.max().unwrap();
    let min_y = ys.clone().min().unwrap();
    let max_y = ys.max().unwrap();
    let min_z = zs.clone().min().unwrap();
    let max_z = zs.max().unwrap();

    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let length = max_z - min_z + 1;

    let mut palette_map: HashMap<String, i32> = HashMap::new();
    let mut palette_vec: Vec<NbtTag> = Vec::new();
    palette_map.insert("minecraft:air".to_string(), 0);
    palette_vec.push(NbtTag::from(NbtCompound::from_iter([(
        "Name".to_owned(),
        NbtTag::from("minecraft:air".to_owned()),
    )])));
    let mut i = 1;
    for block in blocks_dict.values() {
        if !palette_map.contains_key(block) {
            palette_map.insert(block.to_owned(), i);
            i += 1;
            palette_vec.push(NbtTag::from(NbtCompound::from_iter([(
                "Name".to_owned(),
                NbtTag::from(block.to_owned()),
            )])));
        }
    }

    let blocks: Vec<NbtTag> = (0..width)
        .into_par_iter()
        .flat_map_iter(|x| {
            let mut local_blocks = Vec::with_capacity((height * length) as usize); // preallocate
            for y in 0..height {
                for z in 0..length {
                    let palette_id = if let Some(block_name) = blocks_dict.get(&(x, y, z)) {
                        *palette_map.get(block_name).unwrap_or(&0)
                    } else {
                        0
                    };
                    local_blocks.push(get_block(x, y, z, palette_id));
                }
            }
            local_blocks
        })
        .collect();

    let mut inner = NbtCompound::new();
    inner.put("blocks".to_owned(), blocks);
    inner.put("entities".to_owned(), NbtTag::List(vec![]));
    inner.put("palette".to_owned(), palette_vec);
    inner.put(
        "size".to_owned(),
        vec![
            NbtTag::from(width),
            NbtTag::from(height),
            NbtTag::from(length),
        ],
    );
    inner.put("DataVersion".to_owned(), 3953);

    Nbt::new("".to_owned(), inner)
}

pub fn save_nbt(nbt: Nbt, filename: &str) {
    let buf = nbt.write();
    let mtime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    gzip::write_gzip_file(filename, mtime as u32, &buf).unwrap();
}

fn to_file(blocks_dict: HashMap<(i32, i32, i32), String>, filename: &str, mode: &str) {
    let nbt = match mode {
        "schem" => Some(to_schem(blocks_dict)),
        "structure" => Some(to_structure(blocks_dict)),
        _ => None,
    };

    save_nbt(nbt.expect(&format!("Invalid mode: {}", mode)), filename);
}

#[pyfunction]
fn export(py_dict: HashMap<(i32, i32, i32), String>, path: &str, mode: &str) -> PyResult<()> {
    to_file(py_dict, path, mode);
    Ok(())
}

#[pymodule]
fn nbtexport(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(export, m)?)?;
    Ok(())
}
