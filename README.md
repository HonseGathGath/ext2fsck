# rfsck-ext2

A **microscopic, educational** ext2 filesystem checker written in Rust.

**Scope**: exactly three audit phases, each building on the last.  Runs on small
local disk images (e.g. 4 MB with 1 KiB blocks — one block group, tiny inode
table, trivial layout).  Not a production `e2fsck` replacement.

---

## Phases

| # | Phase | Status |
|---|-------|--------|
| 1 | **Bitmap & Global Counter Audit** — read block/inode bitmaps, count free entries, compare against the block group descriptor table, and write back corrections. | ✅ Done |
| 2 | **Link Count Audit** — crawl the directory tree starting at root inode #2, count hard-link references per inode, compare against `i_links_count`. | 🔲 Planned |
| 3 | **Directory Sanity Checking** — verify every directory's first two entries are `.` and `..`. | 🔲 Planned |

See [`plan.md`](plan.md) for detailed implementation steps.

---

## Quick Start

### 1. Create a test image

```bash
# Create a 4 MB sparse file
dd if=/dev/zero of=fakeFS.img bs=1M count=4

# Format as ext2 with 1 KiB blocks (so offsets are easy to calculate)
mkfs.ext2 -F -b 1024 fakeFS.img
```

### 2. Populate with test data

```bash
mkdir /tmp/test
sudo mount -o loop fakeFS.img /tmp/test

sudo mkdir /tmp/test/docs
sudo mkdir /tmp/test/home
echo "this is from a block" | sudo tee /tmp/test/docs/notes.txt

sudo ln /tmp/test/docs/notes.txt /tmp/test/home/link.txt

sudo umount /tmp/test
```

### 3. Run the checker

```bash
cargo run
```

Example output on a clean image:

```
from the fsck
block count is: 4096, inode count is: 1024
used_dirs_count is 4, free inodes: 1010, free blocks: 3803
checked free blocks: 3803, checked free inodes: 1010
```

### 4. Corrupt the image (for testing)

```bash
debugfs -w fakeFS.img
```

Inside `debugfs`:

```
modify_inode docs/notes.txt   # change i_links_count
clri docs/notes.txt            # clear an inode
```

Then re-run `cargo run` to see mismatch warnings and automatic correction.

---

## Architecture

```
src/
├── main.rs                  # Entry point — opens disk, reads superblock + BGDT
├── lib.rs                   # Module declarations
├── superblock.rs            # ext2superblock struct (1024 bytes at offset 1024)
├── bg_descriptor_table.rs   # ext2block_group_descriptor_table struct (32 bytes)
├── checker.rs               # Phase 1 — bitmap audit, comparison, writeback
├── inode_table.rs           # ext2_inode struct (128 bytes) — needs field-width fix
└── utils.rs                 # count_zeroes() — count zero bits in a bitmap
```

**Dependencies**: `serde` + `bincode` for reading binary structs from disk.
No other crates.

**Block size**: hard-coded at 1024 bytes (set via `mkfs.ext2 -b 1024`).
This keeps all offset math trivial: `offset = block_number × 1024`.

---

## Layout Reference

| Byte offset | Content |
|-------------|---------|
| `0` | Boot block (unused, 1024 bytes) |
| `1024` | Superblock |
| `2048` | Block group descriptor table |
| `bg_block_bitmap × 1024` | Block bitmap (1 bit per block) |
| `bg_inode_bitmap × 1024` | Inode bitmap (1 bit per inode) |
| `bg_inode_table × 1024` | Inode table (128 bytes per inode) |

---

## License

Educational use.  No warranty, no production use.


