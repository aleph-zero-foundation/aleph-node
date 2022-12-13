use std::{
    fs::{self, File},
    io::{ErrorKind, Write},
};

use log::info;
use serde_json::Value;

use crate::Storage;

pub fn write_to_file(write_to_path: String, data: &[u8]) {
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&write_to_path)
    {
        Ok(file) => file,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => match File::create(&write_to_path) {
                Ok(file) => file,
                Err(why) => panic!("Cannot create file: {:?}", why),
            },
            _ => panic!("Unexpected error when creating file: {}", &write_to_path),
        },
    };

    file.write_all(data).expect("Could not write to file");
}

pub fn read_json_from_file(path: String) -> Value {
    let content = file_content(path);
    serde_json::from_str(&content).expect("Could not deserialize file to json format")
}

pub fn file_content(path: String) -> String {
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("Could not read file: `{}`", path))
}

pub fn save_snapshot_to_file(snapshot: Storage, path: String) {
    let data = serde_json::to_vec_pretty(&snapshot).unwrap();
    info!(
        "Writing snapshot of {} key-val pairs and {} total bytes",
        snapshot.len(),
        data.len()
    );
    write_to_file(path, &data);
}

pub fn read_snapshot_from_file(path: String) -> Storage {
    let snapshot: Storage =
        serde_json::from_str(&fs::read_to_string(path).expect("Could not read snapshot file"))
            .expect("could not parse from snapshot");
    info!("Read snapshot of {} key-val pairs", snapshot.len());
    snapshot
}
