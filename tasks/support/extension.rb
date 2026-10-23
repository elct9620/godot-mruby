# frozen_string_literal: true

require "fileutils"
require "open3"
require "rbconfig"

require_relative "mruby_build"

# Where the extension is built and how a build of it reaches the place Godot
# loads it from: bin/ of the addon, under the name godot_mruby.gdextension
# resolves for each platform. Backs tasks/extension.rake.
module Extension
  ROOT = File.expand_path("../..", __dir__)
  MANIFEST = File.join(ROOT, "rust", "Cargo.toml")
  TARGET_DIR = File.join(ROOT, "rust", "target")
  VENDOR_DIR = File.join(ROOT, "vendor")
  ADDON_BIN = File.join(ROOT, "godot", "addons", "godot_mruby", "bin")
  CRATE = "godot_mruby"

  # Apple Silicon is the macOS build machine. Its compiler targets both
  # architectures, so the x86_64 half is the one cross-built.
  NATIVE_TRIPLE = "aarch64-apple-darwin"
  CROSS_TRIPLE = "x86_64-apple-darwin"
  ARCHES = { NATIVE_TRIPLE => "arm64", CROSS_TRIPLE => "x86_64" }.freeze

  # The cargo flags for each profile; the profile name is also the directory
  # cargo writes it to.
  PROFILES = { "release" => ["--release"], "debug" => [] }.freeze

  # The cargo flags for each library a platform ships: the editor's, with its
  # tools and the test runner, and the two export templates load, without.
  KINDS = { "editor" => [], "template_debug" => ["--no-default-features"],
            "template" => ["--no-default-features"] }.freeze

  module_function

  def macos?
    RbConfig::CONFIG["host_os"].include?("darwin")
  end

  # Debug builds of every library for this host, installed where Godot
  # loads them.
  def build_debug_libraries
    KINDS.each_key { |kind| build_host_library("debug", kind) }
  end

  # The libraries this platform ships, in the profile PROFILE names; macOS
  # ships universal libraries of both architectures.
  def build_shipped_libraries
    KINDS.each_key do |kind|
      macos? ? build_universal_library(profile, kind) : build_host_library(profile, kind)
    end
  end

  # The mruby builds the libraries link, for beni to build: this host's, and
  # on macOS the x86_64 half's.
  def mruby_builds
    MrubyBuild.names(macos? ? ["host", CROSS_TRIPLE] : ["host"])
  end

  # Every kind builds to the same file, so each is installed before the next
  # builds.
  def build_host_library(profile, kind)
    system(build_cargo_env("host", kind),
           "cargo", "build", *PROFILES.fetch(profile), *KINDS.fetch(kind), "--manifest-path", MANIFEST,
           exception: true)

    source, name = host_library(kind)
    install(File.join(TARGET_DIR, profile, source), name)
  end

  def build_universal_library(profile, kind)
    ARCHES.each_key do |triple|
      system(build_cargo_env(triple == NATIVE_TRIPLE ? "host" : triple, kind),
             "cargo", "build", *PROFILES.fetch(profile), *KINDS.fetch(kind), "--target", triple,
             "--manifest-path", MANIFEST, exception: true)
    end

    output = locate_universal_library(profile, kind)
    system("lipo", "-create", "-output", output, *ARCHES.keys.map { |t| locate_slice(t, profile) }, exception: true)
    require_arches!(output)
    install(output, macos_library(kind))
  end

  # A distributed build is release unless PROFILE=debug asks for one to
  # compare against.
  def profile
    ENV.fetch("PROFILE", "release").tap do |name|
      raise "PROFILE must be one of #{PROFILES.keys.join(", ")}, got #{name}" unless PROFILES.key?(name)
    end
  end

  # The triple cargo builds for without --target. It is read from rustc
  # rather than Ruby, which may run emulated on a machine of another
  # architecture.
  def host_triple
    @host_triple ||= begin
      output, status = Open3.capture2("rustc", "-vV", chdir: ROOT)
      raise "rustc could not report its host" unless status.success?

      output[/^host: (\S+)/, 1]
    end
  end

  def host_arch
    case host_triple
    when /\Ax86_64-/ then "x86_64"
    when /\Aaarch64-/ then "arm64"
    else raise "Unsupported host: #{host_triple}"
    end
  end

  # Cargo's output name for this host, and the name the addon expects for
  # the library of `kind`.
  def host_library(kind)
    case host_triple
    when /apple-darwin/ then ["lib#{CRATE}.dylib", macos_library(kind)]
    when /linux/ then ["lib#{CRATE}.so", "lib#{CRATE}.linux.#{host_arch}.#{kind}.so"]
    when /windows/ then ["#{CRATE}.dll", "#{CRATE}.windows.#{host_arch}.#{kind}.dll"]
    else raise "Unsupported host: #{host_triple}"
    end
  end

  def macos_library(kind)
    "lib#{CRATE}.macos.#{kind}.dylib"
  end

  # The native build finds the host build's archive through BENI_VENDOR_DIR,
  # which .cargo/config.toml sets. MRUBY_LIB_DIR outranks it, so it names
  # any other build: a cross target's, or one without the debug hook.
  def build_cargo_env(base, kind)
    build = MrubyBuild.name(base, kind)
    return {} if build == "host"

    { "MRUBY_LIB_DIR" => File.join(VENDOR_DIR, "mruby", "build", build, "lib") }
  end

  def locate_slice(triple, profile)
    File.join(TARGET_DIR, triple, profile, "lib#{CRATE}.dylib")
  end

  def locate_universal_library(profile, kind)
    File.join(TARGET_DIR, profile, macos_library(kind))
  end

  # Replace rather than overwrite: a running editor still maps the old
  # library, and rewriting it in place invalidates its code signature.
  def install(source, name)
    target = File.join(ADDON_BIN, name)
    FileUtils.mkdir_p(ADDON_BIN)
    FileUtils.rm_f(target)
    FileUtils.cp(source, target)
    target
  end

  def require_arches!(path)
    archs, status = Open3.capture2("lipo", "-archs", path)
    raise "lipo could not read #{path}" unless status.success?

    missing = ARCHES.values - archs.split
    raise "#{path} is missing #{missing.join(", ")} (has #{archs.strip})" unless missing.empty?
  end
end
