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
packsPos      Offset   0      A offset to a array of PackLocator
packCount     u16      8      The number of pack contained in the container.
_reserved     [u8;54]  10     MUST be 0.
============= ======== ====== ===========

The size of of this header, is 64 bytes

This mainly reuse the same structure than Pack header.
Readers may want to always parse the first 64 bytes of a pack as a PackHeader to gather basic
information on it (as knowing its kind and size).

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
packSize         Size      16     The size of the pack.
                                  Must be equal to the packSize in the packheader of the pointed pack
packOffset       Offset    24     | The offset (starting from the beginning of
                                    the container file) where to find the pack.
================ ========= ====== ===========

Full Size : 24 bytes.
