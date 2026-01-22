use anyhow::Result;
use vergen_git2::{CargoBuilder, Emitter, Git2Builder, RustcBuilder};

fn main() -> Result<()> {
    // Generate build timestamp
    let build = CargoBuilder::default().build_timestamp(true).build()?;

    // Generate git information
    let git2 = Git2Builder::default()
        .branch(true)
        .sha(true)
        .build()?;

    // Generate rustc information
    let rustc = RustcBuilder::default().semver(true).build()?;

    // Emit all instructions
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&git2)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
