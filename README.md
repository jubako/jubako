# Jubako

## What is Jubako ?

Jūbako is the traditional lunch box used in Japan to store Bentos.
It is a small box that stores food in small compartments.

Jubako is a container format to store things in organized manner.
It is composed of packs that can be composed as needed.

It is container format extensible to fulfill specific need.
As any container format it allow to store content in the container.
It has some specificity :

- Content can be compressed or not. Decide whether the content is compressed or not is made
  at content level.
- Direct access. You don't need to decompress the whole archive on the file system or in
  memory to access a content.
- Content is accessed using one or several entries stored in indexes.
- The metadata (stored in the entries) are not defined. Each use case can (and must)
  specify which metadata to store.
- Each entry can point to one content (basic use case) but it is not necessary.
  An entry can point to several contents or none.
- The content can come in different variants. For example, images can be in low and high
  resolution.
- Jubako can be incremental. It is possible to create archive containing only the
  difference between an existing archive and the content you want to store. (To implement)
- Content can be put in different packs inside a container. Packs may be missing or
  reused in another Jubako container.


## What Jubako is not ?

Jubako is not a file format.

As xml, Jubako is a format describing how to store content and how it is
structured. It doesn't specify what is stored and the hierarchy between those content.

The classical usage Jubako is to be used as base structure for a real life container.

## Using Jubako

Jubako library is the low level library to read and write Jubako container.
Jubako format is somehow a metaformat, each user (vendor) of Jubako has to
specify its own format based on Jubako.

So, the classic use case is to create a library on top of jubako to wrap jubako
structure and provide high level implementation.

You can have a look to [arx](https://github.com/jubako/arx) which is file
archive based on Jubako or [waj](https://github.com/jubako/waj) to store website.


## Specification

You can find the specification and other documentation in the `spec` directory.
