# kym format

## Why a new format ?

Working on libzim I discover few "mistakes" that not ease the reading or
creation process.

- Dirent has no size information. The size of a dirent depends of the size of
the url and the title, and there is no size information in the header.
So you have to parse a the dirent to know its size. You cannot read directly
the title because you don't where it is (you have to search for '\\0', to know
the end of the url)

- Cluster has no size information. You cannot now directly the size of a
cluster. For an uncompressed cluster you can find the size quite easily has the
header is not compressed. But for compressed cluster, you have to uncompress the
data (and you don't know the size of the compressed data, nor the uncompressed
one) to be able to read the data.

- At creation, the size of the "Header"'s datas is not known before you know all
the content in the zim file. So you cannot start to write the content directly
in the zim file. You have to write things in temporary file and keep data
structure in memory. And so you cannot create big zim file on computer with
small ram.

We also want to do a series of improvement in the zim format :

- No more namespace. The separation between the article namespace (A) and
Image (I) is totally useless. The (B) namespace is not used at all.
Only the metadata (M) namespace is really use.
The (X) namespace for index is only used by only one article (xapian database).
It could be merge somewhere else, in the M namespace or directly in the header.
See https://github.com/openzim/libzim/issues/15

- We want content signing. See https://github.com/openzim/libzim/issues/40

- Category handling. See https://github.com/openzim/libzim/issues/75

- We want to be able to split zim files efficiently.

- We want to have zim extensions. Having a small "base" zim file we may want to
have extension to new content. Image is the base zim file is without image.
Or new articles if the base zim is a selection of articles.

- We may want to have different kind of extensions. Low and high resolution
image.

- We want to handle zim update. New version of a zim file could come as an
update to a previous zim. This way, we avoid to the user to download all the
content again.

- Zim update should be easily doable. When displaying a wikimedia content, a
client application may allow the user to change the content of an article
(as wikipedia does), and store the change as a zim update.

## Why the name "kym" ?

Despite :

- This work is greatly based on the libzim and zim format

- I work for the kiwix fondation who greatly use the zim format

- I somehow the only maintainer of libzim (thanks to kiwix fondation)

This work is made independently from kiwix or openzim organization.
For now this is more an essay than a real project to implement this.
It may change in the futur but for now there is absolutly no plan nor promise
that I (or other) will implement this format.

So I wanted to clearly make the distinction between the zim format and this new
format, so a different name (and not something like "zim2"). As this is publish
under the organization "kymeria", the name is "kym".

## Main ideas

The idea is to have a different kind of subcontent. Those subcontent could be
stored as independent files or all in one file (concat). The full content,
usefull to the user is the combination of different subcontents.

## Structures

### Integer

All integers are bigendian. This allow comparaison of integer to be made the
same way as comparaison of bytes array.

### Strings

- C format. This is the classical array of char ending with a `\0`. This allow
string to be as long as needed but need parsing of all the array (find the `\0`)
to know size of the string. This is noted as `cstring` format.

- Pascal format. This is an array where the first char is the size of the
string. There is no `\0` at the end. The size of the array is the same than a
`cstring` (n + 1). The string is limite to 255 chars, but a implementation can
know the size of string (and how many memory to reserve) by simply reading the
first char. This is noted as `pstring` format.

An empty string is the same in `cstring` or `pstring`  : a `\0` .

### Offset / Size

Most of the time, offset and size need to be more than 4Gb. So we need to use
more than 32 bits.

- We can store them on 64 bits. This allow a direct mapping to C type pointer.
However it propably uses too many bits

- We can store them on 40 bits (5 bytes). This allow use to store offsets up to
1To. Most of the time it is enough.

Depending of how the offset will be used, we may use a `8pointer` or a `5pointer`.

`8pointer` will be used by default as it is the "native" type.

`5pointer` will be used in array where they will be a lot of pointer "repetition".

Offset stored on 32 bits will be named `4pointer`

### Array vs List

