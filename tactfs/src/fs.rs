use std::path::PathBuf;

pub struct FileSystem {
    path : PathBuf,
    branch : String,
    build_key : String,
    cdn_key : String,
    install_key : String,
    im_size : u32,
    cdn_path : String,
    cdn_host : String,
    cdn_servers : Vec<String>,
    tags : Vec<String>,
    armadillo : String,
    last_activated : String,
    version : String,
    keyring : String,
    product : String,
}
impl FileSystem {
    pub fn open(path : PathBuf) {
        
    }
}

pub struct FileSystemProvider;
impl FileSystemProvider {
    pub fn enumerate(root : PathBuf) {

    }
}