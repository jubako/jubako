=============
Jubako format
=============

Introduction
============

A Jubako container is a logical container composed of different sub containers, called `Pack`s.
Those packs can be stored independently or together in a ContainerPack. (which is itself a pack).
The full content, useful to the user is the combination of different packs.

There are four kinds of pack : manifest, directory, content and container.


Block and Checksum
==================

Jubako packs are composed of blocks.
A block is a range of bytes, with a given offset and size.
What is stored in a block depends of the context but most of the time it will
be a header/tail, a cluster or raw data.

All blocks are followed by a CRC32 (0x04C11DB7) on 4 bytes.
Computing the CRC32 of the data and the following CRC should give a reminder of 0
if data is not corrupted (assuming it was not intentionnal)

Blocks are clearly identified in the specification.
Size of blocks are specified with the inner the block.
Outer size is always inner+4(sizeof CRC32).

No data in Jubako is stored outside of a block.
Implementation should always check the block containing the data before parsing the data.

Base Structures
===============

Integer
-------

All integers are little endian.

Size of integer are specified with the number of bits.
- A 8 bits unsigned integer is called u8
- A 64 bits unsigned integer is called u64.

Strings
-------

- C format. This is the classical array of char ending with a ``\0``. This allow
  string to be as long as needed but need parsing of all the array (find the ``\0``)
  to know size of the string. This is noted as ``cstring`` format.

- Pascal format. This is an array where the first char is the size of the
  string. There is no ``\0`` at the end. The size of the array is the same than a
  ``cstring`` (n + 1). The string is limite to 255 bytes, but a implementation can
  know the size of string (and how many memory to reserve) by simply reading the
  first byte. This is noted as ``pstring`` format.
  
An empty string is the same in ``cstring`` or ``pstring``: a ``\0``.

``cstring`` and ``pstring`` are array of byte (uint8). They are utf8 encoded.
Because of the utf8 encoding, the size of the (Unicode) string may be lower than
the size of the ``pstring``. The size stored in the first byte is the size of the
encoded string.

Offset / Size
-------------

Otherwise specified, Offset and Size are 64 bits (u64).

Size and Offset may be combined together in one u64.
In this case, the u64 is called a SizedOffset.
The 16 first (highest) bits (2 Bytes) of the SizedOffset represent the Size.
The 48 last (lowest) bits of the SizeOffset represent the Offset.

This allow to point a 16MB sized memory at a position up to 256TB.

Header/Tail
-------------

Most of Jubako structure are headers. They are starting the content they are describing
(pack, index, ...)

However, some structures are tails. They are written at the **end** of the content
they are describing.

Offset (and SizedOffset) always point to the start of the header/tail. Never
to the start of the content.

- In the case of the header, the data is directly following the header, without padding.
- In the case of the tail, the data is just before the tail, without padding.
  The start of the data can be computed by subscribing the data size (specified in the tail) to the offset of the tail.

- `pack <pack.rst>`_ describe the common structures of all packs.
- The `manifest <manifest.rst>`_. It is mainly a header "pointing" to other subcontent.
- The `pack <pack.rst>`_. This is where contents are stored.
- The `directory <directory.rst>`_. This is where all indexes are stored, pointing to content in the packs.
