use std::{fs::OpenOptions, io::{self, Read, Seek, SeekFrom}};

use ext2fsck::{bg_descriptor_table::ext2block_group_descriptor_table, checker::check, superblock::ext2superblock};



fn main() -> io::Result<()> {
    println!("from the fsck");

    let mut disk = OpenOptions::new().read(true).write(true).open("fakeFS.img")?;

    disk.seek(SeekFrom::Start(1024))?;
    let mut sb_buffer = [0u8; 1024];
    let _ = disk.read_exact(&mut sb_buffer);
    let superblock: ext2superblock = bincode::deserialize(&sb_buffer).unwrap();

    println!("block count is: {}, inode count is: {}", superblock.s_blocks_count, superblock.s_inodes_count);

    disk.seek(SeekFrom::Start(2048))?;
    let mut bg_buffer = [0u8; 1024];
    let _ = disk.read_exact(&mut bg_buffer);
    let block_group_descriptor_table: ext2block_group_descriptor_table = bincode::deserialize(&bg_buffer).unwrap();

    println!("used_dirs_count is {}, free inodes: {}, free blocks: {}", block_group_descriptor_table.bg_used_dirs_count, block_group_descriptor_table.bg_free_inodes_count, block_group_descriptor_table.bg_free_blocks_count);

    let _ = check(superblock, block_group_descriptor_table, &mut disk);    

    Ok(())

}

