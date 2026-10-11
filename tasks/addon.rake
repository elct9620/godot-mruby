# frozen_string_literal: true

require_relative "support/addon"

namespace :addon do
  desc "Package the addon with every platform's library and its licenses into #{Addon::PACKAGE}"
  task package: ["beni:vendor:setup", "extension:vendor_godot_cell"] do
    Addon.package
  end

  desc "Install a package (ZIP=path, default #{Addon::PACKAGE}) into a copy of the test project and verify it loads"
  task :verify do
    Addon.verify!(ENV.fetch("ZIP", Addon::PACKAGE))
  end

  desc "Play the release checks with a package (ZIP=path) installed into a copy of the test project"
  task :release do
    Addon.verify_release!(ENV.fetch("ZIP", Addon::PACKAGE))
  end
end
