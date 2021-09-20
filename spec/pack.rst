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
majorVersion  u8      8      The major version of the pack = 1
minorVersion  u8      9      The minor version of the pack = 0
id            [u8,16] 10     uuid of the pack
_padding      [u8, 6] 26     MUST be 0.
packSize      Size    32     Size of the pack
checkInfoPos  Offset  40     The checksum position (allways at end of the pack)
_reserved     [u8,16] 48     MUST be 0.
============= ======= ====== ===========

Full Size : 64 bytes

This header is extended with a specific header for each pack kind.


appVendorId
-----------

The appVendorId, is a identifier of the vendor (user of Jubako format).

appVendorId is used to identify the kind of the archive. It could be :

- Kiwix (html content).
- A file archive (file/directory with file access right metadata).
- Embedded resources in a specific executable.
- A media container (video/sound/subtitle as entry in the Jubako archive)
- ...

The couple ``magic``/``appVendorId`` must be considered as the true, full magic number of the format.


Version
-------

Major version (M) is updated if there are not compatible changes in the format.
Minor version (m) is updated if there are compatible changes in the format.

A implementation able to read M.m will be able to read M.m+k (by definition of minor).
It SHOULD be able to read M.m-k (by testing minor and conditiannaly use existing features)
It WOULD be able to read M-k.* but it could be assume that two differents major version is
two different formats.



CheckInfo tailer
================

All pack must end with a checkInfo structure.

``packSize`` - ``checkInfoPos`` give the size of this structure.

The global structure of checkInfo is a series of checks.
Each series of check start with a byte telling which it is and by the data of the actual check.

For now, two checks are supported :

- 0 : No check. No data.
- 1 : Blake3 chek. The data is the 32 bytes checksum.

CheckInfo is mandatory so the mimimun length of CheckInfo is 1 Byte (no check).
For now, maximum length is 33 bytes.

New check kind will be added in the future.

The checksum is computed base of the whole content of the pack, from Offset(0) to Offset(checkInfoPos).
The mainPack is the only exception to this as we mask some mutable data (See Jubako mainPack spec for this).
