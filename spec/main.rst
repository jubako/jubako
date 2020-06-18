==========
Arx format
==========


Main ideas
==========


The idea is to have a different kind of subcontent. Those subcontent could be
stored as independent files or all in one file (concat). The full content,
usefull to the user is the combination of different subcontents.

Base Structures
===============

Integer
-------

All integers are bigendian. This allow comparaison of integer to be made the
same way as comparaison of bytes array.

Strings
-------

- C format. This is the classical array of char ending with a ``\0``. This allow
  string to be as long as needed but need parsing of all the array (find the ``\0``)
  to know size of the string. This is noted as `cstring` format.

- Pascal format. This is an array where the first char is the size of the
  string. There is no ``\0`` at the end. The size of the array is the same than a
  ``cstring`` (n + 1). The string is limite to 255 chars, but a implementation can
  know the size of string (and how many memory to reserve) by simply reading the
  first char. This is noted as ``pstring`` format.

An empty string is the same in ``cstring`` or ``pstring``  : a ``\0`` .

Offset / Size
-------------

Most of the time, offset and size need to be more than 4Gb. So we need to use
more than 32 bits.

- We can store them on 64 bits. This allow a direct mapping to C type pointer.
  However it propably uses too many bits

- We can store them on 40 bits (5 bytes). This allow use to store offsets up to
  1To. Most of the time it is enough.

Depending of how the offset will be used, we may use a ``8pointer`` or a ``5pointer``.

``8pointer`` will be used by default as it is the "native" type.

``5pointer`` will be used in array where they will be a lot of pointer "repetition".

Offset stored on 32 bits will be named ``4pointer``

Array vs List
-------------

Array in the rest of this document is an array of element of the same size.

An Nth element can be directly acceded at offset : ``ArrayOffset + N * size(element)``

A list is a series of elements than can be size differently. So it is "almost"
impossible to have a direct access to a Nth element. A implementation will have
to read all previous elements (to know their size) before being able to access
to the Nth element.

A list may be doubled with a offset array. With this "double" structure,
a Nth element can be acceded at offset :

``ListOffset + *(ArrayOffset + N * size(element))``

Content Part
============

There are three kind of subcontent:

- The `arx <arx.rst>`_ file itself. It is mainly a header "pointing" to other subcontent.
- The `pack <pack.rst>`_. This is where contents are stored.
- The `directory <directory.rst>`_. This is where all indexes are stored, pointing to content in the packs.
