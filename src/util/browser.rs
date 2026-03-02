use anyhow::Result;

pub fn open_url(url: &str) -> Result<()> {
    open::that(url)?;
    Ok(())
}
