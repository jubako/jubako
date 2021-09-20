=========
Directory
=========

The packs store the content of the data. However, no metadata is stored in packs.

To discover content in a Arx file, this content has to be referenced in the directory.

The directory is somehow the most complex part of the Arx format as it allow different
kind of situation and usage :

- Most applications want to access entries using text key (a name/url/path).
- Some applications want to access entries using differents keys.
- Some applications may want different kind of entries in the same Arx file.
- Some applications may want to separate entries in different "namespaces".
- Some applications may want to use a tree/directory index.
- Some applications may want to use some complex specific index (fulltext index)

Arx format provide a sets of indexes kind that can be used to index the content.
It is also possible to reference an application specific index (as a xapian index).

Arx indexes tries to answer the following constraints :

- An index can contain all the entries or not.
- An index can be associated to a key.
  In this case the index must be sorted by the key to allow a binary search.
  Key value must be unique.
- An index can be only a list of entry. The index is sorted in the order needed.
- An index can contain metadata associated to the entry
- Two indexes can index different contents. In this case, they somehow implement a
  different namespace.
- A same content can be indexed by different index.

A directory is composed of different parts:

- A header listing the indexes, the entry stores and the key stores.
- Entry stores, an array of entries, the `real index content`.
  The entry store contains a description (structure) of the entries stored.
  As each entry in a store must have the same size, variable size keys are stored in
  a key stores and referenced in the entries.
- Key stores, to store the value of variable size keys.
- Indexes, composed of a description of the index, and what entry store to use.


Directory header
================

This is the header of the directory.
It lists all other data in the directory.

================ ======= ====== ===========
Field Name       Type    Offset Description
================ ======= ====== ===========
indexPtrPos      Offset  0      A Offset to a array of index offsets.
entryStorePtrPos Offset  8      A Offset to a array of entryStore offsets.
keyStorePtrPos   Offset  16     A Offset to a array of keyStore offsets.
indexCount       u32     24     Number of index in the directory
entryStoreCount  u32     28     Number of entryStore in the directory
keyStoreCount    u8      32     Numbre of keyStore in the directory (16 store max).
freeData         [u8,31] 33     Free data, application specific to extend the header
================ ======= ====== ===========

Full Size : 64


Entry Store
===========

Entry store contains the main data of an index.
It is an array of fixed size entries.
It describes the size of the entries and how to interpret them.
It may contain entries for more than one index.

Key Store
=========

Deported bytes array are stored in key stores.

A store is composed of data and a tailer.

Plain store
-----------

If the ``storeType`` is 0, the store is a plain store.
There is no (internal) index, and the store is only composed of the data and small tailer.
The data is composed of Pstring, the entries key contains directly the offsets
of the pstring in the data.

The plain store tailer is :

============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
storeType      u8                 0      The type of the store.
dataSize       u64                1      The size of the data store.


Indexed Store
-------------

If ``storeType`` is 1, the store is indexed.
The store type is composed of the data, and the tailer.
The tailer itself contains a index, storing the offset of the key data in data.

By definition, a indexed keystore is usefull if ``nb_bytes(entryCount) < offsetSize``.


============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
storeType      u8                 0      The type of the store.
entryCount     u32                1      The number of entry in the store.
offsetSize     u8                 5      The number of bytes to represent the offsets
dataSize       uN                 6      The size of the data store.
                                         This size define the size of the offset in the
                                         index.
offset1                                  The offset of the second entry
                                         (and the size of first entry)
...
offsetN
============== ================== ====== ===========

Indexed Store with size [TODO]
------------------------------

If ``storeType`` is 2, the store is indexed.
The store type is followed by an index, the dataSize and the data itself.

============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
storeType      u8                 0      The type of the store.
entryCount                               The number of entry in the store.
                                         This number define the size of the key in the
                                         entry.
dataSize       u64                6      The size of the data store.
                                         This size define the size of the offset
                                         and size in the index.
offset0                                  Offset of the first entry
size0                                    Size of the first entry
offset1
size1
...
offsetN
sizeN
data                                     The data
============== ================== ====== ===========


Enty Store
==========

Plain EntryStore
================

The first kind of index is a plain listing of entry.

Tailer
------

============= ================== ================= =============
Field Name    Type               Offset            Description
============= ================== ================= =============
indexType     u8                 0                 0
entrySize     u16                1                 The size of one entry.
variantCount  u8                 3                 The number of variants in this index.
keyCount      u8                 4                 The number of key info.
                                                   (May differs from the number of key
                                                    as key may be composed of several key infos)
keyInfo0                                           The type of the key0
keyInfo1                                           The type of the key1
...                                                ...
keyInfoN                                           The type of the keyN
dataSize      Size
============= ================== ================= =============


Full Size : 13 + N*keyInfosize(most of the time 1 byte per keyInfo)

The index itself is a array of entries, each entry having a size of
``entrySize``.
The number of entries is ``dataSize``/``entrySize``.

Each entry is a list of values. The number of values is to be defined after decoding
the key infos.

Variant
-------

The structure of the entry can varying (union in C, or Enum in rust).
When a directory contains several variants, the interpretation of the (binary) entry can vary depending of the entry itself.

``variantCount`` define how many variants is possible for the entries.
Most of the time it is equal to 1 (only one way to parse entries).

