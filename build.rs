//! Build script — emits `VERGEN_GIT_DESCRIBE` from git tags.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let git2 = vergen_git2::Git2::builder()
        .describe(true, true, None)
        .build();

    vergen_git2::Emitter::default()
        .add_instructions(&git2)?
        .emit()?;

    Ok(())
}
