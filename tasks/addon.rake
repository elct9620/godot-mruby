# frozen_string_literal: true

require_relative "support/addon"

namespace :addon do
  desc "Package the addon with every platform's library and its licenses into #{Addon::PACKAGE}"
  task package: "beni:vendor:setup" do
    missing = Addon.missing_libraries
    abort "The addon is missing libraries:\n#{missing.join("\n")}" unless missing.empty?

    rm_f Addon::PACKAGE
    mkdir_p File.dirname(Addon::PACKAGE)
    Dir.mktmpdir do |dir|
      Addon.stage(dir)
      sh "zip", "-qr", Addon::PACKAGE, Addon::FOLDER, chdir: dir
    end
  end

  desc "Install a package (ZIP=path, default #{Addon::PACKAGE}) into a copy of the test project and verify it loads"
  task :verify do
    Addon.verify!(ENV.fetch("ZIP", Addon::PACKAGE))
  end
end
