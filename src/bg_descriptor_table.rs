use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Serialize, Deserialize)]

pub struct ext2block_group_descriptor_table {
    pub bg_block_bitmap: u32,
    pub bg_inode_bitmap: u32,
    bg_inode_table: u32,
    pub bg_free_blocks_count: u16,
    pub bg_free_inodes_count: u16,
    pub bg_used_dirs_count: u16,
    bg_pad: u16,
    bg_reserved: [u8; 12]
}
