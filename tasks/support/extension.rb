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

  module_function

  def macos?
    RbConfig::CONFIG["host_os"].include?("darwin")
  end

  def host_arch
    case RbConfig::CONFIG["host_cpu"]
    when /x86_64|x64|amd64/ then "x86_64"
    when /arm64|aarch64/ then "arm64"
    else raise "Unsupported host CPU: #{RbConfig::CONFIG["host_cpu"]}"
    end
  end

  # Cargo's output name for this host, and the name the addon expects.
  def host_library
    case RbConfig::CONFIG["host_os"]
    when /darwin/ then ["lib#{CRATE}.dylib", MACOS_LIBRARY]
    when /linux/ then ["lib#{CRATE}.so", "lib#{CRATE}.linux.#{host_arch}.so"]
    when /mswin|mingw/ then ["#{CRATE}.dll", "#{CRATE}.windows.#{host_arch}.dll"]
    else raise "Unsupported host OS: #{RbConfig::CONFIG["host_os"]}"
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

  def universal(profile)
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
