#[derive(Default)]
pub struct Shader {
    #[allow(unused)]
    imports: Vec<String>,
    pub contents: String,
    pub vertex_entry: String,
    pub fragment_entry: String,
}

impl Shader {
    pub fn from_source(contents: &str) -> Self {
        let re = regex::Regex::new(r#"import\(\"([\w\.]+)\"\)"#).unwrap();

        let mut imports = vec![];

        for line in contents.lines() {
            if let Some(cap) = re.captures(line) {
                if let Some(import) = cap.get(1).map(|e| e.as_str()) {
                    imports.push(import.to_string());
                }
            }
        }

        Self {
            imports,
            contents: contents.to_string(),
            vertex_entry: "vs_main".to_string(),
            fragment_entry: "fs_main".to_string(),
        }
    }

    pub fn from_path(path: &str) -> Self {
        let re = regex::Regex::new(r#"import\(\"([\w\.]+)\"\)"#).unwrap();

        let common_code = std::fs::read_to_string("crates/rend/shaders/common.wgsl").unwrap();
        let source_code = std::fs::read_to_string(path).expect("Can't read source code!");

        let source_code = format!("{common_code}\n\n{source_code}");

        let mut imports = vec![];

        for line in source_code.lines() {
            if let Some(cap) = re.captures(line) {
                if let Some(import) = cap.get(1).map(|e| e.as_str()) {
                    // println!("Shader {path} imports {import}");
                    imports.push(import.to_string());
                }
            }
        }

        Self {
            imports,
            contents: source_code,
            vertex_entry: "vs_main".to_string(),
            fragment_entry: "fs_main".to_string(),
        }
    }
}
