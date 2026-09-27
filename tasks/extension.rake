# frozen_string_literal: true

namespace :extension do
  desc "Build the extension for this host (debug) and install it into the addon"
  task build: "beni:build" do
    sh "cargo", "build", "--manifest-path", Extension::MANIFEST

    source, name = Extension.host_library
    Extension.install(File.join(Extension::TARGET_DIR, "debug", source), name)
  end

  desc "Build the library this platform ships (PROFILE=release|debug) and install it into the addon"
  task dist: "beni:build" do
    profile = Extension.profile
    flags = Extension::PROFILES.fetch(profile)

    if Extension.macos?
      Extension::ARCHES.each_key do |triple|
        sh Extension.cargo_env(triple),
           "cargo", "build", *flags, "--target", triple, "--manifest-path", Extension::MANIFEST
      end

      output = Extension.universal_library(profile)
      sh "lipo", "-create", "-output", output, *Extension::ARCHES.keys.map { |t| Extension.slice(t, profile) }
      Extension.require_arches!(output)
      Extension.install(output, Extension::MACOS_LIBRARY)
    else
      sh "cargo", "build", *flags, "--manifest-path", Extension::MANIFEST

      source, name = Extension.host_library
      Extension.install(File.join(Extension::TARGET_DIR, profile, source), name)
    end
  end
end

namespace :extension do
  desc "Write the changed godot-cell into vendor/"
  task :godot_cell do
    GodotCell.vendor
    GodotCell.check_lock
  end
end

# cargo resolves no crate graph without the changed godot-cell, and every
# build and check reaches cargo through beni:build.
task "beni:build" => "extension:godot_cell"
