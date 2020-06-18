=========
Directory
=========

All the content (and metadata) are stored in extension. It is enough to a implementation to find any content, using the entry index.

However, we may need more ways to lookup, regroup, filter content.

Most applications want to access entries using text key (a name/url/path).
Some appliactions want to access entries using differents keys.
(As kiwix using name for metadata, url or name for articles and url for other resources/image/css/js)

Some applications may want to separate entries in different "namespaces".
Some applications may want to use a tree/directory index.
Some applications may want to use some complex specific index (fulltext index)

Entry indexes are here for that :

- An index can contain all the entries or not.
- An index can be associated to a key. In this case the index must be sorted by the key. To allow a binary search. Key value must be unique.
- An index can be only a list of entry. The index is sorted in the order needed.
- An index can contain metadata associated to the entry.
- Two indexes can index different contents. In this case, they somehow implement a different namespace.
- A content can be indexed by different index.

Index Pack header
=================

This is the header of an pack. An pack can be store of a fs or part of a arx file.

============ ======= ====== ===========
Field Name   Type    Offset Description
============ ======= ====== ===========
magic        u32     0      The magic number to detect the type of the file
                            0x61727869 (arxi)
appVendorId  u32     4      An id to identify the usage of the arx file (its usage)
majorVersion u8      8      The major version of the pack = 1
minorVersion u8      9      The minor version of the pack = 0
id           [u8,16] 18     id of the pack
fileSize     u64     10     The size of the pack (on file system)
checkInfoPos u64     48     The checksum position (allways at end of the pack)
_padding     u16     34     A padding, may be used in the futur
indexCount   u32     36     Number of index in the pack
indexPtrPos  u64     40     A ``8pointer`` to a array of entryInfo offsets.
freeData     [u8,72] 56     Free data, application specific to extend the header
============ ======= ====== ===========

Full Size : 128

Index Header
============

Each index is specific to a usage and their internal structure can be pretty different.

Each of them share the same first byte :


=========== ==== ====== ===========
Field Name  Type Offset Description
=========== ==== ====== ===========
indexType   u8   0      | The type of the index.
                        | Highest bit is 0 for arx index, 1 for application specific
headerSize  u16  1      The size of the header
=========== ==== ====== ===========



Plain list index
================

Header
------

The first kind of index know by arx implementation is a listing of entry, along with
some metadata

============= ================== ================= =============
Field Name    Type               Offset            Description
============= ================== ================= =============
indexType     u8                 0                 0
headerSize    u16                1                 The size of the header
indexKey      u8                 1                 | The primary key of the index.
                                                   | 0 if no primary key.
                                                   | 1 for the first key.
                                                   | 2 for second ...
entrySize     u8                 2                 The size of one entry.
padding       u8                 3                 Reserved.
indexLength   u32                4                 The number of entry in the index.
keysDataPos   u64                8                 The offest (relative to the header's
                                                   start) of the keydata.
keysDataLen   u64                16                The length of the keydata.
indexArrayPos u64                24                The offset (relative to the header's
                                                   start) of the entryIndexArray.
extraData     ``contentAddress`` 32                An app specific content. Used as free
                                                   form data.
indexName     ``pstring``        36                The name of the index, may be used to
                                                   indentify the index
keyCount      u8                 37+len(indexName) The number of keys (metadata
                                                   associated to each entry)
keyType0      u8                                   The type of the key0
keyType1      u8                                   The type of the key1
...           u8                                   ...
keyTypeN      u8                                   The type of the keyN
============= ================== ================= =============


Full Size : indexArrayOffset + 4*indexLength

The index itself is a array of ``indexLength`` entries, each entry having a size of
``entrySize``.
The array start at ``indexArrayPos``.

Each entry is a list of ``keyCount`` values. The values are described in each ``keyTypeN``.
``entrySize`` is the sum of the size of all the key in the index.

The kind of the key give its size:

If it is 0, the is ``contentAddress``. The size is 4.

Else:
The 4 lowest bits specify the size o    f the key (up to 15 bytes) (named 'SIZE' after)
The 4 highest bits specify how to interpret the key
0b0001 for unsigned int (0x0)
0b0010 for signed int (0x1)
0b0011 for char[] (0x3)
0b0100 for pstring + deported size    + without fast lookup (0x4)
0b0101 for pstring + local size + without fast lookup (0x5)
0b0110 for pstring + deported size    + with fast lookup (0x6)
0b0111 for pstring + local size + with fast lookup. (0x7)

For integer (signed or unsigned) the size must not exceed 8 bytes.
Implementation are free to provide api returning integer using standard size highest
than what is stored.
(They can all the time return a u64 or s64.
Or they can return a u32 if a u24 is stored).

| ``char[]``: is simply an array of char.
| ``pstring`` : this is a pstring but the content is stored in keysData.

It comes with several variants :

Deported size
  This is a classical pstring. The value of the key is simply the offset (in keysData)
  where to find the pstring.
Local size
  The size of the pstring is stored in the key, not within the pstring.
  So the key is composed of 1B for the size and (SIZE-1)B for the offset
Fast lookup
  The first byte of the pstring is stored in the key.
  So the key is composed of 1B for the first char (SIZE-1)B for the offset.
  The offset points to a pstring not containing the first char.
  The size of the final string is the size of the pstring+1
Local size and fast lookup
  The key is compose of 1B for the size, 1B for the first char and (SIZE-2)B for the
  offset. The offset points directly to the data (not containing the first char).
  The size of the final string is the size stored in the key + 1.


``indexKey`` : The index can be sorted to allow binary search. The comparaison function is unspecified and left to the vendor.

Key Data
--------

The key data is composed of a byte (as a header) and the whole data.
The size in the index's header include this byte ::


    +---+============+
    | 0 | plain data |
    +---+============+


    +---+---+---+---+---+---+---+---+---+============+
    | 1 | Uncompressed size (8bytes)    | plain data |
    +---+---+---+---+---+---+---+---+---+============+

Indirect list index
===================

This kind of index is usefull to create index and reuse metadata declared in another index(es).
It can be used to sort entries in a different order, or merge several indexes or ...

Header
------

The first kind of index know by arx implementation is a listing of entry, along with some metadata

============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
indexType      u8                 0      1 or 2
headerSize     u16                1      The size of the header
baseIndex      u8                 1      The number of the base index.
                                         (0 if indexType is 2)
indexKey       u8                 2      | The primary key of the index.
                                         | (using keys declared in base index)
padding        u8                 3      Reserved.
indexLength    u32                4      | The number of entry in the index.
                                         | Must be <= to the number of entry in the base
                                           index
indexArrayPos  u64                24     The offset (relative to the header's start) of
                                         the entryIndexArray.
extraData      ``contentAddress`` 32     An app specific content. Used as free form data
indexName      ``pstring``        36     The name of the index, may be used to indentify
                                         the index
============== ================== ====== ===========


If indexType is 1, the indexArray is a array of u32. Each u32 is the index of the entry in the base index.

If indexType is 2, the indexArray is a array of u40. Each u40 is composed of ::

    +-----------+------+------+------+------+
    | baseIndex | Entry number in baseIndex |
    +-----------+------+------+------+------+

If indexType is 2 and indexKey != 0, the different base indexes must be coherent (The indexKey keys of all index must be comparable)


Overlay index
=============

[TODO]
