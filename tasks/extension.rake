# frozen_string_literal: true

namespace :extension do
  desc "Build the extension for this host (debug) and install it into the addon"
  task build: "beni:build" do
    sh "cargo", "build", "--manifest-path", Extension::MANIFEST

    source, name = Extension.host_library
    Extension.install(File.join(Extension::TARGET_DIR, "debug", source), name)
  end

  if Extension.macos?
    desc "Build the macOS universal extension (release) and install it into the addon"
    task universal: "beni:build" do
      Extension::ARCHES.each_key do |triple|
        sh Extension.cargo_env(triple),
           "cargo", "build", "--release", "--target", triple, "--manifest-path", Extension::MANIFEST
      end

      output = Extension.universal("release")
      sh "lipo", "-create", "-output", output, *Extension::ARCHES.keys.map { |t| Extension.slice(t, "release") }
      Extension.require_arches!(output)
      Extension.install(output, Extension::MACOS_LIBRARY)
    end
  end
end
