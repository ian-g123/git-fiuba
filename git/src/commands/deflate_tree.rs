use std::{
    fs::File,
    io::{Error, Read},
};

fn main() {
    let (path, data) = read_file("3ff2458ff44532b18df91b1e96c524caae26c4c6").unwrap();
    println!("{:?}", String::from_utf8_lossy(&data));
}

fn read_file(hash_str: &str) -> Result<(String, Vec<u8>), Error> {
    let path = format!(".git/objects/{}/{}", &hash_str[0..2], &hash_str[2..]);
    let mut file = File::open(&path).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    println!("{:?}", data);
    let decompressed_data = extract(&data)?;
    Ok((path, decompressed_data))
}

fn extract(data: &[u8]) -> Result<Vec<u8>, Error> {
    // Extract data with zlib
    let mut decoder = flate2::read::ZlibDecoder::new(data);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

// commit 159\0tree 8fd63c209c7d3ae4abd6c8748398f587d406720a\n
// author ian-g123 <igauler@fi.uba.ar> 1698097185 -0300\n
// committer ian-g123 <igauler@fi.uba.ar> 1698097185 -0300\n\n
// pr\n"


// como se imprime el commit que tiene 2 commits \\
// tree ca4e2e939bef7f819b89052a544e3f41cfe88cbb
// parent c0d1375063696b43e36c135e6690e154fc26fc49
// author ian-g123 <igauler@fi.uba.ar> 1698101274 -0300
// committer ian-g123 <igauler@fi.uba.ar> 1698101274 -0300

// 2commits

// tree 95\0100644 baz\00�M%�B��U\u{12}���tV��\u{6}�040000 dir00\0��}u+��\u{16}�.��D$�Iʢ+�100644 sofi\0\u{18}\u{c}�2�\"��骢Wz���+�8'
