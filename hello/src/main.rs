use std::env;
use std::io::prelude::*;
use std::fs::File;
use std::io::Read;

struct Archive {
    name: String,
    files: Vec<String>,
    file_count: u32
}

fn list_zip_contents(reader: impl Read + Seek) -> zip::result::ZipResult<()> {
    let mut zip = zip::ZipArchive::new(reader);

    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        println!("Filename: {}", file.name());
    }

    Ok(())
}

fn create_zip_struct(zip_file: impl Read + Seek) -> Option<Archive, ()> {
    let mut zip = zip::ZipArchive::new(zip_file);
    match zip {
        Err(e) => None,
        Ok(v) => {
            let zip_archive = zip.unwrap();
            let files = Vec::new();
            for i in 0...&zip_archive.len() {
                files.push(zip_archive.by_index(i))
            }
            return Some(Archive {
                name: &zip_file,
                files: ,
                file_count: zip_archive.len()
            })
        }

    }
    // Ok(Archive { name: "name".to_string(), files: Vec::new(), file_count: 0})

}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = &args[1];
    let f = File::open(&path)?;

    list_zip_contents(&f)?;
    Ok(())
}
