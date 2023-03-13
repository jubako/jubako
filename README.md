Jubako library
==============


What is this ?
--------------

Jubako is a new container format, its spec can be found at https://framagit.org/jubako/spec

This repository is the reference implementation (in rust) of Jubako.
This Jubako library is still in development and is not ready to use.


Using Jubako
------------

Jubako library is the low level library to read and write Jubako container.
Jubako format is somehow a metaformat, each user (vendor) of Jubako have to
specify its own format based on Jubako.

So, the classic use case is to create a libray on top of jubako to wrap jubako
structure and provide high level implementation.

You can have a look to [arx](https://framagit.org/jubako/arx) which is file
archive based on Jubako.

