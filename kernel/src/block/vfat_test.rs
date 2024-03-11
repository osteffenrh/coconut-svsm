use super::pflash::Pflash;
use crate::address::PhysAddr;
use crate::utils::MemoryRegion;

use super::fat;
use super::pflash;
use fatfs::{
    DefaultTimeProvider, Dir, FileSystem, FsOptions, LossyOemCpConverter, NullTimeProvider, Write,
};

pub fn test_vfat(mem: MemoryRegion<PhysAddr>) {
    let mut dev = pflash::Pflash::new(mem.start(), mem.len());
    let w = fat::Wrap::<Pflash>::new(&mut dev);
    let f: FileSystem<fat::Wrap<'_, Pflash>, NullTimeProvider, LossyOemCpConverter> =
        FileSystem::new(
            w,
            FsOptions::<DefaultTimeProvider, LossyOemCpConverter>::new(),
        )
        .expect("Error initializing fatfs");

    log::info!(
        "fatfs init, type={}, cluster size={}",
        fat::FatType(f.fat_type()),
        f.cluster_size()
    );
    if let Some(label) = f
        .read_volume_label_from_root_dir()
        .expect("Error reading FS label")
    {
        log::info!("Label = {}", label);
    }

    let root_dir = f.root_dir();

    ls(&root_dir);

    {
        // Create a file and write some data to it.
        let mut r = root_dir.create_file("test").expect("error creating file");
        let data = [0xc0u8, 0xff, 0xee];
        r.write_all(&data).expect("write to file failed");
    }

    ls(&root_dir);

    log::info!("FS test done");
}

fn ls(dir: &Dir<'_, fat::Wrap<'_, Pflash>, NullTimeProvider, LossyOemCpConverter>) {
    log::info!("ls: {} entries", dir.iter().count());
    for r in dir.iter() {
        let e = r.expect("msg");
        log::info!("File size={:4} Name={:?}", e.len(), e.file_name());
    }
}
