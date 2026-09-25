# frozen_string_literal: true

require "fileutils"
require "open3"
require "rbconfig"

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

  MACOS_LIBRARY = "lib#{CRATE}.macos.dylib".freeze

  # The cargo flags for each profile; the profile name is also the directory
  # cargo writes it to.
  PROFILES = { "release" => ["--release"], "debug" => [] }.freeze

  module_function

  def macos?
    RbConfig::CONFIG["host_os"].include?("darwin")
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

  # Cargo's output name for this host, and the name the addon expects.
  def host_library
    case host_triple
    when /apple-darwin/ then ["lib#{CRATE}.dylib", MACOS_LIBRARY]
    when /linux/ then ["lib#{CRATE}.so", "lib#{CRATE}.linux.#{host_arch}.so"]
    when /windows/ then ["#{CRATE}.dll", "#{CRATE}.windows.#{host_arch}.dll"]
    else raise "Unsupported host: #{host_triple}"
    end
  end

  # The native build finds its archive through BENI_VENDOR_DIR, which
  # .cargo/config.toml sets. A cross target reads only MRUBY_LIB_DIR, and
  # that variable outranks the other, so it is set for the cross build alone.
  def cargo_env(triple)
    return {} if triple == NATIVE_TRIPLE

    { "MRUBY_LIB_DIR" => File.join(VENDOR_DIR, "mruby", "build", triple, "lib") }
  end

  def slice(triple, profile)
    File.join(TARGET_DIR, triple, profile, "lib#{CRATE}.dylib")
  end

  def universal_library(profile)
    File.join(TARGET_DIR, profile, MACOS_LIBRARY)
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
