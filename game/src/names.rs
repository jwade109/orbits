use starling::math::randint;
use std::error::Error;
use std::fs::read_to_string;
use std::path::Path;

pub fn load_names_from_file(filename: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(read_to_string(filename)?
        .lines()
        .map(|s| s.to_string())
        .collect())
}

pub fn get_random_ship_name(names: &Vec<String>) -> String {
    if names.is_empty() {
        return String::new();
    }
    let idx = randint(0, names.len() as i32) as usize;
    names[idx].clone()
}
