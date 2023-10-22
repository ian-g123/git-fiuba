use std::{
    fs::File,
    io::{Error, Read},
};

fn main() {
    let (path, data) = read_file("e80e8fbbd85cdf883054fdd45103d2f43a195870").unwrap();
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