If there is several variant, the first N keyInfos describing ``entrySize`` of data correspond to the first variant, the M following keyInfos, describing other ``entrySize`` of data correspond to the second variant, ...

The first key of each variant MUST be a variant identifier (0b1000).
At parsing the index header, it is what allow implementation where the variant definitions start and stop.
When parsing the entry, this key allow implementation to know which variant to use.

If there is only one variant, the ``variantCount`` is 1 and the variantId key SHOULD be omited.
If a variant identifier is present, ``entrySize`` and ``keyCount`` MUST integrate it.

All variants MUST have the same time. (Use padding if not)

Key Type
--------

Each keyType is composed of (at least) one bytes:

- The highest 4 bits (0bTTTT) give the type of the key
- The lowest 4 bits (0bSSSS) give the size of the key (or more information depending of the type of the key)

It may be followed by a complement byte, depending of the key type.

If 0bTTTT is :

- 0b1000 : Variant identifier
- 0b0000 : Padding. The value is ignored but the size is taken into account.
- 0b0001 : Content address. The size is always 4.
- 0b0010 : Integer. Signed or not depends of the value of 0bSSSS
- 0b0100 : char[]
- 0b0110 : PString
- 0b0111 : PString + fast lookup

Variant identifier
..................

``0bSSSS`` must be 0.
Key size is always 1.

Padding
.......

Padding are ignored. Implementation must not provide a way to access the data there.
However, the padding size is taken into account to deduce the offset of other keys.

Padding may be used to combine different index using the same data (as union or
specialized index).

If a entry ends wit a padding, the last padding key (of the last variant) is not necessary.
However, writer implementantion SHOULD include it.

The size of a padding is ``0bSSSS + 1``.

Content Address
...............

``contentAddress`` is used to point to a specific blob.

The content adress can be "patched". The bits `0bSSSS` are used to identify the number of patches.

If ``0bSSSS`` is 0, it "Classic" content address. No patch. The size of the key is 4.
Else we have a "chained" content patch.
The first contentAddress points to a patch to be applied to the second contentAddress.
The second contentAddress may also be a patch (if ``0bSSSS`` >=2) which applies to the third contentAddress.

The size of the key is always ``(0bSSSS + 1) * 4``

Integer
.......

Integer may be signed or not.
The highest bit of 0bSSSS is 1 if signed (0b1SSS) or 0 if not (0b0SSS).
Integer size must be between 1 and 8 bytes.
The size of the integer is ``0b0SSS + 1``.
Implementation are free to provide api returning integer using standard size highest
than what is stored.
(They can all the time return a u64 or s64. Or they can return a u32 if a u24 is stored).


Byte array and PString
......................

Byte array can be stored (embeded) in the entry or deported in another store.
As entries in an index must always have the same size, an embeded array must always be the same size.

If the key needs variable array size, the array must be deported.

Embeded bytes use a ``char[]`` (0b0100).
``0b0SSS + 1``  defined the size of the char (0 size array are impossible).
If the key data starts with a 10 (``0b10SS``), the key info is followed by a complement
byte (``0bssssssss``). The size of the array is ``0b00SS<<8 + 0bssssssss + 9`` (maximum size is 1024 bytes)
The third lower bit of ``0bSXSS`` MUST be 0 and is reserved for futur use.

Deported bytes use a ``PString`` (``0b0110``)
``0bSSSS + 1`` define the size of the key.
The key info is followed by a complement byte giving the index of the key store to use.
The header of the extra store will define the nature of the key (offset or index).
The size of the key doesn't have to be the same size of the index in the key store.
A key store may store a lot of keys (and so have big index) and the index may only use the first ones and so have smaller key.

"Deported bytes" keys may also include a fast lookup (``0b0111``).
The key info must be parsed the same way than for ``0b0110``.
The next keyInfo MUST be a ``char[]`` key info.

The following ``char[]`` is part of the PString.
This char is the beginning of the PString.
The full array is composed of ``concat(<embedded char[]> + <data stored in the keystore>)``
The bytes array stored in the deported store does NOT contains the first byte.

Ref EntryStore [TODO]
=====================

This kind of index is usefull to create index and reuse metadata declared in another index(es).
It can be used to sort entries in a different order, or merge several indexes or ...

Header
------

============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
indexType      u8                 0      1 or 2
headerSize     u16                1      The size of the header
padding        u8                 3      Reserved.
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


Overlay EntryStore [TODO]
=========================

This kind of index allow to reuse a already existing index, adding new keys to the entries in the index.


Header
------

The first kind of index know by arx implementation is a listing of entry, along with some metadata

============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
indexType      u8                 0      3
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

Index
=====

Index is the last part of the directory.
It is a simple header describing the index and where to find the data.


Header
------

The first kind of index know by arx implementation is a listing of entry, along with
some metadata

============= ================== ================= =============
Field Name    Type               Offset            Description
============= ================== ================= =============
storeId       u32                0                 The entry store where to find the entries.
entryCount    u32                4                 The number of entries in the index.
entryOffset   u32                8                 The offset of the first entry in the entry store.
extraData     ``contentAddress`` 12                Some content for the index. Used a extra data.
indexKey      u8                 16                | The primary key of the index.
                                                   | 0 if no primary key.
                                                   | 1 for the first key.
                                                   | 2 for second ...
indexName     ``pstring``        17                The name of the index, may be used to
                                                   indentify the index
============= ================== ================= =============


Full Size : 17 + size of pstring
