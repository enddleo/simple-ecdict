fn main() {
    #[cfg(windows)]
    embed_resource::compile("version.rc", embed_resource::NONE);
}
