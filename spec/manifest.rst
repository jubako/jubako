=============
Manifest pack
=============

The manifest pack first pack accessed when reading a container.
This is the pack that represent the "container".
It give all main information about the container and list other packs.

Jubako manifestHeader
=====================

As any other pack, a manifest pack must start with a pack header.
This pack header is followed by a mainHeader.

================ =========== ====== ===========
Field Name       Type        Offset Description
================ =========== ====== ===========
packCount        u16         0      Number of packInfo slots.
valueStoreOffset SizedOffset 2      The offset of a valuestore
_reserved        [u8;26]     10     Reserved, must be 0;
freeData         [u8;24]     36     Free data, application specific to extend the header
================ =========== ====== ===========

The size of of this header, is 60 bytes.
Associated to the common pack header, the total header size is 128 bytes.
FreeData is a 24 bytes free space to extend the header with application specific information.
Manifest header is a 60 bytes block.

ValueStore
==========

A Manifest pack always containt a value store to store extra data.
This value store is a indexed value store and the first value is always the empty value.
The index 0 acts as a place holder for "no value".

PackInfo
========

At the end of the manifest pack (just before the checkInfo), there is a array of packInfo.
There is ``packCount+1`` packInfo (one for the directoryPack and ``packCount`` for the contentPacks)

It describe the pack parts of a Jubako container and where to find them.


================ =========== ====== ===========
Field Name       Type        Offset Description
================ =========== ====== ===========
uuid             [u8,16]     0      The id of the pack
                                    Must be equal to the id in the packheader of the pointed pack
packSize         Size        16     The size of the pack.
packCheckInfoPos SizedOffset 24     The checkInfo of the pack (mandatory)
packId           u16         32     The id of the pack.
packKind         u8          34     | The kind of the pack.
                                    | b'm' for manifest pack
                                    | b'c' for content pack
                                    | b'd' for directory pack
packGroup        u8          35     Reserved
freeDataId       u16         36     A id in the value store. Application specific.
packLocation     [u8,214]    38     A string locating the pack file
================ =========== ====== ===========

Full Size : 252 bytes.
A pack info is a 252 bytes block.

The packLocation is a URL locating the pack file. For now, two kind of value are possible:
- An empty value : The pack is contained in the current Container pack (only valid if the manifest pack is itself in a container pack)
- A relative path : The pack is located in the file pointed by the path, relative to the directory containing the manifest pack (or container)
- An absolute path : The pack is located in the file pointed by the path.
- A URL with a specified scheme. For now, only the "file:" scheme is supported.

If the manifest pack is in a container pack, implementation SHOULD check for the presence of the pack in the container before using packLocation.
This allow combination of packs in a container pack without modifying the packInfo.

The packLocation is purely informal, implementations are free to read packs from other source.

This is not an error if an pack cannot be found. The implementation may warn the user (in case of mistake in the file handling). The implementation MUST correctly handle the pack missing:

- A library can return that the entry cannot be found because an pack is missing.
- A client application warn the user the pack is missing. A client can offer to the user to download the missing pack. html link to a missing entry could be displayed differently (red).


Several packs can share the same id. In this case, they are considered as alternatives.
Each pack with the same id must provide the same entries (but potentially different content). The pack declared first is considered with high priority on the others.
This can be used to have several packs providing the images (same entries) but different resolution (different content).

It is to the application to handle correctly the alternatives.


The checkInfo tail of each packs must be copied in the manifest pack.
(If the corresponding pack are not including in the manifest pack)

Manifest checksum
=================

Some ``packLocation`` of each ``PackInfo`` is considerered as mutable.
Implementation must be able to rewrite it without invalidating the pack.

To do so, we have to exclude those bytes when computing the checksum:

- Computation of CRC of ``PackInfo`` block doesn't change. When a implementation
  changes ``packLocation``, it MUST recompute the CRC.
- Global checksum (stored in ``packCheckInfo``) is computed as if ``packLocation``
  and CRC bytes were equal to zero. (ie: ``packLocation`` and ``packInfo``'s CRC are masked with ``0x00``)
