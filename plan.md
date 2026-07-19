# rfsck-ext2 — Implementation Roadmap

## Current State (Phase 1 — Complete)

- Reads superblock (offset 1024) and block group descriptor table (offset 2048).
- Scans block group 0's **block bitmap** and **inode bitmap**, counting zero bits.
- Computes the correct number of entries in BG 0 (`min(total, per_group)`) — fixes old
  "last-group-remainder" bug that would miscount when multiple block groups exist.
- Compares scanned free counts against `bg_free_blocks_count` / `bg_free_inodes_count`.
- If mismatched: prints a warning **and writes the corrected BGDT back to disk**.

---

## Blocking Bug (Fix Before Phase 2)

### `inode_table.rs` — `i_links_count` has wrong width

```rust
// CURRENT (wrong)             // CORRECT
i_links_count: u32,           i_links_count: u16,
```

The ext2 on-disk inode (128-byte variant) defines `i_links_count` as `__u16` at
offset 26.  The current `u32` consumes bytes 26–29, which steals the lower half
of `i_blocks` (offset 28–29).  **Every field after offset 26 is shifted by 2
bytes**, making the entire struct deserialize garbage.

**Fix** (one-line change, safe — struct is currently unused):
```
i_links_count: u32  →  i_links_count: u16
```

Also rename `ext2inode_table` → `ext2_inode` (it describes one inode, not a
table of inodes).

---

## Phase 2 — Link Count Audit

### Goal
Walk every directory on the filesystem, count how many directory entries
reference each inode, then compare that count against the inode's
`i_links_count`.  Report (and optionally fix) mismatches.

### Step 2.1 — Read a single inode from disk

**New function** in `inode_table.rs` (or a new `dir.rs`):

```
fn read_inode(disk: &mut File, bgdt: &ext2block_group_descriptor_table,
              inode_nr: u32) -> io::Result<ext2_inode>
```

- Inode table base byte-offset = `bgdt.bg_inode_table * 1024`.
- Inode byte-offset = base + `(inode_nr - 1) * 128`.
- Seek, read 128 bytes, `bincode::deserialize`.

### Step 2.2 — Directory entry type + parser

**New struct** (in `dir.rs` or `inode_table.rs`):

```rust
#[repr(C)]
struct ext2_dir_entry {
    inode: u32,       // 0 means unused
    rec_len: u16,     // total byte length of this entry (incl padding)
    name_len: u8,     // byte length of the name that follows
    file_type: u8,    // 1=reg, 2=dir
    // name[0..name_len] follows immediately
}
```

**Parser** — iterate over raw directory block bytes:

```
offset = 0
while offset < block_size:
    entry = deserialize at offset
    if entry.inode == 0: skip (deleted entry)
    name = bytes[offset+8 .. offset+8+entry.name_len]
    yield (entry.inode, name, entry.file_type)
    offset += entry.rec_len
```

### Step 2.3 — Crawl the directory tree (BFS)

```
let mut ref_count: std::collections::HashMap<u32, u32> = HashMap::new();
let mut queue: Vec<u32> = vec![2];      // root inode

while let Some(dir_ino) = queue.pop() {
    let inode = read_inode(disk, &bgdt, dir_ino);
    verify it is a directory  (i_mode & 0o40000 != 0) — skip if not.

    for each data block of the directory:
        parse entries
        for each entry:
            if name == ".":  continue (self-reference)
            if name == "..": continue (parent — counter incremented at parent)
            *ref_count.entry(entry.inode).or_default() += 1;
            if entry is a directory (file_type == 2):
                queue.push(entry.inode);
}
```

**Data block access**: start with the 12 direct block pointers
(`inode.i_block[0..12]`).  For the tiny test images (4 MB, shallow trees)
indirect blocks will not be needed.  If present, log a skip message.

### Step 2.4 — Compare and report

```
for (ino, counted) in &ref_count {
    let inode = read_inode(disk, &bgdt, *ino);
    let on_disk_count = inode.i_links_count;
    if *counted != on_disk_count as u32 {
        println!("LINK COUNT MISMATCH: inode {} has {} links on disk, but directory scan found {} references", ino, on_disk_count, counted);
    }
}
```