Array in the rest of this document is an array of element of the same size.

An Nth element can be directly acceded at offset : `ArrayOffset + N * size(element)`

A list is a series of elements than can be size differently. So it is "almost"
impossible to have a direct access to a Nth element. A implementation will have
to read all previous elements (to know their size) before being able to access
to the Nth element.

A list may be doubled with a offset array. With this "double" structure,
a Nth element can be acceded at offset :

`ListOffset + *(ArrayOffset + N * size(element))` 


## KymHeader

This is the main header.

| Field Name    | Type     | Offset | Description                              |                  
| ------------- | -------- | ------ | ---------------------------------------- |
| magic         | u32      | 0      | The magic number to detect the type of the file 0x6B796D66 (kymf) |
| appVendorId   | u32      | 4      | An id to identify the usage of the kym file (its usage) |
| majorVersion  | u8       | 8      | The major version of the kym file = 1    |
| minorVersion  | u8       | 9      | The minor version of the kym file = 0    |
| id            | [u8,16]  | 10     | id of the kym file                       |
| checkInfoPos  | u64      | 34     | `8pointer` to a checkInfo structure      |
| packCount     | u8       | 42     | Number of packInfo slots. (excluding indexPack) |
| freeData      | [u8,21]  | 43     | Free data, application specific to extend the header |
| IndexPackInfo | PackInfo | 46     | Information about the index pack         |
| PackInfo0     | PackInfo | 640    | Information about the first content pack |
| ...           | ...      | ...    | ...                                      |
| PackInfoN     | PackInfo |        | Information about the first content pack |

Full Size : 128 + 128*(packCount+1) bytes

The appVendorId, is a identifier of the vendor (user of kym format).
appVendorId is used to identify the kind of the archive. It could be :
 - Kiwix (html content).
 - A file archive (file/directory with file access right metadata).
 - Embedded resources in a specific executable.
 - A media container (video/sound/subtitle as entry in the kym archive)
 - ...

Major version (M) is updated if there are not compatible changes in the format.
Minor version (m) is updated if there are compatible changes in the format.

A implementation able to read M.m will be able to read M.m+k (by definition of minor).
It SHOULD be able to read M.m-k (by testing minor and conditiannaly use existing features)
It WOULD be able to read M-k.* but it could be assume that two differents major version is
two different formats.

FreeData is a 64 bytes free space to extend the header with application
specific information.

The total size of the header is specific to the Major version. It could change
(and so have less space for freeData) as kym format change.

## PackInfo

The KymHeader is directly (a offset 128) followed by an array of packInfo.

It describe the pack part of a kym file and where to find it.

| Field Name       | Type      | Offset | Description                                                              |
| ---------------- | --------- | ------ | ------------------------------------------------------------------------ |
| id               | [u8,16]   | 0      | The id of the pack, must be the same as the uuid of the pointed pack     |
| packId           | u8        | 16     | The id of the pack. 0 for index pack.                                    |
| freeData         | [u8,111]  | 17     | A 256bytes array free data. Application specific.                        |
| packSize         | u64       | 128    | The size of the pack (including its own checkInfo structure)             |
| packOffset       | u64       | 136    | The offset `8pointer` (starting from the beggining of the kym file) where to find the pack.<br />If ==0, the pack has to be searched on the file system |
| packCheckInfoPos | u64       | 144    | The checkInfo of the pack (mandatory)                                    |
| packPath         | pstring<br/>[u8,104] | 152    | A pString pointing to the path of the pack file. <br /> The array is always 104 length. The max string length : 103. |


Full Size : 512 bytes.

An packOffset and an packPath can be set in the same time. In this case the packOffset is predominant. This can be usefull when a kym file and its packs are concatened together, a tool just have to change the offset from 0 to the offset.

The packPath is always relative to the kym filepath.

This is not an error if an pack cannot be found in the file system. The implementation may warn the user (in case of mistake in the file handling). The implementation MUST correctly handle the pack missing:

