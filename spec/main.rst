=========
Head pack
=========

The head pack is the main pack.
This is the pack that represent the "container".
It give all main information about the container and list other packs.

Jubako mainHeader
=================

As any other pack, a header pack must start with pack header.
This pack header is followed by a mainHeader.

============= ======== ====== ===========
Field Name    Type     Offset Description
============= ======== ====== ===========
packCount     u8       0      Number of packInfo slots. (excluding directoryPack)
freeData      [u8,63]  1      Free data, application specific to extend the header

The size of of this header, is 64 bytes. Associated to the common pack header, the total header size is 128 bytes.
FreeData is a 63 bytes free space to extend the header with application specific information.

PackInfo
========

The mainHeader is directly (at offset 128) followed by an array of packInfo.
There is ``packCount+1`` packInfo (one for the directoryPack and ``packCount`` for the contentPacks)

It describe the pack parts of a Jubako container and where to find them.


================ ========= ====== ===========
Field Name       Type      Offset Description
================ ========= ====== ===========
id               [u8,16]   0      The id of the pack
                                  Must be equal to the id in the packheader of the pointed pack
packId           u8        16     The id of the pack. 0 for index pack.
freeData         [u8,103]  17     A 256 bytes array free data. Application specific.
packSize         Size      120    The size of the pack.
                                  Must be equal to the packSize in the packheader of the pointed pack
packOffset       Offset    128    | The offset (starting from the beggining of
                                    the file) where to find the pack.
                                  | If ==0, the pack is not concatenate and must be located somewhere else (file system, bdd, ...)
packCheckInfoPos Offset    136    The checkInfo of the pack (mandatory)
packPath         pstring   144    | A pString pointing to the path of the pack file
                 [u8, 112]        | The array is always 104 length.
                                    The max string length : 112.
================ ========= ====== ===========

Full Size : 256 bytes.

An packOffset and an packPath can be set in the same time. In this case the packOffset is predominant. This can be usefull when a Jubako head file and its packs are concatened together, a tool just have to change the offset from 0 to the offset.

The packPath is always relative to the head pack filepath.

This is not an error if an pack cannot be found in the file system. The implementation may warn the user (in case of mistake in the file handling). The implementation MUST correctly handle the pack missing:

- A library can return that the entry cannot be found because an pack is missing.
- A client application warn the user the pack is missing. A client can offer to the user to download the missing pack. html link to a missing entry could be displayed differently (red).

Several packs can share the same id. In this case, they are considered as alternatives.
Each pack with the same id must provide the same entries (but potentially different content). The pack declared first is considered with high priority on the others.  
This can be used to have several packs providing the images (same entries) but differents resolution (different content).

It is to the application to handle correctly the alternatives.


The checkInfo tailler of each packs must be copied in the head pack.
