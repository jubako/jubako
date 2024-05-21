==============
Container pack
==============

The container pack is a light weigh pack containing other pack.
It is a simple structure to locate other pack in it.

Jubako containerHeader
=====================

Contrairy to other packs, container pack doesn't start with a pack header.
The full header is:

============= ======== ====== ===========
Field Name    Type     Offset Description
============= ======== ====== ===========
magic         u32      0      The magic number to detect the type of the file
version       u8       4      The version of the container
packCount     u16      5      The number of pack contained in the container.
_padding      u8       7
size          Size     8     The size of the file (include header and tail)
============= ======== ====== ===========

The size of of this header, is 16 bytes.

PackLocator
===========

At the end of the container pack (just before the tail), there is a array of packLocator.
There is ``packCount`` packLocator.

It describe the pack parts of a Jubako container and where to find them.


================ ========= ====== ===========
Field Name       Type      Offset Description
================ ========= ====== ===========
id               [u8,16]   0      The id of the pack
                                  Must be equal to the id in the packheader of the pointed pack
packSize         Size      120    The size of the pack.
                                  Must be equal to the packSize in the packheader of the pointed pack
packOffset       Offset    128    | The offset (starting from the beginning of
                                    the container file) where to find the pack.
================ ========= ====== ===========

Full Size : 24 bytes.
