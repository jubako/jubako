====
Pack
====

Content Pack header
===================

============= ======= ====== ===========
Field Name    Type    Offset Description
============= ======= ====== ===========
entryPtrPos   Offset  0      A Offset to a array of entryInfo.
clusterPtrPos Offset  8      A Offset to a array of cluster SizedOffset.
entryCount    u32     16     Number of entry in the pack (max of 2^32 entries per pack)
clusterCount  u32     20     Number of cluster in the pack (max of 2^20)
freeData      [u8,40] 24
============= ======= ====== ===========

Full Size : 64 bytes

ClusterPtrPos array
===================

A array of SizedOffset. Each entry is a offset to the a cluster **tail**.
Offsets may not be writen sequentially. Offsets are relative to the start of the pack.

EntryPtrPos array
=================

An array of EntryInfo

Cluster
=======

A cluster is a container of content. It contains plain data.
There is no information about the name or anything else about a file.

A cluster consisted of the input (potentially compressed) data **followed** by a tail.

============= ========= =================== ===========
Field Name    Type      Offset              Description
============= ========= =================== ===========
type          u8        0                   | The highest 4 bits are reserved.
                                              Must be equal to 0.
                                            | The lowest 4 bits are the cluster
                                              compression :

                                            - 0: nocompression
                                            - 1: lz4
                                            - 2: lzma
                                            - 3: zstd
blobCount     u12       1                   The number of blob in the cluster
                                            (limited to 2^12==4096)
_paddingbit   u1                            Reserved
offsetSize    u3                            | The size (in bytes) of the offsets.
                                            | Define uN (N == 8*(offsetSize+1))
RawDataSize   SizeN     3                   The size of the raw (input) (potentially compressed) data.
DataSize      SizeN     3+uN                The size of the data (uncompressed compressed)
                                            cluster (including this header)
blob1 offset  uN        3+uN*2              Start of second (1) blob, end of the first
                                            blob (0)
blob2 offset  uN        3+uN*3              Start of third (2) blob, end of second blob
...           ...       ...                 ...
blobN offset  uN        3+uN*(blobCount+1)  Start of the last blob, end of the end of the
                                            second to last blob
============= ========= =================== ===========

blob1..blobN represent a array of dimension blobCount-1

| blob0 offset is always 0. Its size is blob1 (array[0])
| blobN (0 < N < blobCount) offset is array[N-1]. Its size is (array[N]-array[N-1])
| blobN (N==blobCount) offset is array[N-1]. It size is (dataSize-array[N-1])

The localization of the cluster data is `offset of the tail - RawDataSize`

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