- A library can return that the entry cannot be found because an pack is missing.
- A client application warn the user the pack is missing. A client can offer to the user to download the missing pack. html link to a missing entry could be displayed differently (red).

Several packs can share the same id. In this case, they are considered as alternatives.
Each pack with the same id must provide the same entries (but potentially different content). The pack declared first is considered with high priority on the others.  
This can be used to have several packs providing the images (same entries) but differents resolution (different content).

It is to the application to handle correctly the alternatives.

## Content Address

Packs will be described later, but, we know that they can contain up to 2^24-1 content.
And there is up to 2^8-1 packs.

So, a content can be addressed with a u32.
The packId is stored in the highest byte of the u32 : packId = contentAddress >> 24;
The contentNumber (in the pack) is stored in the lowest u24 : contentNumber = contentAddress & 0x0FFF

For the rest of the documentation, the type `contentAddress` refer to a u32.


## Index Pack

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

### Index Pack header

This is the header of an pack. An pack can be store of a fs or part of a kym file.

| Field Name    | Type     | Offset | Description                                                       |
| ------------- | -------- | ------ | ----------------------------------------------------------------- |
| magic         | u32      | 0      | The magic number to detect the type of the file 0x6B796D69 (kymi) |
| appVendorId   | u32      | 4      | An id to identify the usage of the kym file (its usage)           |
| majorVersion  | u8       | 8      | The major version of the pack = 1                                 |
| minorVersion  | u8       | 9      | The minor version of the pack = 0                                 |
| id            | [u8,16]  | 18     | id of the pack                                                    |
| fileSize      | u64      | 10     | The size of the pack (on file system)                             |
| checkInfoPos  | u64      | 48     | The checksum position (allways at end of the pack)                |
| _padding      | u16      | 34     | A padding, may be used in the futur                               |
| indexCount    | u32      | 36     | Number of index in the pack                                       |
| indexPtrPos   | u64      | 40     | A `8pointer` to a array of entryInfo offsets.                     |
| freeData      | [u8,72]  | 56     | Free data, application specific to extend the header              |

Full Size : 128

### Index Header

Each index is specific to a usage and their internal structure can be pretty different.

Each of them share the same first byte :


| Field Name | Type | Offset | Description                                     |
| ---------- | ---- | ------ | ----------------------------------------------- |
| indexType  | u8   | 0      | The type of the index.<br/>Highest bit is 0 for kym index, 1 for application specific |
| headerSize | u16  | 1      | The size of the header                          |



### Plain list index

#### Header

The first kind of index know by kym implementation is a listing of entry, along with some metadata

