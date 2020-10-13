#[macro_use]
extern crate log;

#[macro_use(format)]
extern crate pf2e_csheet_shared;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use anyhow::Result;

mod network;
mod parsers;
mod resources;

#[tokio::main]
async fn main() -> Result<()> {
    use std::io::Write;

    pretty_env_logger::init_timed();

    const BASE_PATH: &'static str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/crb/classes/");
    for class_id in 8u8..=9 {
        info!("Loading class #{}...", class_id);
        let path = format!("/Classes.aspx?ID={}", class_id);
        let resource = resources::Aon2Page::from_path(&path)?;
        let (class, extra) = resource.as_single_shared_resource().await?;
        let class_name = class.common().name.as_str();
        let filename = format!("{}.yaml", class_name);
        let path = std::path::Path::new(BASE_PATH).join(filename.as_str());
        info!(
            "Saving {} class information to {}",
            class_name,
            path.display()
        );
        let combined = {
            let mut v = Vec::with_capacity(extra.len() + 1);
            v.push(class);
            v.extend(extra);
            v
        };
        let f = std::fs::File::create(path)?;
        let mut f = std::io::BufWriter::new(f);
        serde_yaml::to_writer(&mut f, &combined)?;
        f.flush()?;
    }

    Ok(())
}
