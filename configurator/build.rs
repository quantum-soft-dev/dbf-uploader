fn main() {
    // Embed Windows resources (manifest for admin privileges)
    #[cfg(windows)]
    {
        let _ = embed_resource::compile("app.rc", embed_resource::NONE);
    }
}
