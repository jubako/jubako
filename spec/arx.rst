==========
Arx format
==========

ArxHeader
=========

This is the main header.

============= ======== ====== ===========
Field Name    Type     Offset Description
============= ======== ====== ===========
magic         u32      0      The magic number to detect the type of the file
                              0x61727866 (arxf)
appVendorId   u32      4      An id to identify the usage of the arx file (its usage)
majorVersion  u8       8      The major version of the arx file = 1
minorVersion  u8       9      The minor version of the arx file = 0
id            [u8,16]  10     id of the arx file
checkInfoPos  u64      34     ``8pointer`` to a checkInfo structure
packCount     u8       42     Number of packInfo slots. (excluding indexPack)
freeData      [u8,21]  43     Free data, application specific to extend the header
IndexPackInfo PackInfo 46     Information about the index pack
PackInfo0     PackInfo 640    Information about the first content pack
...           ...      ...    ...
PackInfoN     PackInfo        Information about the first content pack
============= ======== ====== ===========

Full Size : 128 + 128*(packCount+1) bytes

The appVendorId, is a identifier of the vendor (user of arx format).
appVendorId is used to identify the kind of the archive. It could be :

- Kiwix (html content).
- A file archive (file/directory with file access right metadata).
- Embedded resources in a specific executable.
- A media container (video/sound/subtitle as entry in the arx archive)
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
(and so have less space for freeData) as arx format change.

PackInfo
========

The ArxHeader is directly (a offset 128) followed by an array of packInfo.

It describe the pack part of a arx file and where to find it.


================ ========= ====== ===========
Field Name       Type      Offset Description
================ ========= ====== ===========
id               [u8,16]   0      The id of the pack, must be the same as the uuid of the
                                  pointed pack
packId           u8        16     The id of the pack. 0 for index pack.
freeData         [u8,111]  17     A 256 bytes array free data. Application specific.
packSize         u64       128    The size of the pack
                                  (including its own checkInfo structure)
packOffset       u64       136    | The offset ``8pointer`` (starting from the beggining of
                                    the arx file) where to find the pack.
                                  | If ==0, the pack has to be searched on the file system
packCheckInfoPos u64       144    The checkInfo of the pack (mandatory)
packPath         pstring   152    | A pString pointing to the path of the pack file
                 [u8, 104]        | The array is always 104 length.
                                    The max string length : 103.
================ ========= ====== ===========

Full Size : 256 bytes.

An packOffset and an packPath can be set in the same time. In this case the packOffset is predominant. This can be usefull when a arx file and its packs are concatened together, a tool just have to change the offset from 0 to the offset.

The packPath is always relative to the arx filepath.

This is not an error if an pack cannot be found in the file system. The implementation may warn the user (in case of mistake in the file handling). The implementation MUST correctly handle the pack missing:

- A library can return that the entry cannot be found because an pack is missing.
- A client application warn the user the pack is missing. A client can offer to the user to download the missing pack. html link to a missing entry could be displayed differently (red).

Several packs can share the same id. In this case, they are considered as alternatives.
Each pack with the same id must provide the same entries (but potentially different content). The pack declared first is considered with high priority on the others.  
This can be used to have several packs providing the images (same entries) but differents resolution (different content).

It is to the application to handle correctly the alternatives.

