=========
Directory
=========

The packs store the content of the data. However, no metadata is stored in packs.

To discover content in a Jubako container, this content has to be referenced in the directory.

The directory is somehow the most complex part of the Jubako format as it allow different
kind of situation and usage :

- Most applications want to access entries using text key (a name/url/path).
- Some applications want to access entries using different keys.
- Some applications may want different kind of entries in the same Jubako container.
- Some applications may want to separate entries in different "namespaces".
- Some applications may want to use a tree/directory index.
- Some applications may want to use some complex specific index (fulltext index)

Jubako format provide a sets of indexes kind that can be used to index the content.
It is also possible to reference an application specific index (as a xapian index).

Jubako indexes tries to answer the following constraints :

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
valueStorePtrPos Offset  16     A Offset to a array of keyStore offsets.
indexCount       u32     24     Number of index in the directory
entryStoreCount  u32     28     Number of entryStore in the directory
valueStoreCount  u8      32     Numbre of keyStore in the directory (16 store max).
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

A store is composed of data and a tail.

Plain store
-----------

If the ``storeType`` is 0, the store is a plain store.
There is no (internal) index, and the store is only composed of the data and small tail.
The data is composed of Pstring, the entries key contains directly the offsets
of the Pstring in the data.

The plain store tail is :

============== ================== ====== ===========
Field Name     Type               Offset Description
============== ================== ====== ===========
storeType      u8                 0      The type of the store.
dataSize       u64                1      The size of the data store.


Indexed Store
-------------

If ``storeType`` is 1, the store is indexed.
The store type is composed of the data, and the tail.
The tail itself contains a index, storing the offset of the key data in data.

By definition, a indexed keystore is useful if ``nb_bytes(entryCount) < offsetSize``.


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


Entry Store
===========

Plain EntryStore
================

The first kind of index is a plain listing of entry.

Tail
----

============= ================== ================= =============
Field Name    Type               Offset            Description
============= ================== ================= =============
indexType     u8                 0                 0
entrySize     u16                1                 The size of one entry.
variantCount  u8                 3                 The number of variants in this index.
keyCount (N)  u8                 4                 The number of key info.
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
the key info.

Variant
-------

The structure of the entry can varying (union in C, or Enum in rust).
Each variant is a list of values. The actual types of the values depend of the variant itself.
Variants of a entry may shared a set of common values (at least a variant identifier).
Those common values are regrouped in the "common part".

So a entry is composed of :
- A common part : a list of values
- A variant identifier
- A specific part : a list of values (this list depends of the variant used)

``variantCount`` define how many variants is possible for the entries.
Most of the time it is equal to 0 (no variant, only a common part).

If there is several variant, the first keyInfos describes common properties of the entry.
After that, follows the variants definition.

Each variant definition MUST start with a variant identifier (0b1000) followed by specific keys.
At parsing the index header, variant identifier is what allow implementation where the variant definitions start and stop.
When parsing the entry, this key allow implementation to know which variant to use.

If there is only one variant, by definition all keys are "common" and it SHOULD be no variant (``variantCount`` is 0).
However, nothing prevent to create a entry with one variant and put only some part of the keys (potentially none) in the common part.

If a variant identifier is present, ``entrySize`` and ``keyCount`` MUST integrate it.

All variants MUST have the same size. (Use padding if needed)

KeyInfo
-------

The entry definition (common part and variant) is composed of key info.
A key info describes one "property" or "attribute" of the entry.
It is composed of:
- a key type: at least one byte and potentially several complement bytes
- a key name: the name of the property.

Key Type
--------

Each keyType is composed of (at least) one bytes:

- The highest 4 bits (0bTTTT) give the type of the key
- The lowest 4 bits (0bSSSS) give the size of the key (or more information depending of the type of the key)

It may be followed by a complement byte, depending of the key type.

If 0bTTTT is :

- 0b0000 : Padding
- 0b0001 : ContentAddress
- 0b0010 : Unsigned Integer
- 0b0011 : Signed Integer
- 0b0100 : ...
- 0b0101 : Char[]
- 0b0110 : ..
- 0b0111 : ..

- 0b1000 : VariantId
- 0b1001 : ..
- 0b1010 : Deported Unsigned Integer
- 0b1011 : Deported Signed Integer
- 0b1100 : ..
- 0b1101 : ..
- 0b1110 : ..
- 0b1111 : ..

Variant identifier
..................

``0bSSSS`` must be 0.
Key size is always 1.
The variant identifier in the entry is a u8.

Padding
.......

Padding are ignored. Implementation must not provide a way to access the data there.
However, the padding size is taken into account to deduce the offset of other keys.

Padding may be used to combine different index using the same data (as union or
specialized index).

The size of a padding is ``0bSSSS + 1``.

