# frozen_string_literal: true

namespace :extension do
  desc "Build the editor and template libraries for this host (debug) into the addon"
  task build: "beni:build" do
    Extension.build_debug_libraries
  end

  desc "Build the libraries this platform ships (PROFILE=release|debug) into the addon"
  task dist: "beni:build" do
    Extension.build_shipped_libraries
  end
end

namespace :extension do
  desc "Write the changed godot-cell into vendor/"
  task :vendor_godot_cell do
    GodotCell.vendor
    GodotCell.check_lock
  end
end

# cargo resolves no crate graph without the changed godot-cell, and every
# build and check reaches cargo through beni:build.
task "beni:build" => "extension:vendor_godot_cell"