| Field Name         | Type               | Offset           | Description                                                |
| ------------------ | ------------------ | ---------------- | ---------------------------------------------------------- |
| indexType          | u8                 | 0                | 0                                                          |
| headerSize         | u16                | 1                | The size of the header                                     |
| indexKey           | u8                 | 1                | The primary key of the index.<br/>0 if no primary key.<br/>1 for the first key.<br/>2 for second ... |
| entrySize          | u8                 | 2                | The size of one entry.                                     |
| padding            | u8                 | 3                | Reserved.                                                  |
| indexLength        | u32                | 4                | The number of entry in the index.                          |
| keysDataPos        | u64                | 8                | The offest (relative to the header's start) of the keydata.|
| keysDataLen        | u64                | 16               | The length of the keydata.                                 |
| indexArrayPos      | u64                | 24               | The offset (relative to the header's start) of the entryIndexArray. |
| extraData          | `contentAddress`   | 32               | An app specific content. Used as free form data.           |
| indexName          | `pstring`          | 36               | The name of the index, may be used to indentify the index  |
| keyCount           | u8                 | 37+len(indexName)| The number of keys (metadata associated to each entry)     |
| keyType0           | u8                 |                  | The type of the key0                                       |
| keyType1           | u8                 |                  | The type of the key1                                       |
| ...                | u8                 |                  |                                                            |
| keyTypeN           | u8                 |                  | The type of the keyN                                       |


Full Size : indexArrayOffset + 4*indexLength

The index itself is a array of `indexLength` entries, each entry having a size of `entrySize`.
The array start at `indexArrayPos`.

Each entry is a list of `keyCount` values. The values are described in each `keyTypeN`.
`entrySize` is the sum of the size of all the key in the index.

The kind of the key give its size:

If it is 0, the is `contentAddress`. The size is 4.

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
Implementation are free to provide api returning integer using standard size highest than what is stored.
(They can all the time return a u64 or s64. Or they can return a u32 if a u24 is stored).

`char[]`: is simply an array of char.
`pstring` : this is a pstring but the content is stored in keysData.
It comes with several variants :
- Deported size : This is a classical pstring. The value of the key is simply the offset (in keysData) where to find the pstring.
- Local size : The size of the pstring is stored in the key, not within the pstring.
               So the key is composed of 1B for the size and (SIZE-1)B for the offset
- Fast lookup: The first byte of the pstring is stored in the key.
               So the key is composed of 1B for the first char (SIZE-1)B for the offset.
               The offset points to a pstring not containing the first char. The size of the final string is the size of the pstring+1
- Local size and fast lookup:
               The key is compose of 1B for the size, 1B for the first char and (SIZE-2)B for the offset.
               The offset points directly to the data (not containing the first char). The size of the final string is the size stored in the key + 1.
               
               
`indexKey` : The index can be sorted to allow binary search. The comparaison function is unspecified and left to the vendor.

#### Key Data

The key data is composed of a byte (as a header) and the whole data.
The size in the index's header include this byte.


+---+---+....+---+  
| 0 | plain data |  
+---+---+....+---+  


+---+---+---+---+---+---+---+---+---+---+....+---+  
| 1 | Uncompressed size (8bytes)    | plain data |  
+---+---+---+---+---+---+---+---+---+---+....+---+  

### Indirect list index

This kind of index is usefull to create index and reuse metadata declared in another index(es).
It can be used to sort entries in a different order, or merge several indexes or ...

#### Header

The first kind of index know by kym implementation is a listing of entry, along with some metadata

| Field Name         | Type               | Offset           | Description                                                |
| ------------------ | ------------------ | ---------------- | ---------------------------------------------------------- |
| indexType          | u8                 | 0                | 1 or 2                                                     |
| headerSize         | u16                | 1                | The size of the header                                     |
| baseIndex          | u8                 | 1                | The number of the base index. (0 if indexType is 2)        |
| indexKey           | u8                 | 2                | The primary key of the index.<br/>(using keys declared in base index) |
| padding            | u8                 | 3                | Reserved.                                                  |
| indexLength        | u32                | 4                | The number of entry in the index.<br/>Must be <= to the number of entry in the base index |
| indexArrayPos      | u64                | 24               | The offset (relative to the header's start) of the entryIndexArray. |
| extraData          | `contentAddress`   | 32               | An app specific content. Used as free form data.           |
| indexName          | `pstring`          | 36               | The name of the index, may be used to indentify the index  |


If indexType is 1, the indexArray is a array of u32. Each u32 is the index of the entry in the base index.

If indexType is 2, the indexArray is a array of u40. Each u40 is composed of :

+-----------+------+------+------+------+  
| baseIndex | Entry number in baseIndex |  
+-----------+------+------+------+------+  

If indexType is 2 and indexKey != 0, the different base indexes must be coherent (The indexKey keys of all index must be comparable)


### Overlay index

[TODO]

## Content Pack

### Content Pack header

This is the header of an pack. An pack can be store of a fs or part of a kym file.

| Field Name    | Type     | Offset | Description                                                       |
| ------------- | -------- | ------ | ----------------------------------------------------------------- |
| magic         | u32      | 0      | The magic number to detect the type of the file 0x6B796D63 (kymc) |
| majorVersion  | u8       | 4      | The major version of the pack = 1                                 |
| minorVersion  | u8       | 5      | The minor version of the pack = 0                                 |
| id            | [u8,16]  | 6      | id of the pack                                                    |
| packSize      | u64      | 22     | Size of the pack                                                  |
| checkInfoPos  | u64      | 48     | The checksum position (allways at end of the pack)                |
| \_padding     | u24      | 22     | A padding, may be used in the futur                               |
| entryCount    | u24      | 24     | Number of entry in the pack (max of 2^24 entry per pack)          |
| clusterCount  | u32      | 28     | Number of cluster in the pack      (max of 2^20)                  |
| entryPtrPos   | u64      | 32     | A `8pointer` to a array of entryInfo offsets.                     |
| clusterPtrPos | u64      | 40     | A `8pointer` to a array of cluster offsets.                       |

Full Size : 56 bytes

### ClusterPtrPos array

A array of `8pointer`. Each entry is a offset to the start of a cluster.  Offsets may not be writen sequentially. Offsets are relative to the start of the pack.

### EntryPtrPos array

An array of EntryInfo

### Cluster

A cluster is a container of content. It contains plain data. There is no information about the name or anything else about a file.

| Field Name    | Type     | Offset              | Description                                                               |
| ------------- | -------- | ------------------- | ------------------------------------------------------------------------- |
| type          | u8       | 0                   | The hightest 4 bits are reserved. Must be equal to 0.<br/>The lowest 4 bits are the cluster compression :<br/>0=nocompression<br/>1=lz4<br/>2=lzma<br/>3=zstd |
| clusterSize   | `8size`  | 1                   | The size of the (potentially compressed) cluster  (including this header) |
| blobCount     | u16      | 9                   | The number of blob in the cluster (limited to 2^12==4096)                 |
| offsetSize    | u8       | 11                  | The size (in bytes) of the offsets.<br/>Define uN (N == offsetSize)       |
| dataSize      | uN       | 12                  | The size of the uncompressed data (without the header and the offsets)    |
| blob1 offset  | uN       | 12+uN               | Start of second (1) blob, end of the first blob (0)                       |
| blob2 offset  | uN       | 12+uN*2             | Start of third (2) blob, end of second blob                               |
| ...           | ...      | ...                 | ...                                                                       |
| blobN offset  | uN       | 12+uN*(blobCount)   | Start of the last blob, end of the end of the second to last blob         |
| data          | u8*      | 12+uN*blobCount     | The data, potentially compressed                                          |

blob1..blobN represent a array of dimension blobCount-1

blob0 offset is always 0. Its size is blob1 (array[0])  
blobN (0 < N < blobCount) offset is array[N-1]. Its size is (array[N]-array[N-1])  
blobN (N==blobCount) offset is array[N-1]. It size is (dataSize-array[N-1])  


### Entry info

While the cluster store the data itself, an entry info store metadata about this data.

| Field Name    | Type | Offset | Description                                                             |
| ------------- | ---- | ------ | ----------------------------------------------------------------------- |
| clusterNumber | u32  | 0      | 20 highest bytes = clusterIndex (so 1 048 576 max cluster in an pack)<br/>12 lowest bytes = blobIndex (so 4096 max blob per cluster)|



## Id and Checksum

The id of each pack is the md5sum of the pack itself.
While computing the md5sum :
- id, fileSize and checkInfoPos (offset from 10 to 42) itself is assume to be 0.
- checkInfo is excluded from the md5sum.


CheckInfo is a structure containing information to check the integrety
(and authenticty) of the content.


| Field Name    | Type      | Offset | Description                                                             |
| ------------- | --------- | ------ | ----------------------------------------------------------------------- |
| checkInfoSize | u32       | 0      | The size of the checksum structure.<br/>At least 5 (the checkInfoSize + checkInfoType) |
| checkInfoType | u8        | 4      | Type of the checkinfo. This is a flag.<br/>0b00000001 for fletcher-32 checksum<br/>0b00000010 for sha-256 signature<br/>Highest bit is reserved and must be 0.<br/>If it is set to 1, implementation should assume the format is not compatible. |
| fletcher-32   | u32       | ...    | Fletcher-32 checksum of the content (including the cluster)             |
| sha-256       | [u8, 256] | ...    | SHA-256 signature of the content (including the cluster)                |

All checksum/signature is made assuming the fileSize is 0 and excluding the checkInfo structure.
This allow another signatures to be append to he file without breaking already present signatures.


## Kym Patch

A kym patch is a special kind of kym. It looks like a kym file but is not.

## Kym patch header

| Field Name     | Type     | Offset | Description                                                                                                            |
| -------------- | -------- | ------ | ---------------------------------------------------------------------------------------------------------------------- |
| magic          | u32      | 0      | The magic number to detect the type of the file 0x6B796D70 (kymp)                                                      |
| majorVersion   | u8       | 4      | The major version of the kym patch = 1                                                                                 |
| minorVersion   | u8       | 5      | The minor version of the kym patch = 0                                                                                 |
| uuid           | [u8, 16] | 6      | uuid of the kym patch                                                                                                  |
| baseUuid       | u8, 16   | 22     | The uuid of the base kym file (the one patched)                                                                        |
| extensionCount | u8       | 46     | Number of extensionInfo slots.                                                                                         |
| entryCount     | u32      |        | Total number of entry in the kym patch. This is the number of entry in the patch, not the entry in the final kym file. |
| entryInfoPos   | u64      |        | `8pointer` to the entry information.                                                                                   |
| mimeListPos    | u64      |        | `8pointer` to a list of mimeList.                                                                                      |
| PatchInfoPos   | u64      | 55     | `8pointer` to a list of PatchInfo.                                                                                     |
| IndexPos       | u64      | 63     | `8pointer` to the entry index.                                                                                         |
| checksum       | u64      | 71     |                                                                                                                        |

Full Size : 79 bytes

As for kym file, kym patch header must be directly followed by an array of extensionInfo.

After the patch extensionInfo array, the final kymHeader (and the array of extensionInfo) is directly copied.

### Extensions

Extension are the same format than for kym file.

However, they have to be combined with the base kym extensions following the instruction of PatchInfoPos

### EntryCount and EntryInfo

The entryInfo of the patch have to be merged with the base entryInfos.

\[TODO] Handle remove/rewrite of base entry info. (Use 8th bit to mark a entry to be removed.) Rewrite is just a remove and a add.

### MimeList

MimeList pointed by mimeListPos should be copied at the right position in the final kym file (pointer by the final keyHeader).

Base mimelist is discarded.

The mimeList MUST be "compatible" with the base mimeList (no change in the entryInfo MUST be necessary.)

### PatchInfo list

The patchInfo list give information about how to combine the base kym and the patch kym.

Each PatchInfo entry specify how to generate new extension of the final kym file.

| Field Name   | Type             | Offset | Description                                            |
| ------------ | ---------------- | ------ | ------------------------------------------------------ |
| uuid         | [u8, 16]         | 0      | The uuid of the final extension                        |
| entryCount   | u24              | 16     | Number entry in the final extension                    |
| clusterCount | u24              | 19     | Number of cluster in the patchInfo.                    |
| clusterList  | u32*clusterCount | 22     | The list of cluster to take to generate the extension. |
| checksum     | [u8, 16]         | ...    | The checksum of the final extension.                   |

The entry of the clusterList is a combination of two informations :

* bits 31-32 are not used.

* The 30 bits is 0 if extension number is from base kym or 1 if patch kym.

- the 21-29 highest bits is the extension number. 

- the 20 lowest bits are the cluster number in the extension.

If clusterCount is 0, it means that the extension has not to be generated but simply "copied". The source extension has the same uuid and checksum than the patchInfo.

If PatchInfoPos is null, there is no patchInfo in the patch file. So the patch file has to be considered has if all the base extensions have to be keep unchanged and extension of the patch file to be added at the end. All extension has to be copied.

A base kym file may not have all extensions available localy. In this case, the corresponding PatchInfo should be skip (we cannot apply a patch to a missing content). The new extension is correctly referenced in the final kym header, so client application will be able to download the correct extension later.

If there is more PatchInfo than extension in the base key file, it means that new extension has to be added to the kym file. Extension are simply copied from the kym patch. (clusterCount is 0, uuid and checksum correspond to patch extension)

\[TODO] Add a entryInfo to be able to change the extension entryInfo

### IndexPos



The entryIndexPos is the same of zimFile, with the following exception:

The 2th and 3th bit of indexType are used to :

- 00 : The entryIndexArray is the same that the base index, so no entryIndexArray is provided at the end of the header. FullSize is indexArrayOffset. The header has to be copied from the kym patch (to allow metadata update) and the array from the base kym.

- 01 : The entryIndexArray of the patch has to be merged with the base. This is only valid if the index is a sorted index. indexLength is the length of entry in the patch (not the final). Other field of the patch header as to be copied (to allow metadata update)

- 10: Take the index from the patch and discard the base one.

- 11 Not used.

An implementation may read a patch file "as a zim file". With the extra cost of potential redirection in the code between zim file and patch file.


# Use case

## Classic usage (as now)



Zim file would have only one or two extensions. 

The (main) extension would have two keys (url and title).

The metadata may be store as entry in the only one extension, or in a secondary extension (with only a "name" key)

There will be three indexes : 'Metadata', 'url', 'title'.



However, the new format provide lot more usefull fonctionnality.



## zim variants

We have different variants of a same zim file : No image, No video, ...

For now, mwoffliner launch different zim creation process for each of this variant. So each time, all the downloading/compression/... has to be made several time for the same content (text content is in all zim).

We could create all the variants in the same time :

- Full text of article goes in the "fulltext" extension

- Text without detail goes in the "nodet" extension

- All images goes in the "image" extension

- All video goes in the "video" extension.

Zim files are created at the end of the process by "combining" the extension :

- Full zim with extension "fulltext", "image", "video"

- No vid, no image with only the "fulltext" extension.

- No det, with only the "nodet" extension.

We could also imagine that we create several image extension with different resolution.

As different fulltext extensions with only "WP100", "WP1000" (minus WP100), "WP10000" (minus WP100 and WP1000).

The WP100 zim file with only WP100 extension

The WP1000 zim file with W100 and WP1000 extensions...



As extensions can be stored in a separate file in fs, the would also avoid duplication storage on the server (library.kiwi.org)

The server application (kiwix-serve or other) could slicy change the zimheader to add offsets to extensionlist and "stream" the different extensions as if they were only one file. The client would download only one file, without knowing that everything were store separatly.



## Extension vs patch

Extension is the prefered way when possible.

Extensions are created all together in the same creation process and form a coherent content.

When downloading a "no image" kym file, the .ym file can include all entry indexes but missing the "image" extension. The user would simply have to download the image extension (in the image resoltution she want) to add images to her kym file. No kym file reconstruction or other complex manipulation has to be done.

Patch should be use for kym update (when the content of the patch is not known at base kym creation time).

It is less efficiant as the patch contains the patchInfo list and new indexes. It also need a new kym reconstruction. 


## Allowing a user to change a zim content

A patch file can be used to store changes to a zim file.

If a patch has no PatchInfoPos, the extension of the patch are simply added to the base extensions list.

The index can also be an extension to a base index.

A client application allowing the user to change the content of wikipedia's article would simply store the new (user) version of the article in the patch extension and create the index as extension, with only the entry for the modified article. When application lookup for article, it will first look in the patch index and so, use the modified version.

\[TODO], should we move this in a separated format (zim override), to simplify the format and not mix with a patch ?


