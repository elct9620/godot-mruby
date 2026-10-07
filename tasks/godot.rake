# frozen_string_literal: true

require_relative "support/godot"
require_relative "support/release"

namespace :godot do
  desc "Load the addon headless, then check its scenes and the Ruby tests"
  task :verify do
    Godot.verify!
  end

  desc "Play the release checks on an export template's release build"
  task :release do
    Godot::Release.verify!
  end
end

desc "Open the editor on godot/ with a fresh host build"
task editor: "extension:build" do
  Godot.open_editor!
end
