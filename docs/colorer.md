<!-- Purpose: How to build FFM with real Colorer syntax highlighting. -->

# 🎨 Colorer

The editor can use **[Colorer-library](https://github.com/colorer/Colorer-library)** —
the same C++ engine Far Manager's FarColorer plugin uses — with Far's own
[HRC schemes](https://github.com/colorer/Colorer-schemes). A file then looks
here as it looks in Far, for all 367 languages rather than the dozen the
built-in tokenizer knows.

It is **on by default**, and costs nothing when the engine is not installed:
`colorer-sys` compiles to stubs, `Colorer::open` returns `None`, and
`src/highlight.rs` does the work. So `cargo run`, `./ffm.sh run` and
`ffm.cmd run` all build the Colorer version without needing a C++ toolchain,
ICU, libxml2 or zlib — those are only needed to build the engine itself.

## 🏗️ Building with it

One command installs the engine and Far's schemes under
`~/.local/share/ffm` (`%USERPROFILE%\.local\share\ffm` on Windows), which is
where both the build script and the running program look:

```sh
./ffm.sh colorer      # Linux, macOS
ffm.cmd colorer       # Windows
```

By hand, or to point at a checkout somewhere else:

```sh
git clone https://github.com/colorer/Colorer-library
cmake -S Colorer-library -B Colorer-library/_build -DCOLORER_USE_VCPKG=OFF
cmake --build Colorer-library/_build          # produces libcolorer.a

git clone https://github.com/colorer/Colorer-schemes

export COLORER_INCLUDE_DIR=$PWD/Colorer-library/src
export COLORER_BUILD_DIR=$PWD/Colorer-library/_build


cargo build --features colorer
```

On Debian/Ubuntu the system side is `libicu-dev libxml2-dev zlib1g-dev libminizip-dev`.
Configure Colorer-library with `-DCOLORER_USE_VCPKG=OFF` so it uses the
system minizip rather than vcpkg's `unofficial-minizip` package.

On Windows `ffm.cmd colorer` leaves vcpkg **on** — there is no system ICU,
libxml2 or zlib for CMake to find — and the build script links the result
without help: it adds `_build\vcpkg_installed\<triplet>\lib` to the linker's
search path and defaults the library names to vcpkg's
(`icuuc,icuin,icudt,libxml2,zlib`). `COLORER_LINK_LIBS` overrides that list
for a toolchain that names them differently — and is where to add `minizip`
if the link reports unresolved `unz*` symbols.

`COLORER_CATALOG` is read at run time. Without it these are tried in order:

```
/usr/share/colorer/catalog.xml
/usr/local/share/colorer/catalog.xml
/opt/colorer/catalog.xml
$HOME/.local/share/ffm/colorer/catalog.xml
$HOME/.local/share/ffm/Colorer-schemes/base/catalog.base.xml
```

`$HOME` falls back to `%USERPROFILE%` where it is unset, which is every
Windows shell — without that the last two never resolved and a Windows build
found no schemes even with the engine linked.

A Far install already has one, under `Plugins\FarColorer\base`.

## 🧩 How it fits together

```
colorer-sys/shim.cpp     a C surface over the C++ classes
colorer-sys/src/lib.rs   the raw `extern "C"` declarations
src/highlight/colorer.rs the safe wrapper, and region -> colour
src/highlight.rs         picks Colorer when present, tokenizer otherwise
```

The shim implements the two interfaces Colorer asks a host to provide:

- **`LineSource`** — hands the parser lines out of the editor's buffer.
- **`RegionHandler`** — receives every region found, as
  `(line, start, end, name)`.

Then `ParserFactory::loadCatalog` reads the scheme database,
`HrcLibrary::chooseFileType` picks the language, and `TextParser::parse` walks
the text.

One step is easy to miss and silently produces no colour at all:

```cpp
FileType* type = hrc.chooseFileType(&name, &firstLine);
hrc.loadFileType(type);   // without this the parser runs and reports nothing
```

`chooseFileType` returns a type whose scheme has not been read yet.

## 🏷️ Regions

Colorer reports fully-qualified names — `def:Keyword`, `python:pyStringEdge`.
The vocabulary is much larger than the editor's palette, but it is systematic:
a language's regions are conventionally the `def:` ones with a prefix. So
`region_token` matches on the **suffix**, which is why a language nobody wrote
a table for still lands on the right colour.

Regions overlap — a bracket is both `def:Symbol` and `def:PairStart` — and the
last one to cover a character wins. That is Colorer's own rule.

## 🧵 Threading

Every call into Colorer is serialised behind one process-wide mutex, and the
scheme database is loaded once into a `OnceLock`.

That is not caution. Colorer reads its schemes through libxml2, and
`LibXmlReader`'s constructor calls `xmlSetExternalEntityLoader` and
`xmlSetGenericErrorFunc` — both *global*. Two threads building parsers at once
race on them and segfault. This showed up as the whole test suite crashing under
the parallel runner while every individual test passed.

## ⚠️ Limits

- The built-in tokenizer stays, and is used whenever Colorer is absent, has no
  scheme for the file, or finds nothing. A missing scheme database costs
  colour, never the file.
- Colorer's own **HRD** style files are not read yet; regions are mapped onto
  the existing palette instead. Reading `.hrd` would give the exact Far colours
  per language rather than an approximation.
- ICU is a substantial runtime dependency — `libicudata` alone is ~30 MB. It is
  a shared library on Linux; shipping it on Windows is a real cost to weigh.

## ⚖️ Licences

Colorer-library and Colorer-schemes are MIT. Redistributing either inside a
binary means carrying their notices.
