// A C surface over Colorer-library, wide enough for an editor to colour a file.
#include <cstring>
#include <string>
#include <vector>
#include <memory>

#include "colorer/ParserFactory.h"
#include "colorer/TextParser.h"
#include "colorer/LineSource.h"
#include "colorer/RegionHandler.h"
#include "colorer/HrcLibrary.h"
#include "colorer/FileType.h"

extern "C" {
typedef void (*colorer_region_cb)(void* ctx, size_t line, int sx, int ex, const char* region);
}

namespace {

// Feeds the parser lines out of a plain array the caller owns.
class BufferLines : public LineSource {
 public:
  BufferLines(const char* const* lines, size_t count) : lines_(lines), count_(count) {}
  UnicodeString* getLine(size_t lno) override {
    if (lno >= count_) return nullptr;
    // UTF-8 in, UTF-16 out. An invariant-charset conversion would be wrong
    // for every non-ASCII line, so there is only the one conversion here.
    held_ = UnicodeString::fromUTF8(lines_[lno]);
    return &held_;
  }
 private:
  const char* const* lines_;
  size_t count_;
  UnicodeString held_;
};

// Turns every region the parser reports into one callback.
class Collector : public RegionHandler {
 public:
  Collector(colorer_region_cb cb, void* ctx) : cb_(cb), ctx_(ctx) {}
  void addRegion(size_t lno, UnicodeString*, int sx, int ex, const Region* region) override {
    if (!region) return;
    std::string name;
    region->getName().toUTF8String(name);
    cb_(ctx_, lno, sx, ex, name.c_str());
  }
  void enterScheme(size_t, UnicodeString*, int, int, const Region*, const Scheme*) override {}
  void leaveScheme(size_t, UnicodeString*, int, int, const Region*, const Scheme*) override {}
 private:
  colorer_region_cb cb_;
  void* ctx_;
};

struct Colorer {
  ParserFactory factory;
};

}  // namespace

extern "C" {

void* colorer_open(const char* catalog_path) {
  try {
    auto* c = new Colorer();
    UnicodeString path = UnicodeString::fromUTF8(catalog_path);
    c->factory.loadCatalog(&path);
    return c;
  } catch (...) {
    return nullptr;
  }
}

void colorer_close(void* handle) { delete static_cast<Colorer*>(handle); }

// Returns the number of lines parsed, or -1 on failure.
int colorer_parse(void* handle, const char* filename,
                  const char* const* lines, size_t count,
                  colorer_region_cb cb, void* ctx) {
  if (!handle || !cb) return -1;
  auto* c = static_cast<Colorer*>(handle);
  try {
    UnicodeString name = UnicodeString::fromUTF8(filename);
    UnicodeString first = count ? UnicodeString::fromUTF8(lines[0]) : UnicodeString();
    FileType* type = c->factory.getHrcLibrary().chooseFileType(&name, &first);
    if (!type) return -1;
    // A chosen type is only a name until its scheme is loaded; without this the
    // parser runs and reports nothing at all.
    c->factory.getHrcLibrary().loadFileType(type);

    BufferLines source(lines, count);
    Collector collector(cb, ctx);
    auto parser = c->factory.createTextParser();
    parser->setFileType(type);
    parser->setLineSource(&source);
    parser->setRegionHandler(&collector);
    parser->parse(0, static_cast<int>(count), TextParser::TextParseMode::TPM_CACHE_OFF);
    return static_cast<int>(count);
  } catch (...) {
    return -1;
  }
}

// The file type Colorer would choose, for reporting.
const char* colorer_file_type(void* handle, const char* filename) {
  static thread_local std::string held;
  if (!handle) return nullptr;
  auto* c = static_cast<Colorer*>(handle);
  try {
    UnicodeString name = UnicodeString::fromUTF8(filename);
    UnicodeString empty;
    FileType* type = c->factory.getHrcLibrary().chooseFileType(&name, &empty);
    if (!type) return nullptr;
    held.clear();
    type->getName().toUTF8String(held);
    return held.c_str();
  } catch (...) {
    return nullptr;
  }
}

}  // extern "C"
