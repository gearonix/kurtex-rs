use std::collections::HashMap;
use std::hash::Hash;
use std::path::PathBuf;

#[derive(Debug)]
pub struct RuntimeManager;

#[derive(Debug, Clone)]
pub struct RuntimeOptions<'a> {
    pub root: &'a PathBuf,
    pub files: Vec<PathBuf>,
}

impl RuntimeManager {
    pub fn start(opts: &RuntimeOptions) {
        let root_dir = &opts.root;

        let mut __pending_modules__: HashMap<String, String> = HashMap::new();
    }
}
