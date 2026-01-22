use anyhow::Result;
use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<()> {
    // Generate git information
    let git2 = Git2Builder::default()
        .branch(true)
        .sha(true)
        .build()?;

    // Emit instructions
    Emitter::default()
        .add_instructions(&git2)?
        .emit()?;

    Ok(())
}
