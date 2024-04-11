use std::{
    ffi::OsStr,
    fs::File,
    io::{Read, Write, Seek}, vec,
};
use serde::{Serialize, Deserialize};
use serde_jsonlines::write_json_lines;

#[derive(Serialize, Deserialize, Debug)]
struct FileData {
    name: String,
    filenames: Vec<String>
}

fn get_zip_contents(reader: impl Read + Seek) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut zip = zip::ZipArchive::new(reader)?;
    let mut filenames: Vec<String> = Vec::new();

    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        filenames.push(file.name().to_string());
    }

    Ok(filenames)
}

fn serialize_to<W: Write, T: ?Sized + Serialize>(mut writer: W, value: &T) -> Result<(), std::io::Error> {
    serde_json::to_writer(&mut writer, value)?;
    writer.write_all(b"\n")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut data: Vec<FileData> = Vec::new();

    let dir = &args[1];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("zip")) {
            let file = File::open(&path)?;
            let zip_name = path.file_name().unwrap().to_string_lossy().to_string();
            let filenames = get_zip_contents(file)?;

            println!("Found {} files in {}", filenames.len(), zip_name);

            data.push(FileData {
                name: zip_name,
                filenames: filenames
            });

        }
    }

    let mut out_file = File::create("output.json")?;

    for item in data {
        serialize_to(&mut out_file, &item)?;
    }

    Ok(())
}
