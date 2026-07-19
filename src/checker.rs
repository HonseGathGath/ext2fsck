use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
};

use crate::{
    bg_descriptor_table::ext2block_group_descriptor_table,
    superblock::ext2superblock,
    utils::count_zeroes,
};

const BLOCK_SIZE: u64 = 1024;

fn block_to_offset(block_id: u32) -> u64 {
    (block_id as u64) * BLOCK_SIZE
}

fn items_in_group0(total_count: u32, per_group: u32) -> u32 {
    if total_count < per_group {
        total_count
    } else {
        per_group
    }
}

pub fn check(
    superblock: ext2superblock,
    mut bgdt: ext2block_group_descriptor_table,
    disk: &mut File,
) -> io::Result<()> {
    let blocks_in_group0 =
        items_in_group0(superblock.s_blocks_count, superblock.s_blocks_per_group);

    disk.seek(SeekFrom::Start(block_to_offset(bgdt.bg_block_bitmap)))?;
    let mut blk_bitmap = vec![0u8; BLOCK_SIZE as usize];
    disk.read_exact(&mut blk_bitmap)?;
    let checked_free_blocks = count_zeroes(&blk_bitmap, blocks_in_group0);

    let inodes_in_group0 =
        items_in_group0(superblock.s_inodes_count, superblock.s_inodes_per_group);

    disk.seek(SeekFrom::Start(block_to_offset(bgdt.bg_inode_bitmap)))?;
    let mut ino_bitmap = vec![0u8; BLOCK_SIZE as usize];
    disk.read_exact(&mut ino_bitmap)?;
    let checked_free_inodes = count_zeroes(&ino_bitmap, inodes_in_group0);

    println!(
        "checked free blocks: {}, checked free inodes: {}",
        checked_free_blocks, checked_free_inodes
    );

    let mut corrected = false;

    if checked_free_blocks != bgdt.bg_free_blocks_count as u32 {
        println!(
            "WARNING: block bitmap mismatch — scanned {} free, descriptor says {} free",
            checked_free_blocks, bgdt.bg_free_blocks_count
        );
        bgdt.bg_free_blocks_count = checked_free_blocks as u16;
        corrected = true;
    }

    if checked_free_inodes != bgdt.bg_free_inodes_count as u32 {
        println!(
            "WARNING: inode bitmap mismatch — scanned {} free, descriptor says {} free",
            checked_free_inodes, bgdt.bg_free_inodes_count
        );
        bgdt.bg_free_inodes_count = checked_free_inodes as u16;
        corrected = true;
    }

    if corrected {
        disk.seek(SeekFrom::Start(block_to_offset(2)))?;
        let bytes = bincode::serialize(&bgdt).unwrap();
        disk.write_all(&bytes)?;
        println!("  -> Corrected block group descriptor written to disk.");
    }

    Ok(())
}
