# frozen_string_literal: true

require "digest"
require "fileutils"
require "open-uri"
require "rubygems/package"
require "zlib"

require_relative "extension"

# The copy of gdext's godot-cell the extension builds with: the published
# crate with one condition changed, so a mutable bind waits for the shared
# binds other threads hold. Without it, a thread whose shared bind came first
# binds mutably while another thread's is still held, and gdext panics; Godot
# makes such calls on the language from the editor's scan and from any thread
# printing an error. rust/Cargo.toml patches crates.io's godot-cell with this
# copy, which lives under vendor/ beside mruby, so the repository keeps only
# the change.
module GodotCell
  VERSION = "0.5.5"
  URL = "https://static.crates.io/crates/godot-cell/godot-cell-#{VERSION}.crate".freeze
  CHECKSUM = "edc4695597110f27de8f0adbdf3afd01692e3596cc89de2aaa2a42d31b93bfc4"
  DIR = File.join(Extension::VENDOR_DIR, "godot-cell")
  LOCK = File.join(Extension::ROOT, "rust", "Cargo.lock")

  FILE = "src/blocking_cell.rs"
  # In `borrow_mut`: skip the wait only while the calling thread holds the
  # cell's mutable bind, not whenever it was the last to claim it.
  CONDITION = <<~RUST.gsub(/^/, "        ")
    if self.inner.as_ref().is_currently_bound()
        && tracker_guard.current_thread_shared_count() == 0
        && !tracker_guard.current_thread_has_mut_ref()
  RUST
  REPLACEMENT = <<~RUST.gsub(/^/, "        ")
    if self.inner.as_ref().is_currently_bound()
        && tracker_guard.current_thread_shared_count() == 0
        && !(self.inner.as_ref().is_currently_mutably_bound()
            && tracker_guard.current_thread_has_mut_ref())
  RUST

  module_function

  # The change as the addon's third-party notices show it.
  def change
    "This build changes #{FILE}, in `borrow_mut`, from\n\n#{CONDITION}\nto\n\n#{REPLACEMENT}\n" \
      "The changed source is godot-cell #{VERSION} from crates.io with this change, under the same license.\n"
  end

  # Writes the changed copy into vendor/ unless it is already there.
  def vendor
    return if applied?

    FileUtils.rm_rf(DIR)
    unpack(download)
    apply
  end

  # Refuses a lockfile in which cargo builds any godot-cell but this copy, as
  # it does without a word when gdext comes to require another version.
  def check_lock
    entries = File.read(LOCK).split("[[package]]").select { |entry| entry.include?('name = "godot-cell"') }
    if entries.size == 1 && entries.first.include?(%(version = "#{VERSION}")) && !entries.first.include?("source =")
      return
    end

    raise "Cargo.lock builds a godot-cell other than vendor/godot-cell #{VERSION}; " \
          "gdext now requires another version, whose bind needs checking before the change moves to it"
  end

  def applied?
    File.exist?(File.join(DIR, FILE)) && File.binread(File.join(DIR, FILE)).include?(REPLACEMENT)
  end

  def download
    URI.parse(URL).open("rb", &:read).tap do |crate|
      raise "#{URL} does not match its checksum" unless Digest::SHA256.hexdigest(crate) == CHECKSUM
    end
  end

  # A crate is a gzipped tarball whose entries sit under name-version/.
  def unpack(crate)
    prefix = "godot-cell-#{VERSION}/"
    Gem::Package::TarReader.new(Zlib::GzipReader.new(StringIO.new(crate))) do |tar|
      tar.each do |entry|
        next unless entry.file? && entry.full_name.start_with?(prefix)

        path = File.join(DIR, entry.full_name.delete_prefix(prefix))
        FileUtils.mkdir_p(File.dirname(path))
        File.binwrite(path, entry.read)
      end
    end
  end

  def apply
    path = File.join(DIR, FILE)
    source = File.binread(path)
    raise "#{FILE} of godot-cell #{VERSION} no longer reads as expected" unless source.scan(CONDITION).size == 1

    File.binwrite(path, source.sub(CONDITION, REPLACEMENT))
  end
end