Content Address
...............

``contentAddress`` is used to point to a specific blob.

A ``contentAddress`` is composed of two parts :
- The first bytes (1 or 2) are the ``pack_id`` (The pack in which find the content)
- The last bytes (1, 2, 3 or 4. Equal to ``0b00SS+1``) are the ``content_id`` (The identifier of the content in the pack)

``0bDPCC`` describes the size of the pack_id:
- ``P + 1`` is the size of the pack_id.
- ``CC + 1`` is the size of the ``content_id``

If ``D`` is 1, the key info is followed by ``P+1`` bytes wich are the value of the pack_id.
The entry doesn't contain the pack_id and reader must use the default value as value for the property.


Unsigned and Signed Integer
...........................

Integer may be signed or not.
The keyInfo is ``0bDSSS``.
- ``SSS + 1`` is the size of the integer.
- ``D`` tell is a default value is provided.

If ``D`` is 1, the key info is followed by ``SSS + 1`` bytes which are the value of the integer.
The entry doesn't contain the integer and reader must use the default value as value for the property.

Implementation are free to provide api returning integer using standard size highest
than what is stored.
(They can all the time return a u64 or s64. Or they can return a u32 if a u24 is stored).

If the integer is deported ``0b1010``, two complement bytes follow:
- ``0b00000KKK + 1`` is the size of the key_id in the value store.
- ``0xXX`` the id of the value store.

The value is stored in a value store using a ``0b0SSS + `` size.

If ``D`` is 1, the key is followed by ``KKK + 1`` bytes which are the value of the key_id.


Char[]
......

Byte array can be stored (embedded) in the entry or deported in another store.
As entries in an index must always have the same size, an embedded array must always be the same size.
If the key needs variable array size, the array must be deported.

The keyInfo is ``0bDSSS``.
- ``SSS``  defined the size of the integer in the entry to indicate store the actual size of the char[].
- ``D`` tell is a default value is provided.

The actual data of the byte array can be stored in two way:
- Directly in the entry in a fixed_array. By definition, as entry must have a fixed size,
  this fixed_array is always the same size in all entries.
  If the data is smaller than the fixed_array, the fixed_array is filled with 0.
  If the data is bigger than the fixed_array, the left over part must be put in a variable_array.
- In a variable_array. The variable array is stored in a ValueStore.
  The entry store a pointer (a key/id) to the value in the ValueStore.

A complement byte (``0bKKKZZZZZ``) follows the key info to describe how the data is stored:
- ``KKK`` is the size of the key_id to the variable_array. If ``KKK`` == ``000``, no variable_array is used.
- ``ZZZZZ`` is the size of the fixed_array.

If we use a variable_array (``KKK`` != ``000``), another complement byte follow giving the index of the key store to use.

The data in the entry is composed:
- ``SSS`` bytes telling the size of the char[].
- ``ZZZZZ`` bytes being the first part of the data. May be padded with 0 if size of char[] < ``ZZZZZ``.
- ``KKK`` byte being the key_id of the variable array (if ``KKK`` != ``000``)

If ``D`` is 1, the key info is followed by ``SSS + ZZZZZ + KKK`` bytes which are the value of the char[] as
describe above.
The entry doesn't contain the char[] and reader must use the default value as value for the property.

Unsized char[]

``SSS`` can be zero ``000``. In this case the char[] is unsized. It make sens only in case of :
- ``KKK`` is zero. In this case, the size of the char[] is always ``ZZZZZ``.
- The used value store is a indexed value store. In this case, the size of the data is computed as
  ``ZZZZZ + value_store_offsets[index+1] - value_store_offsets[index]``.

It is not possible to store data smaller than ``ZZZZZ`` without a explicit size.

Key Name
--------

All key types (except padding) are followed by a key name.
The key name is a UTF-8 PString.

Key name associated to a variant id defines the name of the variant.
Other key names define the name of the property.



Ref EntryStore [TODO]
=====================

Overlay EntryStore [TODO]
=========================


Index
=====

Index is the last part of the directory.
It is a simple header describing the index and where to find the data.


Header
------

============= ================== ================= =============
Field Name    Type               Offset            Description
============= ================== ================= =============
storeId       u32                0                 The entry store where to find the entries.
entryCount    u32                4                 The number of entries in the index.
entryOffset   u32                8                 The offset of the first entry in the entry store.
freeData      [u8;4]             12                Some data specific type are free to use
indexKey      u8                 16                | The primary key of the index.
                                                   | 0 if no primary key.
                                                   | 1 for the first key.
                                                   | 2 for second ...
indexName     ``pstring``        17                The name of the index, may be used to
                                                   identify the index
============= ================== ================= =============


Full Size : 17 + size of pstring
