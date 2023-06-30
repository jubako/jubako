======
Jubako
======

Use case
========

Replace zim
-----------

The `zim format<https://github.com/openzim/libzim>`_ is a archive format to store content (mainly html content) in one archive.

It shares a lot of feature of Jubako. (In fact, Jubako is inspired from zim).

Jubako could be use to replace zim format :

- Content would be put in one pack
- An index for the entries with four keys : url, title, mimetype and a contentAddress.
- An index for the redirection with three keys : url, title, index (to the entry).
- An index for the metadata with two keys : name and value.
- An index for the indexes with two keys : name and contentAddress.

Add variants to zim
-------------------

The main usage of zim format is the Kiwix project.

The Kiwix project creates different variants of a same zim file :
Full, No image, No video, ...

Jubako could be use to define a new format which handle those variants.

For example:

The directory structure would be the same as a "simple" zim. But:

- Full text of article goes in the "fulltext" pack
- Text without detail goes in the "nodet" pack
- All images goes in the "image" pack
- All video goes in the "video" pack

The final Jubako files would be created by combining the packs:

- Full Jubako with packs "fulltext", "image", "video"
- No vid, no image with only the "fulltext" pack
- No det, with only the "nodet" pack.

We could also imagine that we create several image packs with different resolutions

The same way, we could create different fulltext packs with only "WP100", "WP1000"
(minus WP100), "WP10000" (minus WP100 and WP1000), then we will create the Jubako files
with :

- The WP100 pack for the WP100 Jubako file.
- The WP100 and WP1000 packs for the WP1000 Jubako file.

As all those packs store the "same" content. They could be created in the same round.

And as packs can be stored as separated files in the fs so we could avoid duplication
storage on the server (library.kiwi.org).

The server application (kiwix-serve or other) could slicy change the Jubako main header
to set the offsets to packs and "stream" the different packs as if they were only one
file. The client would download only one file, without knowing that everything were
store separately.

Allowing a user to change an Jubako content
-------------------------------------------

A overlay file can be used to store changes to a Jubako file.

A client application allowing the user to change the content of wikipedia's article
would simply store the new (user) version of the article in the overlay Jubako.
The article content would be store in the overlay.
When application lookup for article, it will first look in the overlay Jubako and so,
use the modified version.

File Archive
------------

Jubako file can be use to archive as other classic archive does (tar, zip).

Index would store keys:

- name
- permission
- uid
- gid
- mtime
- contentAddress

Another keys could be added to handle symlink or directory.
Two entries using the same contentAddress could be used for hardlink.

As content can be accessed without full decompression, an Jubako file could be fuse-mount
to access its content read only.
In conjunction with an overlay Jubako, it could be possible to create read/write mount.

Other
-----

- Using Jubako overlay, it would be possible to create incremental backup.
- Embed Jubako container as resource in a binary.
- Store python program in a Jubako file, along side a modified python interpreter to look
  file in the Jubako file.
- Use Jubako file as media container.

