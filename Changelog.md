# Jubako 0.2.1

- Use dependency `xz2` instead of `rust-lzma` for lzma compression
- Improved CI

# Jubako 0.2

This is a huge update !!

## Format

### Introduce a common part in entry definition


Instead of having entry being composed of Variants, now entries are composed of a common part
containing all attributes common to all variants and then a variant part with attributes specfic to one variant.

By moving common attributes in a common part, implementation doesn't have to check the variant id before
reading those attributes. We also ensure the attributes are actually the same (which was not the case between variants),
so we only need one builder.

### Introduce a pack tail

The pack tail is a copy of the pack header, bytes swapped.
This allow reader to open a pack "by the end".
This way, we can prepend any content to a pack file and still have a valid pack.

This is especially usefull to create auto-(extractible|moutable|servable) content by concatening
a binary trying to open itself and a Jubako container.


### Introduce deported value store

Value store was already existing to store variable length attribute (in a by nature fixed length entry).
Now, it is possible to store any integer attribute in deported value store.

It is usefull for attribute composed of few big value.
For example, file owner in a arx archive is a integer around `1000`, `1001`... and so need two bytes of storage
per entry. By moving the value in a value store, we only have to store the index in the store and so, we need only one byte of storage.


### Introduce default value

If all entries have the same value for an attribute, we can now store this value directly in the entry layout.
Nothing is stored in the entry itself.
For example, file's owner in an arx archive is probably always the same (`1000`). By storing this value in the layout, we need
zero byte storage per entry.

### Allow the pack_id to be stored in a u16

By storing the pack_id in a u16, we are not limited to 256 packs.
This open the doors to Jubako container composed of a lots of packs (delta packs, regrouped packs, ...)

### Variable size content address

ContentAddress are not always 4 bytes. The content id can now be stored using 1, 2 or 3 bytes.
If all pack_id are the same, it will be stored direcly in the layout to earn one or two bytes per entries.

### Names for attributes and variants

Attributes and variants are now named. Instead of accessible them using a index, they can (and must) be accessed using
a name.

- Better self descriptible layout
- Implementation are more tollerent to compatibility issue as implementation can know if a attributes/variants is present
  in a container or not.

### Introduce ContainerPack

Content pack was stored in manifest pack.
Now, manifest pack do not contain other packs but just reference them.
New container pack now contains other packs (including manifest pack).

This allow a better regroupement of pack as two content packs can now regroup together, even without manifest pack.

This also allow better sharing of content pack between container as manifest can link to another container pack in which
we will search for content pack.

### Move to little endian

By moving to little endian, we are closer to most common hardware implementation and we have a lot less bytes swap.

## Implementation

### API

Well... almost everything has changed. Hopefully, no one but me was using this API.
And tools as arx and waj are already ported.
Sorry but I will not explain every structure renaming here :)

### Cache

Structure as ValueStore and EntryStore are now cached.
We don't recreate (and parse) stores at each request/api call.

### Check tool

Add a tool to check packs, using checksum stored in packs.

### Value store now avoid storing duplicated data

If two entries use the same value (and a value store), they now point to the same value instead
of adding twice the value to the value store.

### Do not compress all data

Based on shannon entropy, we decide if we must compress data or not.

### Compress cluster in different threads

This greatly improve creation time.

### Uncompress content in differnt threads

Instead of doing partial decompression in the main thread, we now do full decompression in background
decompression thread.

- Main thread now return as soon as we have started to decompress content, not when the content is decompressed.
- We decompress the whole cluster, so next read has more chance to read already ready content. (yes, a lot of "read" here)
- As we don't store the decompression context, we use less memory.
- We still allow reading of the partially decompressed content while rest of the cluster is decompressed.

### Sort entry and value store in parallel

Rayon help here to sort our stores.

### Use binary search to locate entry

When entries are sorted in the store, we now use binary search to locate them.
We go from `O(n)` to `O(log(n))`

### Jubako structures are now `Sync` and `Send`

You can share your structure between threads.

### Use stable rust

Jubako now use stable rust. No need for nightly rust and unsteable features.


### Countless speed and memory optimization

Countless I've said !!
