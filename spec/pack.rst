====
Pack
====

Content Pack header
===================

This is the header of an pack. An pack can be store of a fs or part of a arx file.

============= ======= ====== ===========
Field Name    Type    Offset Description
============= ======= ====== ===========
magic         u32     0      The magic number to detect the type of the file
                             0x61727863 (arxc)
majorVersion  u8      4      The major version of the pack = 1
minorVersion  u8      5      The minor version of the pack = 0
id            [u8,16] 6      id of the pack
packSize      u64     22     Size of the pack
checkInfoPos  u64     48     The checksum position (allways at end of the pack)
_padding      u24     22     A padding, may be used in the futur
entryCount    u24     24     Number of entry in the pack (max of 2^24 entry per pack)
clusterCount  u32     28     Number of cluster in the pack (max of 2^20)
entryPtrPos   u64     32     A ``8pointer`` to a array of entryInfo offsets.
clusterPtrPos u64     40     A ``8pointer`` to a array of cluster offsets.
============= ======= ====== ===========

Full Size : 56 bytes

ClusterPtrPos array
===================

A array of ``8pointer``. Each entry is a offset to the start of a cluster.
Offsets may not be writen sequentially. Offsets are relative to the start of the pack.

EntryPtrPos array
=================

An array of EntryInfo

Cluster
=======

A cluster is a container of content. It contains plain data.
There is no information about the name or anything else about a file.

============= ========= ================= ===========
Field Name    Type      Offset            Description
============= ========= ================= ===========
type          u8        0                 | The hightest 4 bits are reserved.
                                            Must be equal to 0.
                                          | The lowest 4 bits are the cluster
                                            compression :

                                          - 0: nocompression
                                          - 1: lz4
                                          - 2: lzma
                                          - 3: zstd
clusterSize   ``8size`` 1                 The size of the (potentially compressed)
                                          cluster (including this header)
blobCount     u16       9                 The number of blob in the cluster
                                          (limited to 2^12==4096)
offsetSize    u8        11                | The size (in bytes) of the offsets.
                                          | Define uN (N == offsetSize)
dataSize      uN        12                The size of the uncompressed data
                                          (without the header and the offsets)
blob1 offset  uN        12+uN             Start of second (1) blob, end of the first
                                          blob (0)
blob2 offset  uN        12+uN*2           Start of third (2) blob, end of second blob
...           ...       ...               ...
blobN offset  uN        12+uN*(blobCount) Start of the last blob, end of the end of the
                                          second to last blob
data          u8*       12+uN*blobCount   The data, potentially compressed
============= ========= ================= ===========

blob1..blobN represent a array of dimension blobCount-1

| blob0 offset is always 0. Its size is blob1 (array[0]
| blobN (0 < N < blobCount) offset is array[N-1]. Its size is (array[N]-array[N-1])
| blobN (N==blobCount) offset is array[N-1]. It size is (dataSize-array[N-1])


Entry info
==========

While the cluster store the data itself, an entry info store metadata about this data.

============= ==== ====== ===========
Field Name    Type Offset Description
============= ==== ====== ===========
clusterNumber u32  0      | 20 highest bytes = clusterIndex (so 1 048 576 max cluster in
                            a pack)
                          | 12 lowest bytes = blobIndex (so 4096 max blob per cluster)
============= ==== ====== ===========
