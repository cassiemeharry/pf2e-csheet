#![deny(bindings_with_variant_name)]
#![deny(unreachable_patterns)]
#![feature(associated_type_defaults)]
// TODO: remove this after the prototype phase is complete.
#![allow(dead_code)]

#[macro_use]
extern crate log;

#[macro_use(format)]
extern crate pf2e_csheet_shared;

#[macro_use]
extern crate rocket;
// #[macro_use]
// extern crate rocket_contrib;

use anyhow::Result;
use pf2e_csheet_shared::storage::ResourceStorage;
use std::{fs::File, path::Path};

// mod character;
mod resources;
mod server;

async fn load_resources(
    storage: &mut dyn ResourceStorage,
    filename: impl AsRef<Path>,
) -> Result<()> {
    let filename = std::fs::canonicalize(filename)?;
    info!("Loading resources from {}", filename.display());
    let mut f = File::open(filename)?;
    let rs: Vec<resources::Resource> = serde_yaml::from_reader(&mut f)?;
    for r in rs {
        debug!("Got CRB resource: {}", r.common().name);
        debug!("Serializing it back");
        let serialized = serde_yaml::to_string(&r).unwrap();
        match serde_yaml::from_str::<resources::Resource>(&serialized) {
            Ok(back) => assert_eq!(&r, &back),
            Err(e) => {
                panic!(
                    "Failed to round-trip resource: {} It serialized to this:\n--------------------\n{}\n--------------------\n",
                    e, serialized,
                );
            }
        }
        debug!("Done re-serializing the resource");
        // debug!("Resource details: {:#?}", r);
        storage
            .register(r)
            .await
            .map_err(|e| anyhow::Error::msg(e))?;
    }

    Ok(())
}

async fn load_sourcebook(
    storage: &mut dyn ResourceStorage,
    directory: impl AsRef<Path>,
) -> Result<()> {
    let d = std::fs::canonicalize(directory)?;
    anyhow::ensure!(
        d.is_dir(),
        "Source book directory \"{}\" doesn't exist",
        d.display()
    );
    load_resources(storage, dbg!(d.join("Monk.yaml"))).await?;
    load_resources(storage, dbg!(d.join("Ranger.yaml"))).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init_timed();

    let mut storage = resources::ResourceStore::new();
    let resources_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/crb/classes");
    load_sourcebook(&mut storage, dbg!(resources_dir)).await?;

    server::serve(storage).await?;

    // debug!("this is a debug message!");

    // trace!("this is a trace message");
    // info!("Loading character...");

    // let character: character::Character = {
    //     let mut f = File::open("nadia.yaml")?;
    //     serde_yaml::from_reader(&mut f)?
    // };
    // // let mut character: character::Character = character::Character::new();
    // // character.name = "Nadia Redmane".into();
    // // let monk_class = resources::ResourceRef::new_from_name("Monk");
    // // character.add_resource(monk_class.clone());
    // // character.set_choice(&monk_class, "Level", "1");
    // let max_hp = character.get_modifier("Max HP", None);
    // println!("Found max HP: {}", max_hp);

    // let qs = character.get_unanswered_questions();
    // info!(
    //     "Found {} unanswered question{}:",
    //     qs.len(),
    //     if qs.len() != 1 { "s" } else { "" }
    // );
    // for q in qs {
    //     info!("  - {:?}", q);
    // }

    // {
    //     let output_filename = "Nadia Redmane - 01.pdf";
    //     info!("Saving character sheet to {:?}...", output_filename);
    //     let start = std::time::Instant::now();
    //     character.save_character_sheet::<raw_pdf_manip::PDF, _>(output_filename)?;
    //     let dt = start.elapsed();
    //     info!("Successfully saved character sheet in {:?}", dt);
    // }

    Ok(())
}