Also check for unreachable inodes that still have `i_links_count > 0`.

### Step 2.5 — Optional correction

Seek to the inode's location, overwrite `i_links_count` bytes (offset 26,
2 bytes) with the corrected value, then `write_all`.

---

## Phase 3 — Directory Sanity Checking

### Goal
Verify that every directory's first two entries are strictly `.` → `self` and
`..` → `parent`.

### Step 3.1 — Piggyback on Phase 2 crawl

During the Phase 2 BFS crawl, when we parse a directory's data blocks, check:

```
let entries: Vec = parse_dir_blocks(...);
if entries.len() < 2 {
    println!("DIRECTORY CORRUPT: inode {} has fewer than 2 entries", dir_ino);
    continue;
}

// Entry 0 must be "."
if entries[0].inode != dir_ino  ||  name_bytes[0..1] != b"." {
    println!("DIRECTORY CORRUPT: inode {} first entry is not '.'", dir_ino);
}

// Entry 1 must be ".."
if name_bytes[0..2] != b".." {
    println!("DIRECTORY CORRUPT: inode {} second entry is not '..'", dir_ino);
}
```

### Step 3.2 — Standalone mode (optional)

If not running Phase 2, Phase 3 can run independently by scanning the inode
table for directory inodes (`i_mode & 0o40000`) and checking each one.

---

## Future Features (beyond the three phases)

### FIXED — Free-block / free-inode superblock sync
After correcting BGDT free counts, sync the superblock fields
(`s_free_blocks_count`, `s_free_inodes_count`) by summing all block group
descriptors.  This requires iterating all BGs.

### Multi-block-group support
Loop over every block group (not just BG 0) and audit each group's bitmaps and
descriptors.  The `items_in_group` helper already accepts parameters for this;
only a loop and per-group offset calculation is needed.

### Indirect-block traversal
For Phase 2, handle singly / doubly / triply indirect blocks so the checker
works on images with deep directories or large files.

### Orphaned-inode report
Detect inodes that are marked allocated in the bitmap but have `i_links_count
== 0` and `i_dtime == 0` (orphans that were never cleaned up).

### Bad-block detection
Verify that every block pointer in every inode falls within the valid range
`[0, s_blocks_count)`.  Report out-of-range pointers.

### Lost-and-found recovery
Collect orphaned inodes and directory entries pointing to unallocated inodes
into a `lost+found` directory (like `e2fsck` does).

### Zero-length directory entry detection
Scan for entries where `rec_len == 0` (infinite loop risk) or `name_len == 0`
with `inode != 0`.

### Cross-link detection
Detect hard-link loops and directories that appear as their own parent or
grandparent through corrupted `..` entries.

---

## ext2 Layout Reference (1 KiB blocks)

| Offset | Content |
|--------|---------|
| 0–1023 | Boot block (unused) |
| 1024–2047 | Superblock (`ext2superblock`, 1024 bytes) |
| 2048–3071 | BGDT (`ext2block_group_descriptor_table`, 32 bytes per group) |
| varies | Block bitmap (1 block = 8192 bits = 1024 bytes) |
| varies | Inode bitmap (1 block) |
| varies | Inode table (128 bytes per inode × inodes_per_group) |
| varies | Data blocks |

### Inode field offsets (128-byte variant)

| Offset | Size | Field |
|--------|------|-------|
| 0 | 2 | `i_mode` (0x4000 = directory) |
| 2 | 2 | `i_uid` |
| 4 | 4 | `i_size` |
| 8 | 4 | `i_atime` |
| 12 | 4 | `i_ctime` |
| 16 | 4 | `i_mtime` |
| 20 | 4 | `i_dtime` |
| 24 | 2 | `i_gid` |
| **26** | **2** | **`i_links_count`** (u16!) |
| 28 | 4 | `i_blocks` (512-byte sectors) |
| 32 | 4 | `i_flags` |
| 36 | 4 | `i_osd1` |
| 40 | 60 | `i_block[15]` (block pointers) |
| 100 | 4 | `i_generation` |
| 104 | 4 | `i_file_acl` |
| 108 | 4 | `i_dir_acl` |
| 112 | 4 | `i_faddr` |
| 116 | 12 | `i_osd2` |
