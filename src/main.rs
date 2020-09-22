#![deny(bindings_with_variant_name)]
#![deny(unreachable_patterns)]
#![feature(associated_type_defaults)]
// TODO: remove this after the prototype phase is complete.
#![allow(unused)]

#[macro_use]
extern crate log;

use anyhow::{Context as _, Result};
use std::{fs::File, path::Path};

mod macros;

mod bonuses;
mod character;
mod parsers;
mod resources;
mod stats;

fn load_resource(filename: impl AsRef<Path>) -> Result<()> {
    let mut f = File::open(filename)?;
    let rs: Vec<resources::Resource> = serde_yaml::from_reader(&mut f)?;
    for r in rs {
        info!("Got CRB resource: {}", r.name());
        debug!("Resource details: {:#?}", r);
        r.register();
    }

    Ok(())
}

fn load_sourcebook(directory: impl AsRef<Path>) -> Result<()> {
    let d = directory.as_ref();
    anyhow::ensure!(
        d.is_dir(),
        "Source book directory \"{}\" doesn't exist",
        d.display()
    );
    let path = d.join("monk.yaml");
    load_resource(path)
}
//     macro_rules! load {
//         ($($t:ty => $filename:literal ,)*) => {
//             $(
//                 let path = d.join(concat!($filename, ".yaml"));
//                 if path.is_file() {
//                     load_resource::<$t, _>(&path).with_context(|| format!("Error parsing source book file {}", path.display()))?;
//                 } else {
//                     warn!("Source path {} doesn't exist", path.display());
//                 }
//             )*
//         };
//     }

//     use resources::*;

//     load! {
//         Ancestry => "ancestries",
//         Background => "backgrounds",
//         Class => "classes",
//         Feat => "feats",
//         Heritage => "heritages",
//         Item => "items",
//     }

//     Ok(())
// }

fn main() -> Result<()> {
    pretty_env_logger::init_timed();

    load_sourcebook("resources/crb")?;

    let character: character::Character = {
        let mut f = std::fs::File::open("nadia.yaml")?;
        serde_yaml::from_reader(&mut f)?
    };

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
