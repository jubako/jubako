====
Pack
====

Pack represent part of a Jubako container.
They can be stored concatenated to each other or stored in separated unit (file, url, ...)


Pack header
===========

All packs starts with the same header

============= ======= ====== ===========
Field Name    Type    Offset Description
============= ======= ====== ===========
magic         u32     0      The magic number to detect the type of the file
appVendorId   u32     4      Specific magic number to identify specific usage
majorVersion  u8      8      The major version of the pack = 0
minorVersion  u8      9      The minor version of the pack = 1
id            [u8;16] 10     uuid of the pack
flags         u8      26     Some flags (must be 0)
_reserved     [u8; 5] 27     MUST be 0.
packSize      Size    32     Size of the pack
checkInfoPos  Offset  40     The checksum position (always at end of the pack)
packCount*    u16     48     | PackCount in container pack.
                             | Only valid if magic == 0x6a626b.. (jbkC).
                             | Else must be 0 and considered as _reserved.
_reserved     [u8,10] 50     MUST be 0.
CRC32         u32     60     The CRC32 of the 60 first bytes of the header
============= ======= ====== ===========

The size of of this header, is 64 bytes.
ContainerHeader is a 60 bytes block.


Full Size : 64 bytes

This header is extended with a specific header for each pack kind.


magic
-----

The magic number is the identifier of the kind of pack/file.

For manifest pack the value is 0x6a626b6d (ascii encoding of jbkm)
For directory pack the value is 0x6a626b64 (ascii encoding of jbkd)
For content pack the value is 0x6a626b63 (ascii encoding of jbkc)

Another magic values may be added in the futur if new kind of pack is created.
All magic values will start with the bytes [0x6a, 0x62, 6x6b] (jbk)


appVendorId
-----------

The appVendorId, is a identifier of the vendor (user of Jubako format).

appVendorId is used to identify the kind of the archive. It could be :

- Kiwix (html content).
- A file archive (file/directory with file access right metadata).
- Embedded resources in a specific executable.
- A media container (video/sound/subtitle as entry in the Jubako archive)
- ...

The couple ``magic``/``appVendorId`` must be considered as the true,
full magic number of the format.


Version
-------

Major version (M) is updated if there are not compatible changes in the format.
Minor version (m) is updated if there are compatible changes in the format.

A implementation able to read M.m will be able to read M.m+k (by definition of minor).
It SHOULD be able to read M.m-k (by testing minor and conditionally use existing features)
It WOULD be able to read M-k.* but it could be assume that two different major version is
two different formats.

Current major version is ``0`` and format is considered as unstable.
Any change in minor version must be handled as a breaking change.
Major version will be moved to ``1`` when Jubako format will be stable.


CheckInfo tail
==============

All packs must contain a checkInfo structure at its end (just before the Pack tail).

``packSize`` - 64 - ``checkInfoPos`` give the size of this structure (Outer size of the block).

The global structure of checkInfo is a series of checks.
Each series of check start with a byte telling which it is and by the data of the actual check.

For now, two checks are supported :

- 0 : No check. No data.
- 1 : Blake3 check. The data is the 32 bytes checksum.

CheckInfo is mandatory so the mimimun length of CheckInfo is 1 Byte (no check).
For now, maximum length is 33 bytes.

New check kind will be added in the future.

The checksum is computed base of the whole content of the pack, from Offset(0) to Offset(checkInfoPos).
The manifestPack is the only exception to this as we mask some mutable data (See Jubako manifestPack spec for this).

CheckInfo is a ``packSize`` - 64 - 4 - ``checkInfoPos`` block.


Pack tail
=========

All packs must end with a pack tail

The pack tail is a 64 bytes buffer. It is the exact same value of the 64 bytes buffer of the header but byte swapped.
The pack tail must be included in the packSize.
