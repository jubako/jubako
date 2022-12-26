# Layout and schema

`Layout` (and other) is the description of what is stored in the entrystore.
`Schema` is the description of what is expected for our entries.


## Layout

`Layout` (and other) is the description of what is stored in the entrystore.
It cames in two different flavours.

### RawLayout and RawProperty

RawLayout is the layout as stored in the entrystore. It is a vector of RawProperty.
No coherence check is made, simply what is stored


### Layout, Variant and Property

Layout is what is stored in the entry store but is a more logical way.

Layout contains different Variants and Variant contains a list of Property.

Here coherence is ensure.
Layout knows it size. Variant_id are check (and properties regroup in variant).
Properties know at which offset to search the value. RawProperty can be regrouped in one Property (VLArray with lookup Array are regrouped in one Property)

Layout is build from RawLayout.

Layout is by nature dynamic as it has to addapt to what is read.


## Schema

`Schema` is closed from a `Layout` but it is different in the sense that it describe what is expected. This is not a one to one mapping.

For exemple, `Schema::Integer` may accept a `Property::U8` and a `Property::U16`.

`Schema` is taking a `Store`, get its `Layout` and verify it (is it in accordance with the schema).
Then it produces a `Builder`.

## Builder

`Builder` is in charge to create the entry.
It is build by the `Schema` from the information comming from the `Layout`.
It is "mostly" a set of sub-builder and value-builder. value-builder are create by the layout.
`Builder` acts as proxy on top of `Store`
Builder runs by taking a entry index and produce a entry.

The `Entry`'s structure generated is dependent of the `Schema`, NOT the `Layout`.
The `Entry`'s values (and how they are parsed) is dependent of the `Layout`.


## Finder

`Finder` are high level structure to access entry.
It is specialized from a `Schema` and wrapp a `Schema::Builder`.
