
let's start
first twe have to create a fake file system,
we can create a fiel that will act as a raw hard drive, we can use either truncate or dd
```
dd if=/dev/zero of=fakeFS.img bs=1M count=4
```

then we use mkfs to format the disk image 
```
mkf.ext2 -F -b 1024 fakeFS.img
```

-b 1024 option forces the block size to be 1KB, so standard offsets and sizes

and we also have to populate it, so we mount it and we create some fake data

```
mkdir /tmp/test
sudo mount -o loop fakeFS.img /tmp/test

sudo mkdir /tmp/test/docs
sudo mkdir /tmp/test/home
echo "this is from a block" | sudo tee /tmp/test/docs/notes.txt

sudo ln /tmp/test/docs/notes.txt /tmp/test/home/link.txt

sudo umount /tmp/test #needed so we can properly see updates in the filesystem, to avoid old cache data
```

and in order to know if it works, we need to actually corrupt it so that we can fix it and for that w e can use debugfs

```
debugfs -w fakeFS.img
```
to corrupt a link count:

```debugfs: modify_inode docs/notes.txt```

or to corrupt a block count or status:

```debugfs: clri docs/notes.txt```

and thus we have a corrupt ext2 filesystem we can use to test the program,


