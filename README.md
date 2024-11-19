# Jubako

Jubako is a new, open, efficient, and flexible container format designed to store and manage diverse data types, including text or binary files.
It emphasizes speed and efficiency, offering fast compression and decompression with minimal overhead.
This makes it ideal for managing large datasets and facilitating efficient distribution.

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
- The metadata (stored in the entries) are not defined.
  Metadata are stored following a schema each application must define.
- Each entries can point to one content (basic use case) but it is not necessary.
  An entry can point to several contents or none.
- The content can come in different variants. For example, images can be in low and high
  resolution.
- Jubako can be incremental. It is possible to create archive containing only the
  difference between an existing archive and the content you want to store. (To implement)
- Content can be put in different packs inside a container. Packs may be missing or
  reused in another Jubako container.


## What Jubako is not ?

Jubako is a not file format.

As xml, Jubako is a format describing how to store content and how it is
structured. It doesn't specify what is stored and the hierarchy between those content.

The classical usage Jubako is to be used as base structure for a real life container.

## Using Jubako

Jubako library is the low level library to read and write Jubako container.
Jubako format is somehow a metaformat, each user (arx, waj, you...) of Jubako has to
specify its own format based on Jubako.

So, the classic use case is to create a library on top of jubako to wrap jubako
structure and provide high level implementation.

See [examples](examples) to see how to use it.

You can have a look to [arx](https://github.com/jubako/arx) which is files
archive based on Jubako or [waj](https://github.com/jubako/waj) to store website.

But as all Jubako files are in Jubako format. So waj, arx or anything else are Jubako files.
Jubako provide some tools to manipulate them in a generic way.

You can install thoses tools using:
`cargo install jubako --features build_bin`

**Note:** The `build_bin` feature is required for the `jbk` command-line interface.
If these features are not included in the installation, you'll only have access to the Jubako library, not the CLI tools.


## Usage Examples

### CLI Tool (`jbk`)

After installing with the `build_bin` feature, the `jbk` command-line tool becomes available:

* Checks the integrity of a Jubako archive:

   ```bash
   jbk check my_archive.jbk
   ```

* Manages location of packs within a Jubako container.

   Packs may be stored in one file or separatly. Locations of packs are stored in the manifest pack and can be modified:

   ```bash
   jbk locate my_archive.jbk a1b2c3d4-e5f6-7890-1234-567890abcdef # print location of pack `a1b2c3d4-e5f6-7890-1234-567890abcdef`
   jbk locate my_archive.jbk a1b2c3d4-e5f6-7890-1234-567890abcdef new/path/to/pack.jbkc # change location of pack `a1b2c3d4-e5f6-7890-1234-567890abcdef`
   ```

* Explores internal structures of a Jubako archive.

    ```bash
    jbk explore my_archive.jbk <key_part>::<key_part>::...
    ```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## Sponsoring

I ([@mgautierfr](https://github.com/mgautierfr)) am a freelance developer. All jubako projects are created in my free time, which competes with my paid work.
If you want me to be able to spend more time on Jubako projects, please consider [sponsoring me](https://github.com/sponsors/jubako).
You can also donate on [liberapay](https://liberapay.com/jubako/donate) or [buy me a coffee](https://buymeacoffee.com/jubako).

## License

This project is licensed under the MIT License - see the [LICENSE-MIT](LICENSE-MIT) file for details.
