# What is Arx ?

Arx means "ARchive eXtensible".

It is archive format extensible to furfill specific need.
As any archive format it allow to store content in the archive. It has some specificties :
- Content can be compressed or not. Both decision can be made in the same archive to different content.
- Direct access. You don't need to decompress the whole archive on the filesystem or in memory to access a content.
- Content is accessed using one or several entries stored in indexes.
- The metadata (stored in te entries) are not defined. Each use case can (and must) specify which metadata to store.
- Each entry can point to one content (basic use case) but it is not necessary. An entry can point to several content or none.
- The content can come in different variants. For exemple, images can be in low and high resolution.
- Arx can be incremental. It is possible to create archive containing only the difference between an existing archive and the content you want to store.


# Technical point

The specification (WIP) is available [here](spec/main.md)

# Use case

## Replace zim.

Arx archives could be use to replace zim format.
- Content would be put in one pack
- An index for the entries with four keys : url, title, mimetype and a contentAddress.
- An index for the redirection with three keys : url, title, index (to the entry).
- An index for the metadata with two keys : name and value.
- An index for the indexes with two keys : name and contentAddress.

## Add variants to zim.

We have different variants of a same zim file : No image, No video, ...

The directory structure would be the same as a "simple" zim. But:
- Full text of article goes in the "fulltext" pack
- Text without detail goes in the "nodet" pack
- All images goes in the "image" pack
- All video goes in the "video" pack

The final arx files would be created by combining the packs:
- Full arx with packs "fulltext", "image", "video"
- No vid, no image with only the "fulltext" pack
- No det, with only the "nodet" pack.

We could also imagine that we create several image packs with different resolutions

The same way, we could create different fulltext packs with only "WP100", "WP1000"
(minus WP100), "WP10000" (minus WP100 and WP1000), then we will create the arx files with:
- The WP100 pack for the WP100 arx file.
- The WP100 and WP1000 packs for the WP1000 arx file.

As all those packs store the "same" content. They could be created in the same round.

And as packs can be stored as separated files in the fs so we could avoid dupliaction
storage on the server (library.kiwi.org).

The server application (kiwix-serve or other) could slicy change the arxheader to set
the offsets to packs and "stream" the different packs as if they were only one file.
The client would download only one file, without knowing that everything were store
separatly.

## Allowing a user to change an arx content

A overlay file can be used to store changes to a arx file.

A client application allowing the user to change the content of wikipedia's article
would simply store the new (user) version of the article in the overlay arx.
The article content would be store in the overlay.
When application lookup for article, it will first look in the overlay arx and so,
use the modified version.

## File Archive.

Arx file can be use to archive as other classic archive does (tar, zip).

Index would store keys:
- name
- permission
- uid
- gid
- mtime
- contentAddress

Another keys could be added to handle symlink or directory.
Two entries using the same contentAddress could be used for hardlink.

As content can be accessed without full decompression, an arx file could be fuse-mount to access its content read only.
In conjuction with an overlay arx, it could be possible to create read/write mount.

## Other

- Using arx overlay, it would be possible to create incremental backup.
- Embend arx file as resource in a binary.
- Store python programme in a arx file, along side a modified python interpreter to look file in the arx file.
- Use arx file as mediacontainer.

