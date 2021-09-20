Why a new format ?
==================

Working on libzim I discover few "mistakes" that not ease the reading or
creation process.

- Dirent has no size information. The size of a dirent depends of the size of
  the url and the title, and there is no size information in the header.
  So you have to parse a the dirent to know its size. You cannot read directly
  the title because you don't where it is (you have to search for '\\0', to know
  the end of the url)
- Cluster has no size information. You cannot now directly the size of a
  cluster. For an uncompressed cluster you can find the size quite easily has the
  header is not compressed. But for compressed cluster, you have to uncompress the
  data (and you don't know the size of the compressed data, nor the uncompressed
  one) to be able to read the data.
- At creation, the size of the "Header"'s datas is not known before you know all
  the content in the zim file. So you cannot start to write the content directly
  in the zim file. You have to write things in temporary file and keep data
  structure in memory. And so you cannot create big zim file on computer with
  small ram.

We also want to do a series of improvement in the zim format :

- No more namespace. The separation between the article namespace (A) and
  Image (I) is totally useless. The (B) namespace is not used at all.
  Only the metadata (M) namespace is really use.
  The (X) namespace for index is only used by only one article (xapian database).
  It could be merge somewhere else, in the M namespace or directly in the header.
  See https://github.com/openzim/libzim/issues/15
- We want content signing. See https://github.com/openzim/libzim/issues/40
- Category handling. See https://github.com/openzim/libzim/issues/75
- We want to be able to split zim files efficiently.
- We want to have zim extensions. Having a small "base" zim file we may want to
  have extension to new content. Image is the base zim file is without image.
  Or new articles if the base zim is a selection of articles.
- We may want to have different kind of extensions. Low and high resolution
  image.
- We want to handle zim update. New version of a zim file could come as an
  update to a previous zim. This way, we avoid to the user to download all the
  content again.
- Zim update should be easily doable. When displaying a wikimedia content, a
  client application may allow the user to change the content of an article
  (as wikipedia does), and store the change as a zim update.

While all this improvement concerns the kiwix usage, I also want to explore new
use case of an advanced archive format. For exemple:

- Classical file system archive
- Backup
- Software distribution
- Packaging
- ...

This work is made independently from kiwix or openzim organization.
For now this is more an essay than a real project to implement this.
It may change in the futur but for now there is absolutly no plan nor promise
that I (or other) will implement this format.
